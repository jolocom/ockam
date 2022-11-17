use crate::{PortalInternalMessage, PortalMessage, TcpPortalRecvProcessor};
use core::time::Duration;
use ockam_core::compat::{boxed::Box, net::SocketAddr, sync::Arc};
use ockam_core::{async_trait, AccessControl, Decodable, DenyAll, Mailbox, Mailboxes};
use ockam_core::{Address, Any, Result, Route, Routed, Worker};
use ockam_node::{Context, ProcessorBuilder, WorkerBuilder};
use ockam_transport_core::TransportError;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tracing::{debug, info, trace, warn};

/// Enumerate all `TcpPortalWorker` states
///
/// Possible state transitions are:
///
/// `Outlet`: `SendPong` -> `Initialized`
/// `Inlet`: `SendPing` -> `ReceivePong` -> `Initialized`
#[derive(Clone)]
enum State {
    SendPing { ping_route: Route },
    SendPong { pong_route: Route },
    ReceivePong,
    Initialized,
}

/// Enumerate all portal types
#[derive(Debug, Clone)]
enum TypeName {
    Inlet,
    Outlet,
}

/// A TCP Portal worker
///
/// A TCP Portal worker is responsible for managing the life-cycle of
/// a portal connection and is created by
/// [`TcpInletListenProcessor::process`](crate::TcpInletListenProcessor)
/// after a new connection has been accepted.
pub(crate) struct TcpPortalWorker {
    state: State,
    tx: Option<OwnedWriteHalf>,
    rx: Option<OwnedReadHalf>,
    peer: SocketAddr,
    // router_address: Address, // TODO @ac for AccessControl
    internal_address: Address,
    remote_address: Address,
    receiver_address: Address,
    remote_route: Option<Route>,
    is_disconnecting: bool,
    type_name: TypeName,
}

impl TcpPortalWorker {
    /// Start a new `TcpPortalWorker` of type [`TypeName::Inlet`]
    pub(crate) async fn start_new_inlet(
        ctx: &Context,
        stream: TcpStream,
        peer: SocketAddr,
        // router_address: Address, // for AccessControl
        ping_route: Route,
        access_control: Arc<dyn AccessControl>,
    ) -> Result<Address> {
        Self::start(
            ctx,
            peer,
            // router_address,
            State::SendPing { ping_route },
            Some(stream),
            TypeName::Inlet,
            access_control,
        )
        .await
    }

    /// Start a new `TcpPortalWorker` of type [`TypeName::Outlet`]
    pub(crate) async fn start_new_outlet(
        ctx: &Context,
        peer: SocketAddr,
        // router_address: Address, // for AccessControl
        pong_route: Route,
        access_control: Arc<dyn AccessControl>,
    ) -> Result<Address> {
        Self::start(
            ctx,
            peer,
            // router_address,
            State::SendPong { pong_route },
            None,
            TypeName::Outlet,
            access_control,
        )
        .await
    }

    /// Start a new `TcpPortalWorker`
    async fn start(
        ctx: &Context,
        peer: SocketAddr,
        // router_address: Address,
        state: State,
        stream: Option<TcpStream>,
        type_name: TypeName,
        access_control: Arc<dyn AccessControl>,
    ) -> Result<Address> {
        let internal_address = Address::random_tagged("TcpPortalWorker_internal");
        let remote_address = Address::random_tagged("TcpPortalWorker_remote");
        let receiver_address = Address::random_tagged("TcpPortalRecvProcessor");

        info!(
            "Creating new {:?} at internal: {}, remote: {}",
            type_name, internal_address, remote_address
        );

        let (rx, tx) = match stream {
            Some(s) => {
                let (rx, tx) = s.into_split();
                (Some(rx), Some(tx))
            }
            None => (None, None),
        };

        let worker = Self {
            state,
            tx,
            rx,
            peer,
            // router_address,
            internal_address,
            remote_address: remote_address.clone(),
            remote_route: None,
            receiver_address,
            is_disconnecting: false,
            type_name,
        };

        // TODO: @ac 0#TcpPortalWorker_internal
        // in:  0#TcpPortalWorker_internal  <=  [0#TcpPortalRecvProcessor]
        // out: n/a
        let internal_mailbox = Mailbox::new(
            worker.internal_address.clone(),
            Arc::new(DenyAll),
            /*ockam_core::AllowSourceAddress(
                worker.receiver_address.clone(),
            )),
            */
            Arc::new(DenyAll),
        );

        // TODO: @ac 0#TcpPortalWorker_remote
        // in:  0#TcpPortalWorker_remote_6  <=  [0#TcpRecvProcessor_12]
        // out: 0#TcpPortalWorker_remote_6  =>  [0#TcpRouter_main_addr_0, 0#outlet, 0#TcpPortWorker_remote_n]
        let remote_mailbox = Mailbox::new(
            worker.remote_address.clone(),
            access_control,
            Arc::new(DenyAll),
            // FIXME: uncomment below:
            /*
            // TODO @ac need a way to specify AC for incoming from client API because we
            //          don't know if this is coming in over Transport or SecureChannel or...
            Arc::new(ockam_core::AnyAccessControl::new(
                ockam_core::CredentialAccessControl,
                //ockam_node::access_control::AllowTransport::single(crate::TCP),
                ockam_core::ToDoAccessControl,
            )),
            Arc::new(ockam_core::AnyAccessControl::new(
                AllowDestinationAddress(worker.router_address.clone()),
                ockam_core::ToDoAccessControl, // TODO @ac this needs Allow dynamic addresses
            )),

            */
        );

        // start worker
        WorkerBuilder::with_mailboxes(
            Mailboxes::new(internal_mailbox, vec![remote_mailbox]),
            worker,
        )
        .start(ctx)
        .await?;

        Ok(remote_address)
    }
}

