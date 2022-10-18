use std::borrow::Cow;

use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;
use ockam_core::{self, async_trait};

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
#[cfg_attr(test, derive(PartialEq, Eq, Clone))]
#[cbor(transparent)]
#[serde(transparent)]
pub struct Token<'a>(#[n(0)] pub Cow<'a, str>);

impl<'a> Token<'a> {
    pub fn new(token: impl Into<Cow<'a, str>>) -> Self {
        Self(token.into())
    }
}

mod node {
    use minicbor::Decoder;
    use tracing::trace;

    use ockam_core::api::Request;
    use ockam_core::{self, Result};
    use ockam_node::Context;

    use crate::cloud::enroll::auth0::AuthenticateAuth0Token;
    use crate::cloud::enroll::enrollment_token::{EnrollmentToken, RequestEnrollmentToken};
    use crate::cloud::CloudRequestWrapper;
    use crate::nodes::NodeManagerWorker;
    use ockam_identity::credential::Attributes;

    use super::*;

    const TARGET: &str = "ockam_api::cloud::enroll";

    impl NodeManagerWorker {
        /// Executes an enrollment process to generate a new set of access tokens using the auth0 flow.
        pub(crate) async fn enroll_auth0(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<AuthenticateAuth0Token> = dec.decode()?;
            let cloud_route = req_wrapper.route()?;
            let req_body: AuthenticateAuth0Token = req_wrapper.req;
            let req_builder = Request::post("v0/enroll").body(req_body);
            let api_service = "auth0_authenticator";

            trace!(target: TARGET, "executing auth0 flow");
            self.request_controller(
                ctx,
                api_service,
                None,
                cloud_route,
                api_service,
                req_builder,
            )
            .await
        }

        /// Executes an enrollment process to generate a new set of access tokens using the okta flow.
        pub(crate) async fn enroll_okta(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<AuthenticateAuth0Token> = dec.decode()?;
            let cloud_route = req_wrapper.route()?;
            let req_body: AuthenticateAuth0Token = req_wrapper.req;
            let req_builder = Request::post("v0/enroll").body(req_body);
            let api_service = "okta_authenticator";

            trace!(target: TARGET, "executing okta flow");
            self.request_controller(
                ctx,
                api_service,
                None,
                cloud_route,
                api_service,
                req_builder,
            )
            .await
        }

        /// Generates a token that will be associated to the passed attributes.
        pub(crate) async fn generate_enrollment_token(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<Attributes> = dec.decode()?;
            let cloud_route = req_wrapper.route()?;
            let req_body: Attributes = req_wrapper.req;
            let req_body = RequestEnrollmentToken::new(req_body);

            let label = "enrollment_token_generator";
            trace!(target: TARGET, "generating tokens");

            let req_builder = Request::post("v0/").body(req_body);
            self.request_controller(
                ctx,
                label,
                "request_enrollment_token",
                cloud_route,
                "projects",
                req_builder,
            )
            .await
        }

        /// Authenticates a token generated by `generate_enrollment_token`.
        pub(crate) async fn authenticate_enrollment_token(
            &mut self,
            ctx: &mut Context,
            dec: &mut Decoder<'_>,
        ) -> Result<Vec<u8>> {
            let req_wrapper: CloudRequestWrapper<EnrollmentToken> = dec.decode()?;
            let cloud_route = req_wrapper.route()?;
            let req_body: EnrollmentToken = req_wrapper.req;
            let req_builder = Request::post("v0/enroll").body(req_body);
            let api_service = "enrollment_token_authenticator";

            trace!(target: TARGET, "authenticating token");
            self.request_controller(
                ctx,
                api_service,
                None,
                cloud_route,
                api_service,
                req_builder,
            )
            .await
        }
    }
}

pub mod auth0 {
    use super::*;

    #[async_trait::async_trait]
    pub trait Auth0TokenProvider: Send + Sync + 'static {
        async fn token(&self) -> ockam_core::Result<Auth0Token<'_>>;
    }

    // Req/Res types

    #[derive(serde::Deserialize, Debug, PartialEq, Eq)]
    pub struct DeviceCode<'a> {
        pub device_code: Cow<'a, str>,
        pub user_code: Cow<'a, str>,
        pub verification_uri: Cow<'a, str>,
        pub verification_uri_complete: Cow<'a, str>,
        pub expires_in: usize,
        pub interval: usize,
    }

    #[derive(serde::Deserialize, Debug, PartialEq, Eq)]
    pub struct TokensError<'a> {
        pub error: Cow<'a, str>,
        pub error_description: Cow<'a, str>,
    }

    #[derive(serde::Deserialize, Debug)]
    #[cfg_attr(test, derive(PartialEq, Eq, Clone))]
    pub struct Auth0Token<'a> {
        pub token_type: TokenType,
        pub access_token: Token<'a>,
    }

    #[derive(Encode, Decode, Debug)]
    #[cfg_attr(test, derive(Clone))]
    #[rustfmt::skip]
    #[cbor(map)]
    pub struct AuthenticateAuth0Token<'a> {
        #[cfg(feature = "tag")]
        #[n(0)] pub tag: TypeTag<1058055>,
        #[n(1)] pub token_type: TokenType,
        #[n(2)] pub access_token: Token<'a>,
    }

    impl<'a> AuthenticateAuth0Token<'a> {
        pub fn new(token: Auth0Token<'a>) -> Self {
            Self {
                #[cfg(feature = "tag")]
                tag: TypeTag,
                token_type: token.token_type,
                access_token: token.access_token,
            }
        }
    }

    // Auxiliary types

    #[derive(serde::Deserialize, Encode, Decode, Debug)]
    #[cfg_attr(test, derive(PartialEq, Eq, Clone))]
    #[rustfmt::skip]
    #[cbor(index_only)]
    pub enum TokenType {
        #[n(0)] Bearer,
    }
}

