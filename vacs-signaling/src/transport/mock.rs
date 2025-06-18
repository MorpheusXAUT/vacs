use crate::error::SignalingError;
use crate::transport::SignalingTransport;
use async_trait::async_trait;
use tokio::sync::mpsc;
use vacs_protocol::SignalingMessage;

pub struct MockHandle {
    pub outgoing_rx: mpsc::Receiver<SignalingMessage>,
    pub incoming_tx: mpsc::Sender<SignalingMessage>,
}

pub struct MockTransport {
    outgoing: mpsc::Sender<SignalingMessage>,
    incoming: mpsc::Receiver<SignalingMessage>,
}

impl MockTransport {
    #[tracing::instrument(level = "info")]
    pub fn new() -> (Self, MockHandle) {
        let (outgoing_tx, outgoing_rx) = mpsc::channel(32);
        let (incoming_tx, incoming_rx) = mpsc::channel(32);

        let transport = Self {
            outgoing: outgoing_tx,
            incoming: incoming_rx,
        };

        let handle = MockHandle {
            outgoing_rx,
            incoming_tx,
        };

        (transport, handle)
    }
}

#[async_trait]
impl SignalingTransport for MockTransport {
    #[tracing::instrument(level = "debug", skip(self))]
    async fn send(&mut self, msg: SignalingMessage) -> Result<(), SignalingError> {
        tracing::debug!("Sending SignalingMessage");
        self.outgoing.send(msg).await.map_err(|err| {
            tracing::warn!(?err, "Failed to send SignalingMessage");
            SignalingError::Transport(anyhow::anyhow!(err))
        })
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn recv(&mut self) -> Result<SignalingMessage, SignalingError> {
        match self.incoming.recv().await {
            Some(msg) => {
                tracing::debug!(?msg, "Received SignalingMessage");
                Ok(msg)
            }
            None => {
                tracing::warn!("Channel closed");
                Err(SignalingError::Disconnected)
            }
        }
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn close(&mut self) -> Result<(), SignalingError> {
        self.incoming.close();
        Ok(())
    }
}