enum DisconnectionReason {
    FailedTx,
    FailedRx,
    Remote,
}

impl TcpPortalWorker {
    fn clone_state(&self) -> State {
        self.state.clone()
    }

    /// Start a `TcpPortalRecvProcessor`
    async fn start_receiver(&mut self, ctx: &Context, onward_route: Route) -> Result<()> {
        if let Some(rx) = self.rx.take() {
            let receiver =
                TcpPortalRecvProcessor::new(rx, self.internal_address.clone(), onward_route);

            // TODO: @ac 0#TcpPortalRecvProcessor
            // in:  n/a
            // out: 0#TcpPortalRecvProcessor_7  =>  [0#TcpPortalWorker_internal_5, 0#TcpRouter_main_addr_0]
            // TODO processors can't receive messages, should they be incoming:DenyAll by default?
            let mailbox = Mailbox::new(
                self.receiver_address.clone(),
                Arc::new(DenyAll),
                Arc::new(DenyAll),
                /*
                Arc::new(ockam_core::DenyAll),
                Arc::new(ockam_core::AnyAccessControl::new(
                    AllowDestinationAddress(self.internal_address.clone()),
                    // TODO @ac incoming message onward route
                    // Is this always going to be TcpRouter_main_addr? Nope!
                    // AllowDestinationAddress(self.router_address.clone()),
                    LocalOriginOnly, // TODO @ac I don't think we can get this tighter given it's defined by msg
                )),
                 */
            );
            ProcessorBuilder::with_mailboxes(Mailboxes::new(mailbox, vec![]), receiver)
                .start(ctx)
                .await?;

            Ok(())
        } else {
            Err(TransportError::PortalInvalidState.into())
        }
    }

    async fn notify_remote_about_disconnection(&mut self, ctx: &Context) -> Result<()> {
        // Notify the other end
        if let Some(remote_route) = self.remote_route.take() {
            ctx.send_from_address(
                remote_route,
                PortalMessage::Disconnect,
                self.remote_address.clone(),
            )
            .await?;

            debug!(
                "Notified the other side from {:?} at: {} about connection drop",
                self.type_name, self.internal_address
            );
        }

        // Avoiding race condition when both inlet and outlet connections
        // are dropped at the same time. In this case we want to wait for the `Disconnect`
        // message from the other side to reach our worker, before we shut it down which
        // leads to errors (destination Worker is already stopped)
        // TODO: Remove when we have better way to handle race condition
        ctx.sleep(Duration::from_secs(1)).await;

        Ok(())
    }

    async fn stop_receiver(&self, ctx: &Context) -> Result<()> {
        // Avoiding race condition when both inlet and outlet connections
        // are dropped at the same time. In this case Processor may stop itself
        // while we had `Disconnect` message from the other side. Let it stop itself,
        // but recheck that by calling `stop_processor` and ignoring the error
        // TODO: Remove when we have better way to handle race condition
        ctx.sleep(Duration::from_secs(1)).await;

        if ctx
            .stop_processor(self.receiver_address.clone())
            .await
            .is_ok()
        {
            debug!(
                "{:?} at: {} stopped receiver due to connection drop",
                self.type_name, self.internal_address
            );
        }

        Ok(())
    }

    /// Start the portal disconnection process
    async fn start_disconnection(
        &mut self,
        ctx: &Context,
        reason: DisconnectionReason,
    ) -> Result<()> {
        self.is_disconnecting = true;

        match reason {
            DisconnectionReason::FailedTx => {
                self.notify_remote_about_disconnection(ctx).await?;
            }
            DisconnectionReason::FailedRx => {
                self.notify_remote_about_disconnection(ctx).await?;
                self.stop_receiver(ctx).await?;
            }
            DisconnectionReason::Remote => {
                self.stop_receiver(ctx).await?;
            }
        }

        ctx.stop_worker(self.internal_address.clone()).await?;

        info!(
            "{:?} at: {} stopped due to connection drop",
            self.type_name, self.internal_address
        );

        Ok(())
    }

