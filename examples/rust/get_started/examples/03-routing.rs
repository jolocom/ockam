// This node routes a message.

use hello_ockam::{Echoer, Hop};
use ockam::{route, Context, Result};

#[ockam::node(
    incoming = "ockam::access_control::LocalOriginOnly",
    outgoing = "ockam::access_control::LocalDestinationOnly"
)]
async fn main(mut ctx: Context) -> Result<()> {
    // Start a worker, of type Echoer, at address "echoer"
    ctx.start_worker("echoer", Echoer).await?;

    // Start a worker, of type Hop, at address "h1"
    ctx.start_worker("h1", Hop).await?;

    // Send a message to the worker at address "echoer",
    // via the worker at address "h1"
    ctx.send(route!["h1", "echoer"], "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
