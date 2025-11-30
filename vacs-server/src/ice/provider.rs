pub mod cloudflare;
pub mod stun;

use crate::ice::IceError;
use vacs_protocol::http::webrtc::IceConfig;

#[async_trait::async_trait]
pub trait IceConfigProvider: Send + Sync {
    async fn get_ice_config(&self, user_id: &str) -> Result<IceConfig, IceError>;
}
