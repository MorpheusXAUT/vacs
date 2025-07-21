use crate::config::{AppConfig, BackendEndpoint, APP_USER_AGENT};
use crate::secrets::cookies::SecureCookieStore;
use anyhow::Context;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;
use url::Url;

pub struct AppState {
    pub config: AppConfig,
    pub http_client: reqwest::Client,
    cookie_store: Arc<SecureCookieStore>,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let cookie_store = Arc::new(SecureCookieStore::default());

        Ok(Self {
            config: AppConfig::parse()?,
            http_client: reqwest::ClientBuilder::new()
                .user_agent(APP_USER_AGENT)
                .cookie_provider(cookie_store.clone())
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
    ) -> anyhow::Result<R>
    where
        R: DeserializeOwned,
    {
        let request_url = self.parse_http_request_url(endpoint, query)?;

        log::trace!("Performing HTTP GET request: {}", request_url.as_str());
        let response = self
            .http_client
            .get(request_url.clone())
            .send()
            .await
            .context("Failed to send HTTP GET request")?
            .error_for_status()
            .context("Received non-200 HTTP status for GET request")?
            .json::<R>()
            .await
            .context("Failed to parse HTTP GET response")?;
        log::trace!("HTTP GET request succeeded: {}", request_url.as_str());

        Ok(response)
    }

    pub async fn http_post<R, P>(
        &self,
        endpoint: BackendEndpoint,
        query: Option<&[(&str, &str)]>,
        payload: Option<P>,
    ) -> anyhow::Result<R>
    where
        R: DeserializeOwned,
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
            .context("Failed to send HTTP POST request")?
            .error_for_status()
            .context("Received non-200 HTTP status for POST request")?
            .json::<R>()
            .await
            .context("Failed to parse HTTP POST response")?;
        log::trace!("HTTP POST request succeeded: {}", request_url.as_str());

        Ok(response)
    }
}
