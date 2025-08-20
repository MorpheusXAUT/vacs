use crate::audio::manager::AudioManager;
use crate::config::{AppConfig, BackendEndpoint, APP_USER_AGENT};
use crate::error::{Error, FrontendError};
use crate::secrets::cookies::SecureCookieStore;
use crate::signaling::Connection;
use anyhow::Context;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{oneshot, Mutex};
use url::Url;
use vacs_protocol::http::ws::WebSocketToken;
use vacs_protocol::ws::SignalingMessage;

pub struct AppStateInner {
    pub config: AppConfig,
    connection: Connection,
    pub audio_manager: AudioManager,
    pub http_client: reqwest::Client,
    cookie_store: Arc<SecureCookieStore>,
    active_call_peer_id: Option<String>,
}

pub type AppState = Mutex<AppStateInner>;

impl AppStateInner {
    pub fn new() -> anyhow::Result<Self> {
        let cookie_store = Arc::new(SecureCookieStore::default());
        let config = AppConfig::parse()?;

        Ok(Self {
            config: config.clone(),
            connection: Connection::new(),
            audio_manager: AudioManager::new(&config.audio)?,
            http_client: reqwest::ClientBuilder::new()
                .user_agent(APP_USER_AGENT)
                .cookie_provider(cookie_store.clone())
                .timeout(Duration::from_millis(config.backend.timeout_ms))
                .build()
                .context("Failed to build HTTP client")?,
            cookie_store,
            active_call_peer_id: None,
        })
    }

    pub fn persist(&self) -> anyhow::Result<()> {
        self.cookie_store
            .save()
            .context("Failed to save cookie store")?;

        Ok(())
    }

    pub fn clear_cookie_store(&self) -> anyhow::Result<()> {
        self.cookie_store
            .clear()
            .context("Failed to clear cookie store")
    }

