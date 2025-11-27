use crate::ice::IceError;
use reqwest::{Client, header};
use serde::Deserialize;
use std::fmt::{Debug, Formatter};
use std::time::Duration;
use tracing::instrument;
use vacs_protocol::http::webrtc::{IceConfig, IceServer};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudflareIceConfig {
    pub ice_servers: Vec<IceServer>,
}

impl From<CloudflareIceConfig> for IceConfig {
    fn from(value: CloudflareIceConfig) -> Self {
        Self {
            ice_servers: value.ice_servers,
        }
    }
}

#[derive(Clone)]
pub struct CloudflareIceProvider {
    client: Client,
    url: String,
    api_token: String,
    ttl: u64,
}

impl Debug for CloudflareIceProvider {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CloudflareIceProvider")
            .field("ttl", &self.ttl)
            .finish_non_exhaustive()
    }
}

impl CloudflareIceProvider {
    const CLOUDFLARE_TURN_HTTP_TIMEOUT: Duration = Duration::from_secs(1);
    const CLOUDFLARE_TURN_API_URL: &'static str = "https://rtc.live.cloudflare.com/v1/turn/keys/{TURN_KEY_ID}/credentials/generate-ice-servers";

    pub fn new(
        turn_key_id: impl AsRef<str>,
        turn_key_api_token: impl Into<String>,
        ttl: u64,
    ) -> Result<Self, IceError> {
        Ok(Self {
            client: reqwest::ClientBuilder::new()
                .user_agent(crate::APP_USER_AGENT)
                .timeout(Self::CLOUDFLARE_TURN_HTTP_TIMEOUT)
                .build()
                .map_err(|e| IceError::Provider(format!("Failed to create HTTP client: {e}")))?,
            url: Self::CLOUDFLARE_TURN_API_URL.replace("{TURN_KEY_ID}", turn_key_id.as_ref()),
            api_token: turn_key_api_token.into(),
            ttl,
        })
    }
}

#[async_trait::async_trait]
impl crate::ice::provider::IceConfigProvider for CloudflareIceProvider {
    #[instrument(level = "debug", err)]
    async fn get_ice_config(&self, user_id: &str) -> Result<IceConfig, IceError> {
        tracing::debug!("Providing Cloudflare ICE config");

        tracing::trace!("Generating TURN credentials");
        let res = self
            .client
            .post(&self.url)
            .bearer_auth(&self.api_token)
            .header(header::CONTENT_TYPE, "application/json")
            .json(&serde_json::json!({ "ttl": self.ttl, "customIdentifier": user_id}))
            .send()
            .await;

        let response = match res {
            Ok(response) => response,
            Err(err) => {
                tracing::warn!(?err, "Failed to generate TURN credentials");
                return Err(IceError::Provider(format!(
                    "Failed to generate TURN credentials: {err}"
                )));
            }
        };

        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if !status.is_success() {
            tracing::warn!(
                ?status,
                ?body,
                "Received error while generating TURN credentials"
            );
            return Err(IceError::Provider(format!(
                "Received error while generating TURN credentials: {status}"
            )));
        }

        match serde_json::from_str::<CloudflareIceConfig>(&body) {
            Ok(ice_config) => {
                tracing::trace!("Successfully generated TURN credentials");
                Ok(ice_config.into())
            }
            Err(err) => {
                tracing::warn!(?err, ?body, "Failed to parse TURN credentials response");
                Err(IceError::Provider(format!(
                    "Failed to parse TURN credentials response: {err}"
                )))
            }
        }
    }
}
