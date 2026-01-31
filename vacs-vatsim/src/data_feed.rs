#[cfg(feature = "test-utils")]
pub mod mock;
mod vatsim;

pub use vatsim::VatsimDataFeed;

use crate::ControllerInfo;
use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DataFeedError {
    #[error("Request failed: {0}")]
    Request(#[from] reqwest::Error),
}

#[async_trait]
pub trait DataFeed: Send + Sync {
    async fn fetch_controller_info(&self) -> crate::Result<Vec<ControllerInfo>>;
}