    pub async fn connect_signaling(&mut self, app: &AppHandle) -> Result<(), Error> {
        log::info!("Connecting to signaling server");

        if self.connection.is_logged_in() {
            log::info!("Already connected and logged in with signaling server");
            return Ok(());
        }

        log::debug!("Retrieving WebSocket auth token");
        let token = self
            .http_get::<WebSocketToken>(BackendEndpoint::WsToken, None)
            .await?
            .token;

        log::debug!("Connecting to signaling server");
        let (disconnect_tx, disconnect_rx) = oneshot::channel();
        self.connection
            .connect(
                app.clone(),
                self.config.backend.ws_url.as_str(),
                token.as_str(),
                disconnect_tx,
            )
            .await?;

        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            let requested = disconnect_rx.await.unwrap_or(false);

            log::debug!("Signaling connection task ended, cleaning up state");
            app_clone
                .state::<AppState>()
                .lock()
                .await
                .handle_signaling_connection_closed(&app_clone, requested)
                .await;
            log::debug!("Finished cleaning up state after signaling connection task ended");
        });

        log::info!("Successfully connected to signaling server");
        Ok(())
    }

    pub async fn disconnect_signaling(&mut self, app: &AppHandle) {
        log::info!("Disconnecting from signaling server");

        if !self.connection.is_connected() {
            log::info!("Tried to disconnection from signaling server, but not connected");
            return;
        }

        self.connection.disconnect();
        app.emit("signaling:disconnected", Value::Null).ok();
        log::debug!("Successfully disconnected from signaling server");
    }

    pub async fn send_signaling_message(&mut self, msg: SignalingMessage) -> Result<(), Error> {
        log::trace!("Sending signaling message: {msg:?}");

        if !self.connection.is_logged_in() {
            log::warn!("Not logged in with signaling server, cannot send message");
            return Err(Error::Network("Not connected".to_string()));
        };

        if let Err(err) = self.connection.send(msg).await {
            log::warn!("Failed to send signaling message: {err:?}");
            return Err(err.into());
        }

        log::trace!("Successfully sent signaling message");
        Ok(())
    }

    async fn handle_signaling_connection_closed(&mut self, app: &AppHandle, requested: bool) {
        log::info!("Handling closed signaling server connection, requested: {requested}");

        app.emit("signaling:disconnected", Value::Null).ok();
        if !requested {
            app.emit::<FrontendError>(
                "error",
                Error::Network("Disconnected from websocket connection".to_string()).into(),
            )
            .ok();
        }
        log::debug!("Successfully handled closed signaling server connection");
    }

    pub fn set_active_call_peer_id(&mut self, peer_id: Option<String>) {
        self.active_call_peer_id = peer_id;
    }

    pub fn active_call_peer_id(&self) -> Option<&String> {
        self.active_call_peer_id.as_ref()
    }

    fn parse_http_request_url(
        &self,
        endpoint: BackendEndpoint,
        query: Option<&[(&str, &str)]>,
    ) -> anyhow::Result<Url> {
        if let Some(query) = query {
            Url::parse_with_params(&self.config.backend.endpoint_url(endpoint), query)
                .context("Failed to parse HTTP request URL with params")
        } else {
            Url::parse(&self.config.backend.endpoint_url(endpoint))
                .context("Failed to parse HTTP request URL")
        }
    }

    pub async fn http_get<R>(
        &self,
        endpoint: BackendEndpoint,
        query: Option<&[(&str, &str)]>,
    ) -> Result<R, Error>
    where
        R: DeserializeOwned + Default,
    {
        let request_url = self.parse_http_request_url(endpoint, query)?;

        log::trace!("Performing HTTP GET request: {}", request_url.as_str());
        let response = self
            .http_client
            .get(request_url.clone())
            .send()
            .await
            .map_err(map_reqwest_error)?
            .error_for_status()
            .map_err(map_reqwest_status_code)?;

        let result = if response.status() == StatusCode::NO_CONTENT {
            R::default()
        } else {
            response
                .json::<R>()
                .await
                .context("Failed to parse HTTP GET response")?
        };

        log::trace!("HTTP GET request succeeded: {}", request_url.as_str());
        Ok(result)
    }

    pub async fn http_post<R, P>(
        &self,
        endpoint: BackendEndpoint,
        query: Option<&[(&str, &str)]>,
        payload: Option<P>,
    ) -> Result<R, Error>
    where
        R: DeserializeOwned + Default,
        P: Serialize,
    {
        let request_url = self.parse_http_request_url(endpoint, query)?;

        log::trace!("Performing HTTP POST request: {}", request_url.as_str());
        let request = self.http_client.post(request_url.clone());
        let request = if let Some(payload) = payload {
            request.json(&payload)
        } else {
            request
        };
        let response = request
            .send()
            .await
            .map_err(map_reqwest_error)?
            .error_for_status()
            .map_err(map_reqwest_status_code)?;

        let result = if response.status() == StatusCode::NO_CONTENT {
            R::default()
        } else {
            response
                .json::<R>()
                .await
                .context("Failed to parse HTTP POST response")?
        };

        log::trace!("HTTP POST request succeeded: {}", request_url.as_str());
        Ok(result)
    }

    pub async fn http_delete<R>(
        &self,
        endpoint: BackendEndpoint,
        query: Option<&[(&str, &str)]>,
    ) -> Result<R, Error>
    where
        R: DeserializeOwned + Default,
    {
        let request_url = self.parse_http_request_url(endpoint, query)?;

        log::trace!("Performing HTTP DELETE request: {}", request_url.as_str());
        let response = self
            .http_client
            .delete(request_url.clone())
            .send()
            .await
            .map_err(map_reqwest_error)?
            .error_for_status()
            .map_err(map_reqwest_status_code)?;

        let result = if response.status() == StatusCode::NO_CONTENT {
            R::default()
        } else {
            response
                .json::<R>()
                .await
                .context("Failed to parse HTTP DELETE response")?
        };

        log::trace!("HTTP DELETE request succeeded: {}", request_url.as_str());
        Ok(result)
    }
}

fn map_reqwest_error(err: reqwest::Error) -> Error {
    if err.is_timeout() || err.is_connect() {
        return Error::Network(err.to_string());
    }
    Error::Reqwest(Box::from(err))
}

fn map_reqwest_status_code(err: reqwest::Error) -> Error {
    if let Some(status) = err.status() {
        log::trace!(
            "HTTP request received non-OK HTTP status: {}",
            status.as_u16()
        );
        match status {
            StatusCode::UNAUTHORIZED => Error::Unauthorized,
            _ => Error::Reqwest(Box::from(err)),
        }
    } else {
        Error::Reqwest(Box::from(err))
    }
}