    async fn handle_send_ping(&self, ctx: &Context, ping_route: Route) -> Result<State> {
        // Force creation of Outlet on the other side
        ctx.send_from_address(ping_route, PortalMessage::Ping, self.remote_address.clone())
            .await?;

        debug!("Inlet at: {} sent ping", self.internal_address);

        Ok(State::ReceivePong)
    }

    async fn handle_send_pong(&mut self, ctx: &Context, pong_route: Route) -> Result<State> {
        // Respond to Inlet
        ctx.send_from_address(
            pong_route.clone(),
            PortalMessage::Pong,
            self.remote_address.clone(),
        )
        .await?;

        if self.tx.is_none() {
            let stream = TcpStream::connect(self.peer)
                .await
                .map_err(TransportError::from)?;
            let (rx, tx) = stream.into_split();
            self.tx = Some(tx);
            self.rx = Some(rx);

            self.start_receiver(ctx, pong_route.clone()).await?;

            debug!(
                "Outlet at: {} successfully connected",
                self.internal_address
            );
        }

        debug!("Outlet at: {} sent pong", self.internal_address);

        self.remote_route = Some(pong_route);
        Ok(State::Initialized)
    }
}

#[async_trait]
impl Worker for TcpPortalWorker {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let state = self.clone_state();

        match state {
            State::SendPing { ping_route } => {
                self.state = self.handle_send_ping(ctx, ping_route.clone()).await?;
            }
            State::SendPong { pong_route } => {
                self.state = self.handle_send_pong(ctx, pong_route.clone()).await?;
            }
            State::ReceivePong | State::Initialized { .. } => {
                return Err(TransportError::PortalInvalidState.into())
            }
        }

        Ok(())
    }

    // TcpSendWorker will receive messages from the TcpRouter to send
    // across the TcpStream to our friend
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        if self.is_disconnecting {
            return Ok(());
        }

        // Remove our own address from the route so the other end
        // knows what to do with the incoming message
        let mut onward_route = msg.onward_route();
        let recipient = onward_route.step()?;

        let return_route = msg.return_route();

        if onward_route.next().is_ok() {
            return Err(TransportError::UnknownRoute.into());
        }

        let state = self.clone_state();

        match state {
            State::ReceivePong => {
                if recipient == self.internal_address {
                    return Err(TransportError::PortalInvalidState.into());
                }

                let msg = PortalMessage::decode(msg.payload())?;

                if let PortalMessage::Pong = msg {
                } else {
                    return Err(TransportError::Protocol.into());
                }

                self.start_receiver(ctx, return_route.clone()).await?;

                debug!("Inlet at: {} received pong", self.internal_address);

                self.remote_route = Some(return_route);
                self.state = State::Initialized;
            }
            State::Initialized => {
                if recipient == self.internal_address {
                    trace!(
                        "{:?} at: {} received internal tcp packet",
                        self.type_name,
                        self.internal_address
                    );

                    let msg = PortalInternalMessage::decode(msg.payload())?;

                    match msg {
                        PortalInternalMessage::Disconnect => {
                            info!(
                                "Tcp stream was dropped for {:?} at: {}",
                                self.type_name, self.internal_address
                            );
                            self.start_disconnection(ctx, DisconnectionReason::FailedRx)
                                .await?;
                        }
                    }
                } else {
                    trace!(
                        "{:?} at: {} received remote tcp packet",
                        self.type_name,
                        self.internal_address
                    );

                    // Send to Tcp stream
                    let msg = PortalMessage::decode(msg.payload())?;

                    match msg {
                        PortalMessage::Payload(payload) => {
                            if let Some(tx) = &mut self.tx {
                                match tx.write_all(&payload).await {
                                    Ok(()) => {}
                                    Err(err) => {
                                        warn!(
                                            "Failed to send message to peer {} with error: {}",
                                            self.peer, err
                                        );
                                        self.start_disconnection(
                                            ctx,
                                            DisconnectionReason::FailedTx,
                                        )
                                        .await?;
                                    }
                                }
                            } else {
                                return Err(TransportError::PortalInvalidState.into());
                            }
                        }
                        PortalMessage::Disconnect => {
                            self.start_disconnection(ctx, DisconnectionReason::Remote)
                                .await?;
                        }
                        PortalMessage::Ping | PortalMessage::Pong => {
                            return Err(TransportError::Protocol.into());
                        }
                    }
                }
            }
            State::SendPing { .. } | State::SendPong { .. } => {
                return Err(TransportError::PortalInvalidState.into())
            }
        };

        Ok(())
    }
}
