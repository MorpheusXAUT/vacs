use crate::app::state::http::HttpState;
use crate::config::BackendEndpoint;
use async_trait::async_trait;
use tauri::{AppHandle, Manager};
use vacs_signaling::auth::TokenProvider;
use vacs_signaling::error::SignalingError;
use vacs_signaling::protocol::http::ws::WebSocketToken;

#[derive(Debug, Clone)]
pub struct TauriTokenProvider {
    handle: AppHandle,
}

impl TauriTokenProvider {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl TokenProvider for TauriTokenProvider {
    async fn get_token(&self) -> Result<String, SignalingError> {
        log::debug!("Retrieving WebSocket auth token");
        let http_state = self.handle.state::<HttpState>();

        let token = http_state
            .http_get::<WebSocketToken>(BackendEndpoint::WsToken, None)
            .await
            .map_err(|err| SignalingError::ProtocolError(err.to_string()))?
            .token;

        log::debug!("Successfully retrieved WebSocket auth token");
        Ok(token)
    }
}
