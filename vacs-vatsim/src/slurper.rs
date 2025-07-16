//! Client for the VATSIM Slurper API.
//!
//! This module provides a [`SlurperClient`] for retrieving the station name of a currently connected VATSIM client.
//!
//! # Examples
//!
//! ```rust
//! #[cfg(test)]
//! mod tests {
//!     use vacs_vatsim::slurper::SlurperClient;
//!     use wiremock::matchers::{method, path, query_param};
//!     use wiremock::{Mock, MockServer, ResponseTemplate};
//!
//!     #[tokio::test]
//!     async fn get_station_name_empty() -> anyhow::Result<()> {
//!         let server = MockServer::start().await;
//!         Mock::given(method("GET"))
//!             .and(path("/users/info"))
//!             .and(query_param("cid", "1234567"))
//!             .respond_with(ResponseTemplate::new(200))
//!             .mount(&server)
//!             .await;
//!
//!         let client = SlurperClient::new(&server.uri())?;
//!
//!         let station_name = client
//!             .get_station_name("1234567")
//!             .await?;
//!
//!         assert_eq!(station_name, None);
//!         Ok(())
//!     }
//! }
//! ```

use anyhow::Context;

/// User-Agent string used for all HTTP requests.
static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Default timeout for HTTP requests against the slurper API.
/// Can be overwritten using [`SlurperClient::with_timeout`].
const SLURPER_DEFAULT_HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);
/// User information endpoint for the slurper API.
const SLURPER_USER_INFO_ENDPOINT: &str = "/users/info";
/// Index of the station name field in the slurper CSV line.
/// Fields are listed in the [VATSIM Slurper API docs](https://vatsim.dev/api/slurper-api/get-user-info).
const SLURPER_STATION_NAME_FIELD_INDEX: usize = 1;
/// Index of the facility type field in the slurper CSV line.
/// Fields are listed in the [VATSIM Slurper API docs](https://vatsim.dev/api/slurper-api/get-user-info).
const SLURPER_FACILITY_TYPE_FIELD_INDEX: usize = 2;
/// Slurper facility type for ATC clients.
const SLURPER_FACILITY_TYPE_ATC: &str = "atc";
/// Slurper facility type for pilots.
const SLURPER_FACILITY_TYPE_PILOT: &str = "pilot";

/// Client for accessing the VATSIM Slurper API.
pub struct SlurperClient {
    /// HTTP client used for all requests.
    client: reqwest::Client,
    /// Full URL for the user information endpoint.
    user_info_endpoint_url: String,
}

