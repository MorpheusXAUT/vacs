//! Client for the VATSIM Slurper API.
//!
//! This module provides a [`SlurperClient`] for retrieving the controller info of a currently connected VATSIM client.
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
//!     async fn get_user_info_empty() -> anyhow::Result<()> {
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
//!         let controller_info = client
//!             .get_controller_info("1234567")
//!             .await?;
//!
//!         assert_eq!(controller_info, None);
//!         Ok(())
//!     }
//! }
//! ```

use crate::{ControllerInfo, FacilityType};
use anyhow::Context;
use tracing::instrument;

/// Default timeout for HTTP requests against the slurper API.
/// Can be overwritten using [`SlurperClient::with_timeout`].
const SLURPER_DEFAULT_HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);
/// User information endpoint for the slurper API.
const SLURPER_USER_INFO_ENDPOINT: &str = "/users/info";
/// Index of the callsign field in the slurper CSV line.
/// Fields are listed in the [VATSIM Slurper API docs](https://vatsim.dev/api/slurper-api/get-user-info).
const SLURPER_CALLSIGN_FIELD_INDEX: usize = 1;
/// Index of the facility type field in the slurper CSV line.
/// Fields are listed in the [VATSIM Slurper API docs](https://vatsim.dev/api/slurper-api/get-user-info).
const SLURPER_FACILITY_TYPE_FIELD_INDEX: usize = 2;
/// Index of the frequency field in the slurper CSV line.
/// Fields are listed in the [VATSIM Slurper API docs](https://vatsim.dev/api/slurper-api/get-user-info).
const SLURPER_FREQUENCY_FIELD_INDEX: usize = 3;
/// Index of the visibility range field in the slurper CSV line.
/// Fields are listed in the [VATSIM Slurper API docs](https://vatsim.dev/api/slurper-api/get-user-info).
const SLURPER_VISIBILITY_RANGE_FIELD_INDEX: usize = 4;
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
            .user_agent(crate::APP_USER_AGENT)
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
            .user_agent(crate::APP_USER_AGENT)
            .timeout(timeout)
            .build()
            .context("Failed to create HTTP client")?;
        Ok(self)
    }

    /// Fetches the controller info for a given CID.
    ///
    /// This method queries the Slurper user info API for the given CID and returns the corresponding
    /// callsign and frequency, if available.
    /// If multiple entries are found (e.g., the user has connected one or multiple ATIS stations),
    /// the first entry with a visibility range greater than zero is returned.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(ControllerInfo))` if an active VATSIM ATC connection was found.
    /// - `Ok(None)` if no active VATSIM connection was found, the CID is connected as a pilot, or no entry with a visibility range greater than zero was found.
    /// - `Err(anyhow::Error)` if retrieving or parsing the data failed.
    ///
    /// # Examples
    /// ```rust
    /// use vacs_vatsim::slurper::SlurperClient;
    /// use wiremock::matchers::{method, path, query_param};
    /// use wiremock::{Mock, MockServer, ResponseTemplate};
    ///
    /// #[tokio::test]
    /// async fn get_controller_info() -> anyhow::Result<()> {
    ///     let server = MockServer::start().await;
    ///     Mock::given(method("GET"))
    ///         .and(path("/users/info"))
    ///         .and(query_param("cid", "1234567"))
    ///         .respond_with(ResponseTemplate::new(200).set_body_string(
    ///             "1459660,LOWW_D_ATIS,atc,121.730,0,48.11028,16.56972,0,0,0,0,0,0,0,0,\n\
    ///              1234567,LOVV_CTR,atc,123.450,600,47.66667,14.33333,0,0,0,0,0,0,0,0,\n\
    ///              1459660,LOWW_A_ATIS,atc,122.955,0,48.11028,16.56972,0,0,0,0,0,0,0,0,\n",
    ///         ))
    ///         .mount(&server)
    ///         .await;
    ///
    ///     let client = SlurperClient::new(&server.uri())?;
    ///
    ///     let controller_info = client
    ///         .get_controller_info("1234567")
    ///         .await?.unwrap();
    ///
    ///     assert_eq!(controller_info.callsign, "LOVV_CTR");
    ///     assert_eq!(controller_info.frequency, "123.450");
    ///     Ok(())
    ///  }
    /// ```
    #[instrument(level = "debug", skip(self), err)]
    pub async fn get_controller_info(&self, cid: &str) -> anyhow::Result<Option<ControllerInfo>> {
        tracing::debug!("Retrieving controller info for CID");

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
    #[instrument(level = "trace", skip(self), err)]
    async fn fetch_slurper_data(&self, cid: &str) -> anyhow::Result<bytes::Bytes> {
        tracing::trace!("Performing HTTP request");
        let response = self
            .client
            .get(self.user_info_endpoint_url.as_str())
            .query(&[("cid", cid)])
            .send()
            .await
            .context("Failed to perform HTTP request")?
            .error_for_status()
            .context("Received non-200 HTTP status code")?;

        tracing::trace!(content_length = ?response.content_length(), "Reading response body");
        let body = response
            .bytes()
            .await
            .context("Failed to read response body")?;

        Ok(body)
    }

    /// Parses the CSV data retrieved from the Slurper user info endpoint and returns the
    /// extracted [`ControllerInfo`].
    #[instrument(level = "trace", skip(self, body), err)]
    fn parse_slurper_data(
        &self,
        cid: &str,
        body: bytes::Bytes,
    ) -> anyhow::Result<Option<ControllerInfo>> {
        tracing::trace!("Parsing CSV");
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(body.as_ref());

        for result in reader.records() {
            let record = match result {
                Ok(rec) => rec,
                Err(err) => {
                    tracing::warn!(?err, "Failed to parse a CSV record");
                    continue;
                }
            };

            match self.extract_controller_info(cid, record)? {
                Some(info) => return Ok(Some(info)),
                None => continue,
            }
        }

        tracing::debug!("CID is present in slurper, but no valid controller info found, returning None");
        Ok(None)
    }

    /// Extracts the [`ControllerInfo`] from the parsed [`csv::StringRecord`], validating the client is
    /// currently logged in using an ATC connection.
    #[instrument(level = "trace", skip(self), err)]
    fn extract_controller_info(
        &self,
        cid: &str,
        record: csv::StringRecord,
    ) -> anyhow::Result<Option<ControllerInfo>> {
        let facility_type = record
            .get(SLURPER_FACILITY_TYPE_FIELD_INDEX)
            .unwrap_or(SLURPER_FACILITY_TYPE_PILOT);

        if !facility_type.eq_ignore_ascii_case(SLURPER_FACILITY_TYPE_ATC) {
            tracing::trace!("CID is pilot, returning None");
            return Ok(None);
        }

        let visibility_range = record
            .get(SLURPER_VISIBILITY_RANGE_FIELD_INDEX)
            .unwrap_or("0")
            .parse::<i32>()
            .unwrap_or(0);

        if visibility_range == 0 {
            tracing::trace!("Station has no visibility range, returning None");
            return Ok(None);
        }

        let callsign = match record.get(SLURPER_CALLSIGN_FIELD_INDEX) {
            Some(callsign) => callsign,
            None => {
                tracing::trace!("Callsign is not present in CSV record in slurper, returning None");
                return Ok(None);
            }
        };
        if callsign.is_empty() {
            tracing::trace!("Empty callsign, returning None");
            return Ok(None);
        }

        let frequency = match record.get(SLURPER_FREQUENCY_FIELD_INDEX) {
            Some(frequency) => frequency,
            None => {
                tracing::trace!(
                    "Frequency is not present in CSV record in slurper, returning None"
                );
                return Ok(None);
            }
        };
        if frequency.is_empty() {
            tracing::trace!("Empty frequency, returning None");
            return Ok(None);
        }

        let facility_type: FacilityType = callsign.into();

        tracing::debug!(
            ?callsign,
            ?frequency,
            ?facility_type,
            "Found controller info for CID"
        );
        Ok(Some(ControllerInfo {
            cid: cid.to_string(),
            callsign: callsign.to_string(),
            frequency: frequency.to_string(),
            facility_type,
        }))
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
    async fn get_controller_info() -> anyhow::Result<()> {
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

        let controller_info = client
            .get_controller_info("1234567")
            .await
            .context("Failed to get controller info")?
            .expect("No controller info found");

        assert_eq!(controller_info.callsign, "LOVV_CTR".to_string());
        assert_eq!(controller_info.frequency, "123.450".to_string());
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_controller_info_multiple_entries() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/info"))
            .and(query_param("cid", "1234567"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "1459660,LOWW_D_ATIS,atc,121.730,0,48.11028,16.56972,0,0,0,0,0,0,0,0,\n\
                1234567,LOVV_CTR,atc,123.450,600,47.66667,14.33333,0,0,0,0,0,0,0,0,\n\
                1459660,LOWW_A_ATIS,atc,122.955,0,48.11028,16.56972,0,0,0,0,0,0,0,0,\n",
            ))
            .mount(&server)
            .await;

        let client = SlurperClient::new(&server.uri()).context("Failed to create client")?;

        let controller_info = client
            .get_controller_info("1234567")
            .await
            .context("Failed to get controller info")?
            .expect("No controller info found");

        assert_eq!(controller_info.callsign, "LOVV_CTR".to_string());
        assert_eq!(controller_info.frequency, "123.450".to_string());
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_controller_info_pilot() -> anyhow::Result<()> {
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

        let controller_info = client
            .get_controller_info("1234567")
            .await
            .context("Failed to get controller info")?;

        assert_eq!(controller_info, None);
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_controller_info_visibility_range_zero() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/info"))
            .and(query_param("cid", "1234567"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "1459660,LOWW_D_ATIS,atc,121.730,0,48.11028,16.56972,0,0,0,0,0,0,0,0,\n\
                1459660,LOWW_A_ATIS,atc,122.955,0,48.11028,16.56972,0,0,0,0,0,0,0,0,\n",
            ))
            .mount(&server)
            .await;

        let client = SlurperClient::new(&server.uri()).context("Failed to create client")?;

        let controller_info = client
            .get_controller_info("1234567")
            .await
            .context("Failed to get controller info")?;

        assert_eq!(controller_info, None);
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_controller_info_empty() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/info"))
            .and(query_param("cid", "1234567"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let client = SlurperClient::new(&server.uri()).context("Failed to create client")?;

        let controller_info = client
            .get_controller_info("1234567")
            .await
            .context("Failed to get controller info")?;

        assert_eq!(controller_info, None);
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_controller_info_empty_cid() -> anyhow::Result<()> {
        let client = SlurperClient::new("http://localhost").context("Failed to create client")?;

        let controller_info = client
            .get_controller_info("")
            .await
            .context("Failed to get controller info")?;

        assert_eq!(controller_info, None);
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_controller_info_non_200_status_code() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/info"))
            .and(query_param("cid", "1234567"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = SlurperClient::new(&server.uri()).context("Failed to create client")?;

        let result = client.get_controller_info("1234567").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().chain().any(|e| {
            e.to_string()
                .contains("HTTP status client error (404 Not Found)")
        }));
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_controller_info_timeout() -> anyhow::Result<()> {
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

        let result = client.get_controller_info("1234567").await;

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
    async fn get_controller_info_missing_facility_type() -> anyhow::Result<()> {
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

        let controller_info = client
            .get_controller_info("1234567")
            .await
            .context("Failed to get controller info")?;

        assert_eq!(controller_info, None);
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_controller_info_empty_callsign() -> anyhow::Result<()> {
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

        let controller_info = client
            .get_controller_info("1234567")
            .await
            .context("Failed to get controller info")?;

        assert_eq!(controller_info, None);
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_controller_info_missing_callsign() -> anyhow::Result<()> {
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

        let controller_info = client
            .get_controller_info("1234567")
            .await
            .context("Failed to get controller info")?;

        assert_eq!(controller_info, None);
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_controller_info_empty_frequency() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/info"))
            .and(query_param("cid", "1234567"))
            .respond_with(ResponseTemplate::new(200).set_body_string("1234567,,atc,,\n"))
            .mount(&server)
            .await;

        let client = SlurperClient::new(&server.uri())?
            .with_timeout(Duration::from_millis(50))
            .context("Failed to create client")?;

        let controller_info = client
            .get_controller_info("1234567")
            .await
            .context("Failed to get controller info")?;

        assert_eq!(controller_info, None);
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_controller_info_missing_frequency() -> anyhow::Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/info"))
            .and(query_param("cid", "1234567"))
            .respond_with(ResponseTemplate::new(200).set_body_string("1234567,LOVV_CTR,atc\n"))
            .mount(&server)
            .await;

        let client = SlurperClient::new(&server.uri())?
            .with_timeout(Duration::from_millis(50))
            .context("Failed to create client")?;

        let controller_info = client
            .get_controller_info("1234567")
            .await
            .context("Failed to get controller info")?;

        assert_eq!(controller_info, None);
        Ok(())
    }

    #[test(tokio::test)]
    async fn get_controller_info_empty_csv_record() -> anyhow::Result<()> {
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

        let controller_info = client
            .get_controller_info("1234567")
            .await
            .context("Failed to get controller info")?;

        assert_eq!(controller_info, None);
        Ok(())
    }
}
