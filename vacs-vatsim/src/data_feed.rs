mod vatsim;
#[cfg(feature = "test-utils")]
pub mod mock;

pub use vatsim::VatsimDataFeed;

use crate::ControllerInfo;
use async_trait::async_trait;

#[async_trait]
pub trait DataFeed: Send + Sync {
    async fn fetch_controller_info(&self) -> anyhow::Result<Vec<ControllerInfo>>;
}
