pub(crate) mod commands;

use tokio::sync::watch;
use vacs_protocol::ws::ClientInfo;
use vacs_signaling::client::SignalingClient;
use vacs_signaling::error::SignalingError;
use vacs_signaling::transport;

pub struct Connection {
    client: SignalingClient<transport::tokio::TokioTransport>,
    shutdown_tx: watch::Sender<()>,
}

impl Connection {
    pub async fn new(ws_url: &str) -> Result<Self, SignalingError> {
        log::trace!("Creating watch channel");
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        log::trace!("Creating tokio transport");
        let transport = transport::tokio::TokioTransport::new(ws_url).await?;
        log::trace!("Creating signaling client");
        let client = SignalingClient::new(transport, shutdown_rx);

        log::trace!("Returning connection");
        Ok(Self {
            client,
            shutdown_tx,
        })
    }

    pub async fn login(&mut self, token: &str)  -> Result<Vec<ClientInfo>, SignalingError> {
        self.client.login(token).await
    }

    pub async fn disconnect(&mut self) -> Result<(), SignalingError> {
        self.client.disconnect().await
    }
}
