use async_trait::async_trait;
use std::time::Duration;
use tokio::sync::broadcast;

#[async_trait]
pub trait RecvWithTimeoutExt<T> {
    async fn recv_with_timeout<F>(&mut self, timeout: Duration, predicate: F) -> anyhow::Result<T>
    where
        F: Fn(&T) -> bool + Send;
}

#[async_trait]
impl<T: Clone + Send> RecvWithTimeoutExt<T> for broadcast::Receiver<T> {
    async fn recv_with_timeout<F>(&mut self, timeout: Duration, predicate: F) -> anyhow::Result<T>
    where
        F: Fn(&T) -> bool + Send,
    {
        loop {
            match tokio::time::timeout(timeout, self.recv()).await {
                Ok(Ok(event)) if predicate(&event) => return Ok(event),
                Ok(Err(err)) => return Err(err.into()),
                Err(_) => return Err(anyhow::anyhow!("Timeout")),
                _ => continue,
            }
        }
    }
}
