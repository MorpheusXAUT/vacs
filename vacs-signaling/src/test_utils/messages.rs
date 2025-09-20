use crate::client::SignalingEvent;
use async_trait::async_trait;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite;

#[async_trait]
pub trait AwaitSignalingEventExt {
    async fn recv_with_timeout<F>(
        &mut self,
        timeout: Duration,
        predicate: F,
    ) -> anyhow::Result<SignalingEvent>
    where
        F: Fn(&SignalingEvent) -> bool + Send;
}

#[async_trait]
impl AwaitSignalingEventExt for broadcast::Receiver<SignalingEvent> {
    async fn recv_with_timeout<F>(
        &mut self,
        timeout: Duration,
        predicate: F,
    ) -> anyhow::Result<SignalingEvent>
    where
        F: Fn(&SignalingEvent) -> bool + Send,
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

#[async_trait]
pub trait AwaitTungsteniteMessageExt {
    async fn recv_with_timeout<F>(
        &mut self,
        timeout: Duration,
        predicate: F,
    ) -> anyhow::Result<tungstenite::Message>
    where
        F: Fn(&tungstenite::Message) -> bool + Send;
}

#[async_trait]
impl AwaitTungsteniteMessageExt for broadcast::Receiver<tungstenite::Message> {
    async fn recv_with_timeout<F>(
        &mut self,
        timeout: Duration,
        predicate: F,
    ) -> anyhow::Result<tungstenite::Message>
    where
        F: Fn(&tungstenite::Message) -> bool + Send,
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
