use crate::config::{APP_USER_AGENT, AppConfig, BackendEndpoint};
use crate::error::Error;
use crate::secrets::cookies::SecureCookieStore;
use crate::signaling::Connection;
use anyhow::Context;
use reqwest::StatusCode;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{oneshot, Mutex};
use url::Url;
use vacs_protocol::http::ws::WebSocketToken;

pub struct AppStateInner {
    pub config: AppConfig,
    connection: Option<Connection>,
    pub http_client: reqwest::Client,
    cookie_store: Arc<SecureCookieStore>,
}

pub type AppState = Mutex<AppStateInner>;

impl AppStateInner {
    pub fn new() -> anyhow::Result<Self> {
        let cookie_store = Arc::new(SecureCookieStore::default());
        let config = AppConfig::parse()?;

        Ok(Self {
            config: config.clone(),
            connection: None,
            http_client: reqwest::ClientBuilder::new()
                .user_agent(APP_USER_AGENT)
                .cookie_provider(cookie_store.clone())
                .timeout(Duration::from_millis(config.backend.timeout_ms))
                .build()
                .context("Failed to build HTTP client")?,
            cookie_store,
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

    pub fn get_connection(&self) -> Option<&Connection> {
        self.connection.as_ref()
    }

    pub async fn connect(&mut self, app: &AppHandle) -> Result<(), Error> {
        log::info!("Connecting to signaling server");

        if self.connection.is_some() {
            log::info!("Already connected to signaling server");
            return Ok(());
        }

        log::debug!("Retrieving WebSocket auth token");
        let token = self
            .http_get::<WebSocketToken>(BackendEndpoint::WsToken, None)
            .await?
            .token;

        log::debug!("Establishing signaling connection");
        let mut connection = Connection::new(self.config.backend.ws_url.as_str()).await?;

        log::debug!("Logging in to signaling server");
        let client_list = connection.login(token.as_str()).await?;

        log::debug!(
            "Successfully connected to signaling server, {} clients connected",
            client_list.len()
        );
        app.emit("signaling:connected", "LOVV_CTR").ok(); // TODO: Update display name
        app.emit("signaling:client-list", client_list).ok();

        let (on_disconnect_tx, on_disconnect_rx) = oneshot::channel();
        connection.start(app.clone(), on_disconnect_tx).await;
        
        self.connection = Some(connection);
        
        let app_clone = app.clone();
        tokio::spawn(async move {
            if on_disconnect_rx.await.is_ok() {
                log::debug!("Signaling connection task ended, cleaning up state");
                app_clone.state::<AppState>().lock().await.handle_connection_closed(&app_clone).await;
                log::debug!("Finished cleaning up state after signaling connection task ended");
            }
        });

        Ok(())
    }

    pub async fn disconnect(&mut self, app: &AppHandle) {
        log::info!("Disconnecting from signaling server");
        
        let connection = self.connection.take();
        if let Some(mut connection) = connection {
            connection.stop().await;
            app.emit("signaling:disconnected", Value::Null).ok();
            log::debug!("Successfully disconnected from signaling server");
        } else {
            log::info!("Tried to disconnection from signaling server, but not connected");
        }
    }
    
    pub async fn handle_connection_closed(&mut self, app: &AppHandle) {
        log::info!("Handling closed signaling server connection");
        
        if self.connection.take().is_some() {
            app.emit("signaling:disconnected", Value::Null).ok();
            log::debug!("Successfully handled closed signaling server connection");
        } else {
            log::info!("Not connected to signaling server, nothing to handle");
        }
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