impl SlurperClient {
    /// Creates a new [`SlurperClient`] with the given API base URL.
    ///
    /// A default HTTP timeout is set ([`SLURPER_DEFAULT_HTTP_TIMEOUT`]), which can be overwritten
    /// using [`SlurperClient::with_timeout`] if necessary.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use vacs_vatsim::slurper::SlurperClient;
    ///
    /// let client = SlurperClient::new("https://slurper.vatsim.net").unwrap();
    /// ```
    pub fn new(api_base_url: &str) -> anyhow::Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .user_agent(APP_USER_AGENT)
            .timeout(SLURPER_DEFAULT_HTTP_TIMEOUT)
            .build()
            .context("Failed to create HTTP client")?;
        Ok(Self {
            client,
            user_info_endpoint_url: format!("{api_base_url}{SLURPER_USER_INFO_ENDPOINT}"),
        })
    }

    /// Creates a version of the [`SlurperClient`] with a user-defined [`std::time::Duration`] timeout.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use vacs_vatsim::slurper::SlurperClient;
    /// use std::time::Duration;
    ///
    /// let client = SlurperClient::new("https://slurper.vatsim.net")
    ///     .unwrap()
    ///     .with_timeout(Duration::from_secs(2))
    ///     .unwrap();
    /// ```
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> anyhow::Result<Self> {
        self.client = reqwest::ClientBuilder::new()
            .user_agent(APP_USER_AGENT)
            .timeout(timeout)
            .build()
            .context("Failed to create HTTP client")?;
        Ok(self)
    }

    /// Fetches the ATC station name for a given CID.
    ///
    /// This method queries the Slurper user info API for the given CID and returns the corresponding
    /// station name, if available.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(String))` if an active VATSIM ATC connection was found.
    /// - `Ok(None)` if no active VATSIM connection was found or the CID is connected as a pilot.
    /// - `Err(anyhow::Error)` if retrieving or parsing the data failed.
    ///
    /// # Examples
    /// ```rust
    /// use vacs_vatsim::slurper::SlurperClient;
    /// use wiremock::matchers::{method, path, query_param};
    /// use wiremock::{Mock, MockServer, ResponseTemplate};
    ///
    /// #[tokio::test]
    /// async fn get_station_name() -> anyhow::Result<()> {
    ///     let server = MockServer::start().await;
    ///     Mock::given(method("GET"))
    ///         .and(path("/users/info"))
    ///         .and(query_param("cid", "1234567"))
    ///         .respond_with(ResponseTemplate::new(200).set_body_string(
    ///             "1234567,LOVV_CTR,atc,123.450,600,47.66667,14.33333,0,0,0,0,0,0,0,0,\n",
    ///         ))
    ///         .mount(&server)
    ///         .await;
    ///
    ///     let client = SlurperClient::new(&server.uri())?;
    ///
    ///     let station_name = client
    ///         .get_station_name("1234567")
    ///         .await?;
    ///
    ///     assert_eq!(station_name, Some("LOVV_CTR".to_string()));
    ///     Ok(())
    ///  }
    /// ```
    pub async fn get_station_name(&self, cid: &str) -> anyhow::Result<Option<String>> {
        tracing::trace!(?cid, "Retrieving station name for CID");

        if cid.is_empty() {
            tracing::debug!("CID is empty, returning None");
            return Ok(None);
        }

        let body = self.fetch_slurper_data(cid).await?;
        if body.is_empty() {
            tracing::debug!(?cid, "CID is not present in slurper, returning None");
            return Ok(None);
        }

        self.parse_slurper_data(cid, body)
    }

    /// Performs an HTTP request to fetch the user info data from the Slurper API.
    async fn fetch_slurper_data(&self, cid: &str) -> anyhow::Result<bytes::Bytes> {
        tracing::trace!(?cid, "Performing HTTP request");
        let response = self
            .client
            .get(self.user_info_endpoint_url.as_str())
            .query(&[("cid", cid)])
            .send()
            .await
            .context("Failed to perform HTTP request")?
            .error_for_status()
            .context("Received non-200 HTTP status code")?;

        tracing::trace!(?cid, content_length = ?response.content_length(), "Reading response body");
        let body = response
            .bytes()
            .await
            .context("Failed to read response body")?;

        Ok(body)
    }

    /// Parses the CSV data retrieved from the Slurper user info endpoint and returns the
    /// extracted station name.
    fn parse_slurper_data(&self, cid: &str, body: bytes::Bytes) -> anyhow::Result<Option<String>> {
        tracing::trace!(?cid, "Reading CSV");
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(body.as_ref());
        match reader.records().next() {
            Some(Ok(record)) => self.extract_station_name(cid, record),
            _ => Err(anyhow::anyhow!("Received empty slurper CSV")),
        }
    }

    /// Extracts the station name from the parsed [`csv::StringRecord`], validating the client is
    /// currently logged in using an ATC connection.
    fn extract_station_name(
        &self,
        cid: &str,
        record: csv::StringRecord,
    ) -> anyhow::Result<Option<String>> {
        let facility_type = record
            .get(SLURPER_FACILITY_TYPE_FIELD_INDEX)
            .unwrap_or(SLURPER_FACILITY_TYPE_PILOT);

        if !facility_type.eq_ignore_ascii_case(SLURPER_FACILITY_TYPE_ATC) {
            tracing::debug!(?cid, "CID is pilot, returning None");
            return Ok(None);
        }

        let station_name = match record.get(SLURPER_STATION_NAME_FIELD_INDEX) {
            Some(station_name) => station_name,
            None => {
                tracing::debug!(
                    ?cid,
                    "CID is not present in CSV record in slurper, returning None"
                );
                return Ok(None);
            }
        };

        if station_name.is_empty() {
            tracing::debug!(?cid, "Empty station name, returning None");
            return Ok(None);
        }

        tracing::debug!(?cid, ?station_name, "Found station name for CID");
        Ok(Some(station_name.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::time::Duration;
    use test_log::test;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn new_client() -> anyhow::Result<()> {
        let client = SlurperClient::new("https://example.org")?;
        assert_eq!(
            client.user_info_endpoint_url,
            "https://example.org/users/info"
        );
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_station_name() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/info"))
            .and(query_param("cid", "1234567"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "1234567,LOVV_CTR,atc,123.450,600,47.66667,14.33333,0,0,0,0,0,0,0,0,\n",
            ))
            .mount(&server)
            .await;

        let client = SlurperClient::new(&server.uri()).context("Failed to create client")?;

        let station_name = client
            .get_station_name("1234567")
            .await
            .context("Failed to get station name")?;

        assert_eq!(station_name, Some("LOVV_CTR".to_string()));
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_station_name_pilot() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/info"))
            .and(query_param("cid", "1234567"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("1234567,AUA123,pilot,,,47.66667,14.33333,0,0,0,0,0,0,0,0,\n"),
            )
            .mount(&server)
            .await;

        let client = SlurperClient::new(&server.uri()).context("Failed to create client")?;

        let station_name = client
            .get_station_name("1234567")
            .await
            .context("Failed to get station name")?;

        assert_eq!(station_name, None);
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_station_name_empty() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/info"))
            .and(query_param("cid", "1234567"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let client = SlurperClient::new(&server.uri()).context("Failed to create client")?;

        let station_name = client
            .get_station_name("1234567")
            .await
            .context("Failed to get station name")?;

        assert_eq!(station_name, None);
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_station_name_empty_cid() -> anyhow::Result<()> {
        let client = SlurperClient::new("http://localhost").context("Failed to create client")?;

        let station_name = client
            .get_station_name("")
            .await
            .context("Failed to get station name")?;

        assert_eq!(station_name, None);
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_station_name_non_200_status_code() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/info"))
            .and(query_param("cid", "1234567"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = SlurperClient::new(&server.uri()).context("Failed to create client")?;

        let result = client.get_station_name("1234567").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().chain().any(|e| {
            e.to_string()
                .contains("HTTP status client error (404 Not Found)")
        }));
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_station_name_timeout() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/info"))
            .and(query_param("cid", "1234567"))
            .respond_with(ResponseTemplate::new(404).set_delay(Duration::from_millis(100)))
            .mount(&server)
            .await;

        let client = SlurperClient::new(&server.uri())?
            .with_timeout(Duration::from_millis(50))
            .context("Failed to create client")?;

        let result = client.get_station_name("1234567").await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .chain()
                .any(|e| e.to_string().contains("operation timed out"))
        );
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_station_name_missing_facility_type() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/info"))
            .and(query_param("cid", "1234567"))
            .respond_with(ResponseTemplate::new(200).set_body_string("1234567,LOVV_CTR,\n"))
            .mount(&server)
            .await;

        let client = SlurperClient::new(&server.uri())?
            .with_timeout(Duration::from_millis(50))
            .context("Failed to create client")?;

        let station_name = client
            .get_station_name("1234567")
            .await
            .context("Failed to get station name")?;

        assert_eq!(station_name, None);
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_station_name_empty_station_name() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/info"))
            .and(query_param("cid", "1234567"))
            .respond_with(ResponseTemplate::new(200).set_body_string("1234567,,atc\n"))
            .mount(&server)
            .await;

        let client = SlurperClient::new(&server.uri())?
            .with_timeout(Duration::from_millis(50))
            .context("Failed to create client")?;

        let station_name = client
            .get_station_name("1234567")
            .await
            .context("Failed to get station name")?;

        assert_eq!(station_name, None);
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_station_name_missing_station_name() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/info"))
            .and(query_param("cid", "1234567"))
            .respond_with(ResponseTemplate::new(200).set_body_string("1234567\n"))
            .mount(&server)
            .await;

        let client = SlurperClient::new(&server.uri())?
            .with_timeout(Duration::from_millis(50))
            .context("Failed to create client")?;

        let station_name = client
            .get_station_name("1234567")
            .await
            .context("Failed to get station name")?;

        assert_eq!(station_name, None);
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_station_name_empty_csv_record() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/info"))
            .and(query_param("cid", "1234567"))
            .respond_with(ResponseTemplate::new(200).set_body_string("\n"))
            .mount(&server)
            .await;

        let client = SlurperClient::new(&server.uri())?
            .with_timeout(Duration::from_millis(50))
            .context("Failed to create client")?;

        let result = client.get_station_name("1234567").await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .chain()
                .any(|e| e.to_string().contains("Received empty slurper CSV"))
        );
        Ok(())
    }
}
