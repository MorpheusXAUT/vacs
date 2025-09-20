pub mod mock;

use crate::error::SignalingError;
use async_trait::async_trait;

#[async_trait]
pub trait TokenProvider: Send + Sync + 'static {
    async fn get_token(&self) -> Result<String, SignalingError>;
}
