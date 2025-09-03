use crate::app::state::{AppStateInner, sealed};
use crate::config::BackendEndpoint;
use crate::error::Error;
use anyhow::Context;
use reqwest::StatusCode;
use serde::Serialize;
use serde::de::DeserializeOwned;
use url::Url;

pub trait AppStateHttpExt: sealed::Sealed {
    fn clear_cookie_store(&self) -> anyhow::Result<()>;
    async fn http_get<R>(
        &self,
        endpoint: BackendEndpoint,
        query: Option<&[(&str, &str)]>,
    ) -> Result<R, Error>
    where
        R: DeserializeOwned + Default;
    async fn http_post<R, P>(
        &self,
        endpoint: BackendEndpoint,
        query: Option<&[(&str, &str)]>,
        payload: Option<P>,
    ) -> Result<R, Error>
    where
        R: DeserializeOwned + Default,
        P: Serialize;
    async fn http_delete<R>(
        &self,
        endpoint: BackendEndpoint,
        query: Option<&[(&str, &str)]>,
    ) -> Result<R, Error>
    where
        R: DeserializeOwned + Default;
}

impl AppStateHttpExt for AppStateInner {
    fn clear_cookie_store(&self) -> anyhow::Result<()> {
        self.cookie_store
            .clear()
            .context("Failed to clear cookie store")
    }

    async fn http_get<R>(
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

    async fn http_post<R, P>(
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

    async fn http_delete<R>(
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

impl AppStateInner {
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
