use crate::data_feed::DataFeed;
use crate::{ControllerInfo, FacilityType};
use anyhow::Context;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::Deserialize;
use std::fmt::{Debug, Formatter};
use std::time::{Duration, Instant};
use tracing::instrument;

const DATA_FEED_DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(1);
const DATA_FEED_DEFAULT_CACHE_TTL: Duration = Duration::from_secs(15);

#[derive(Debug)]
pub struct VatsimDataFeed {
    url: String,
    client: reqwest::Client,
    cache_ttl: Duration,
    cache: RwLock<Option<Cache>>,
}

impl VatsimDataFeed {
    pub fn new(url: &str) -> anyhow::Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .user_agent(crate::APP_USER_AGENT)
            .timeout(DATA_FEED_DEFAULT_HTTP_TIMEOUT)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            url: url.to_string(),
            client,
            cache_ttl: DATA_FEED_DEFAULT_CACHE_TTL,
            cache: Default::default(),
        })
    }

    pub fn with_timeout(mut self, timeout: Duration) -> anyhow::Result<Self> {
        self.client = reqwest::ClientBuilder::new()
            .user_agent(crate::APP_USER_AGENT)
            .timeout(timeout)
            .build()
            .context("Failed to create HTTP client")?;
        Ok(self)
    }

    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self.cache = Default::default();
        self
    }

    #[instrument(level = "trace", skip(self), err)]
    async fn fetch_data_feed(&self) -> anyhow::Result<VatsimDataFeedResponse> {
        tracing::trace!("Fetching VATSIM data feed");
        let response = self
            .client
            .get(self.url.clone())
            .send()
            .await
            .context("Failed to perform HTTP request")?;

        tracing::trace!(content_length = ?response.headers().get(reqwest::header::CONTENT_LENGTH), "Parsing VATSIM data feed response body");
        let body = response
            .json()
            .await
            .context("Failed to parse VATSIM data feed response body")?;

        Ok(body)
    }
}

#[async_trait]
impl DataFeed for VatsimDataFeed {
    #[instrument(level = "debug", skip(self), err)]
    async fn fetch_controller_info(&self) -> anyhow::Result<Vec<ControllerInfo>> {
        tracing::debug!("Fetching controller info");

        if let Some(cache) = self.cache.read().as_ref()
            && cache.updated_at.elapsed() < self.cache_ttl
        {
            tracing::debug!(?cache, "Returning cached controller info");
            return Ok(cache.data.clone());
        }

        let data_feed = self.fetch_data_feed().await?;
        let controllers: Vec<ControllerInfo> =
            data_feed.controllers.into_iter().map(Into::into).collect();

        let cache = Cache {
            data: controllers.clone(),
            updated_at: Instant::now(),
        };
        *self.cache.write() = Some(cache);

        tracing::debug!(controllers = ?controllers.len(), "Returning controller info");
        Ok(controllers)
    }
}

struct Cache {
    data: Vec<ControllerInfo>,
    updated_at: Instant,
}

impl Debug for Cache {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache")
            .field("controllers", &self.data.len())
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

#[derive(Debug, Deserialize)]
struct VatsimDataFeedResponse {
    pub controllers: Vec<VatsimDataFeedController>,
}

#[derive(Debug, Deserialize)]
struct VatsimDataFeedController {
    cid: i32,
    callsign: String,
    frequency: String,
}

impl From<VatsimDataFeedController> for ControllerInfo {
    fn from(value: VatsimDataFeedController) -> Self {
        Self {
            cid: value.cid.to_string(),
            frequency: value.frequency,
            facility_type: FacilityType::from(value.callsign.as_str()),
            callsign: value.callsign,
        }
    }
}
