pub(crate) mod commands;

use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, watch, oneshot};
use tokio::task::JoinSet;
use vacs_protocol::ws::{ClientInfo, SignalingMessage};
use vacs_signaling::client::SignalingClient;
use vacs_signaling::error::SignalingError;
use vacs_signaling::transport::tokio::TokioTransport;

pub struct Connection {
    client: Arc<Mutex<SignalingClient<TokioTransport>>>,
    shutdown_tx: watch::Sender<()>,
    tasks: JoinSet<()>,
}

impl Connection {
    pub async fn new(ws_url: &str) -> Result<Self, SignalingError> {
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let transport = TokioTransport::new(ws_url).await?;
        let client = Arc::new(Mutex::new(SignalingClient::new(transport, shutdown_rx)));

        Ok(Self {
            client,
            shutdown_tx,
            tasks: JoinSet::new(),
        })
    }

    pub async fn login(&mut self, token: &str) -> Result<Vec<ClientInfo>, SignalingError> {
        self.client.lock().await.login(token).await
    }

    pub async fn start(&mut self, app: AppHandle, on_disconnect: oneshot::Sender<()>) {
        let client = self.client.clone();
        let mut broadcast_rx = client.lock().await.subscribe();

        self.tasks.spawn(async move {
            log::trace!("Signaling client interaction task started");

            let mut client = client.lock().await;
            client.handle_interaction().await;

            let _ = on_disconnect.send(());

            log::trace!("Signaling client interaction task finished");
        });

        let mut shutdown_rx = self.shutdown_tx.subscribe();
        self.tasks.spawn(async move {
            log::trace!("Signaling connection interaction task started");

            loop {
                tokio::select! {
                    biased;

                    _ = shutdown_rx.changed() => {
                        log::info!("Shutdown signal received, stopping signaling connection handling");
                        break;
                    }

                    msg = broadcast_rx.recv() => {
                        match msg {
                            Ok(msg) => Self::handle_signaling_message(msg, &app),
                            Err(err) => {
                                log::warn!("Received error from signaling client broadcast receiver: {err:?}");
                                break;
                            }
                        }
                    }
                }
            }

            log::trace!("Signaling connection interaction task finished");
        });
    }

    pub async fn stop(&mut self) {
        log::info!("Stopping signaling connection");
        self.shutdown();
        while let Some(res) = self.tasks.join_next().await {
            log::trace!("Signaling connection task joined: {res:?}");
            if let Err(err) = res {
                log::warn!("Task join error while stopping signaling connection: {err:?}")
            }
        }
        log::info!("Successfully stopped signaling connection");
    }

    pub fn shutdown(&self) {
        log::trace!("Shutdown requested for signaling connection");
        let _ = self.shutdown_tx.send(());
    }

    fn handle_signaling_message(msg: SignalingMessage, app: &AppHandle) {
        match msg {
            SignalingMessage::CallOffer { .. } => {}
            SignalingMessage::CallEnd { .. } => {}
            SignalingMessage::CallIceCandidate { .. } => {}
            SignalingMessage::ClientConnected { client } => {
                log::trace!("Client connected: {client:?}");
                app.emit("signaling:client-connected", client).ok();
            }
            SignalingMessage::ClientDisconnected { id } => {
                log::trace!("Client disconnected: {id:?}");
                app.emit("signaling:client-disconnected", id).ok();
            }
            SignalingMessage::Error { .. } => {}
            _ => {}
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        log::debug!("Signaling connection dropped, sending shutdown signal");
        self.shutdown();
    }
}
