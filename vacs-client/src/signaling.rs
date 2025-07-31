pub(crate) mod commands;

use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::{Mutex, watch, oneshot};
use tokio::task::JoinSet;
use vacs_protocol::ws::{ClientInfo, SignalingMessage};
use vacs_signaling::client::{InterruptionReason, SignalingClient};
use vacs_signaling::error::SignalingError;
use vacs_signaling::transport;
use crate::config::WS_LOGIN_TIMEOUT;

pub struct Connection {
    client: SignalingClient,
    shutdown_tx: watch::Sender<()>,
    tasks: JoinSet<()>,
}

impl Connection {
    pub async fn new() -> Result<Self, SignalingError> {
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let client = SignalingClient::new(shutdown_rx);

        Ok(Self {
            client,
            shutdown_tx,
            tasks: JoinSet::new(),
        })
    }

    pub async fn login(&mut self, token: &str) -> Result<Vec<ClientInfo>, SignalingError> {
        self.client.login(token, WS_LOGIN_TIMEOUT).await
    }

    pub async fn connect(&mut self, app: AppHandle, ws_url: &str, on_disconnect: oneshot::Sender<()>) -> Result<(), SignalingError> {
        let mut client = self.client.clone();
        let mut broadcast_rx = client.subscribe();

        let (sender, receiver) = transport::tokio::create(ws_url).await?;

        self.tasks.spawn(async move {
            log::trace!("Signaling client interaction task started");

            let reason = client.start(sender, receiver).await;
            match reason {
                InterruptionReason::Disconnected => {
                    log::debug!("Signaling client interaction ended due to disconnect, emitting event");
                    on_disconnect.send(()).ok();
                },
                InterruptionReason::ShutdownSignal => {
                    log::trace!("Signaling client interaction ended due to shutdown signal, not emitting further");
                },
                InterruptionReason::Error(err) => {
                    log::warn!("Signaling client interaction ended due to error: {err:?}");
                },
            };

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
