use crate::http::error::AppError;
use crate::APP_USER_AGENT;
use anyhow::Context;
use axum_login::{AuthUser, AuthnBackend, UserId};
use oauth2::basic::BasicClient;
use oauth2::reqwest::Url;
use oauth2::{reqwest, AuthorizationCode, CsrfToken, EndpointNotSet, EndpointSet, TokenResponse};
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub cid: String,
}

impl AuthUser for User {
    type Id = String;

    fn id(&self) -> Self::Id {
        self.cid.clone()
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.cid.as_bytes()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub code: String,
    pub stored_state: String,
    pub received_state: String,
}

pub type VatsimOAuthClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

#[derive(Debug, Clone)]
pub struct Backend {
    client: VatsimOAuthClient,
    http_client: reqwest::Client,
    vatsim_user_details_endpoint_url: String,
}

impl Backend {
    pub fn new(
        client: VatsimOAuthClient,
        vatsim_user_details_endpoint_url: String,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            client,
            http_client: reqwest::ClientBuilder::new()
                .user_agent(APP_USER_AGENT)
                .build()
                .context("Failed to build HTTP client")?,
            vatsim_user_details_endpoint_url,
        })
    }

    pub fn authorize_url(&self) -> (Url, CsrfToken) {
        self.client.authorize_url(CsrfToken::new_random).url()
    }
}

impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = AppError;

    #[instrument(level = "debug", skip_all, err)]
    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        tracing::debug!("Authenticating user");
        if creds.stored_state != creds.received_state {
            tracing::debug!("CSRF token mismatch");
            return Ok(None);
        }

        tracing::trace!("Exchanging code for VATSIM access token");
        let token = self
            .client
            .exchange_code(AuthorizationCode::new(creds.code))
            .request_async(&self.http_client)
            .await
            .context("Failed to exchange code")
            .map_err(|err| {
                tracing::warn!(?err, "Failed to exchange code for VATSIM access token");
                AppError::Unauthorized("Invalid code".to_string())
            })?;

        tracing::trace!("Fetching user details");
        let response = self
            .http_client
            .get(self.vatsim_user_details_endpoint_url.clone())
            .bearer_auth(token.access_token().secret())
            .send()
            .await
            .context("Failed to get user details")?
            .error_for_status()
            .context("Received non-200 HTTP status code")?;

        tracing::trace!(content_length = ?response.content_length(), "Parsing response body");
        let user_details = response
            .json::<ConnectUserDetails>()
            .await
            .context("Failed to parse response body")?;

        let user = User {
            cid: user_details.data.cid,
        };

        tracing::debug!(?user, "User authenticated");
        Ok(Some(user))
    }

    #[instrument(level = "trace", skip(self), err)]
    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        tracing::trace!(?user_id, "Getting user");
        Ok(Some(User {
            cid: user_id.to_string(),
        }))
    }
}

pub type AuthSession = axum_login::AuthSession<Backend>;

#[derive(Deserialize, Debug, Clone)]
struct ConnectUserDetails {
    data: ConnectUserDetailsData,
}

#[derive(Deserialize, Debug, Clone)]
struct ConnectUserDetailsData {
    cid: String,
}

pub mod mock {
    use super::*;
    use dashmap::DashMap;

    #[derive(Debug, Clone)]
    pub struct MockBackend {
        access_tokens: DashMap<String, String>,
        user_details: DashMap<String, ConnectUserDetails>,
    }

    impl Default for MockBackend {
        fn default() -> Self {
            let access_tokens = DashMap::new();
            let user_details = DashMap::new();

            for i in 0..=5 {
                access_tokens.insert(format!("code{i}"), format!("token{i}"));
                user_details.insert(
                    format!("token{i}"),
                    ConnectUserDetails {
                        data: ConnectUserDetailsData {
                            cid: format!("cid{i}"),
                        },
                    },
                );
            }

            Self {
                access_tokens,
                user_details,
            }
        }
    }
    
    impl AuthnBackend for MockBackend {
        type User = User;
        type Credentials = Credentials;
        type Error = AppError;

        async fn authenticate(
            &self,
            creds: Self::Credentials,
        ) -> Result<Option<Self::User>, Self::Error> {
            if creds.stored_state != creds.received_state {
                return Ok(None);
            }

            let Some(access_token) = self.access_tokens.get(&creds.code).map(|t| t.clone()) else {
                return Err(AppError::Unauthorized("Invalid code".to_string()));
            };

            let Some(user_details) = self.user_details.get(&access_token).map(|d| d.clone()) else {
                return Err(AppError::Unauthorized("Invalid access token".to_string()));
            };

            Ok(Some(User {
                cid: user_details.data.cid,
            }))
        }

        async fn get_user(
            &self,
            user_id: &UserId<Self>,
        ) -> Result<Option<Self::User>, Self::Error> {
            Ok(Some(User {
                cid: user_id.to_string(),
            }))
        }
    }
}
