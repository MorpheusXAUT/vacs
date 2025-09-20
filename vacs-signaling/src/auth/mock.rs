use crate::auth::TokenProvider;
use crate::error::SignalingError;
use async_trait::async_trait;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct MockTokenProvider {
    client_id: usize,
    delay: Option<Duration>,
}

impl MockTokenProvider {
    pub fn new(client_id: usize, delay: Option<Duration>) -> Self {
        Self { client_id, delay }
    }
}

#[async_trait]
impl TokenProvider for MockTokenProvider {
    async fn get_token(&self) -> Result<String, SignalingError> {
        if let Some(delay) = self.delay {
            tokio::time::sleep(delay).await;
        }
        if self.client_id == usize::MAX {
            return Ok("".to_string());
        }
        Ok(format!("token{}", self.client_id))
    }
}