pub mod enrollment_token {
    use ockam_identity::credential::Attributes;
    use serde::Serialize;

    use super::*;

    // Main req/res types

    #[derive(Encode, Debug)]
    #[cfg_attr(test, derive(Decode, Clone))]
    #[rustfmt::skip]
    #[cbor(map)]
    pub struct RequestEnrollmentToken<'a> {
        #[cfg(feature = "tag")]
        #[n(0)] pub tag: TypeTag<8560526>,
        #[b(1)] pub attributes: Attributes<'a>,
    }

    impl<'a> RequestEnrollmentToken<'a> {
        pub fn new(attributes: Attributes<'a>) -> Self {
            Self {
                #[cfg(feature = "tag")]
                tag: TypeTag,
                attributes,
            }
        }
    }

    #[derive(Encode, Decode, Serialize, Debug)]
    #[cfg_attr(test, derive(Clone))]
    #[rustfmt::skip]
    #[cbor(map)]
    pub struct EnrollmentToken<'a> {
        #[cfg(feature = "tag")]
        #[serde(skip)]
        #[n(0)] pub tag: TypeTag<8932763>,
        #[n(1)] pub token: Token<'a>,
    }

    impl<'a> EnrollmentToken<'a> {
        pub fn new(token: Token<'a>) -> Self {
            Self {
                #[cfg(feature = "tag")]
                tag: TypeTag,
                token,
            }
        }
    }

    #[derive(Encode, Debug)]
    #[cfg_attr(test, derive(Decode, Clone))]
    #[rustfmt::skip]
    #[cbor(map)]
    pub struct AuthenticateEnrollmentToken<'a> {
        #[cfg(feature = "tag")]
        #[n(0)] pub tag: TypeTag<9463780>,
        #[n(1)] pub token: Token<'a>,
    }

    impl<'a> AuthenticateEnrollmentToken<'a> {
        pub fn new(token: EnrollmentToken<'a>) -> Self {
            Self {
                #[cfg(feature = "tag")]
                tag: TypeTag,
                token: token.token,
            }
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
pub(crate) mod tests {
    use quickcheck::{Arbitrary, Gen};

    use crate::cloud::enroll::enrollment_token::{AuthenticateEnrollmentToken, EnrollmentToken};
    use crate::cloud::enroll::Token;

    use super::*;

    pub(crate) mod auth0 {
        use crate::cloud::enroll::auth0::*;

        use super::*;

        pub struct MockAuth0Service;

        #[async_trait::async_trait]
        impl Auth0TokenProvider for MockAuth0Service {
            async fn token(&self) -> ockam_core::Result<Auth0Token<'_>> {
                Ok(Auth0Token {
                    token_type: TokenType::Bearer,
                    access_token: Token::new("access_token"),
                })
            }
        }

        #[derive(Debug, Clone)]
        struct RandomAuthorizedAuth0Token(AuthenticateAuth0Token<'static>);

        impl Arbitrary for RandomAuthorizedAuth0Token {
            fn arbitrary(g: &mut Gen) -> Self {
                RandomAuthorizedAuth0Token(AuthenticateAuth0Token::new(Auth0Token {
                    token_type: TokenType::Bearer,
                    access_token: Token::arbitrary(g),
                }))
            }
        }
    }

    mod enrollment_token {
        use super::*;

        #[derive(Debug, Clone)]
        struct RandomAuthorizedEnrollmentToken(AuthenticateEnrollmentToken<'static>);

        impl Arbitrary for RandomAuthorizedEnrollmentToken {
            fn arbitrary(g: &mut Gen) -> Self {
                RandomAuthorizedEnrollmentToken(AuthenticateEnrollmentToken::new(
                    EnrollmentToken::new(Token::arbitrary(g)),
                ))
            }
        }
    }

    impl Arbitrary for Token<'static> {
        fn arbitrary(g: &mut Gen) -> Self {
            Token(String::arbitrary(g).into())
        }
    }
}
