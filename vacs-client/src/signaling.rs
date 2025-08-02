pub(crate) mod commands;

use crate::config::{WS_LOGIN_TIMEOUT, WS_READY_TIMEOUT};
use tauri::{AppHandle, Emitter};
use tokio::sync::{oneshot, watch};
use tokio::task::JoinSet;
use vacs_protocol::ws::SignalingMessage;
use vacs_signaling::client::{InterruptionReason, SignalingClient};
use vacs_signaling::error::SignalingError;
use vacs_signaling::transport;

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

    pub async fn connect(
        &mut self,
        app: AppHandle,
        ws_url: &str,
        token: &str,
        on_disconnect: oneshot::Sender<()>,
    ) -> Result<(), SignalingError> {
        log::info!("Connecting to signaling server");

        log::debug!("Creating signaling connection");
        let (sender, receiver) = transport::tokio::create(ws_url).await?;

        let (ready_tx, ready_rx) = oneshot::channel();
        let mut client = self.client.clone();
        self.tasks.spawn(async move {
            log::trace!("Signaling client interaction task started");

            let reason = client.start(sender, receiver, ready_tx).await;
            match reason {
                InterruptionReason::Disconnected => {
                    log::debug!("Signaling client interaction ended due to disconnect, emitting event");
                    let _ = on_disconnect.send(());
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

        let app_clone = app.clone();
        let mut broadcast_rx = self.client.subscribe();
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
                            Ok(msg) => Self::handle_signaling_message(msg, &app_clone),
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

        log::debug!("Waiting for signaling connection to be ready");
        if tokio::time::timeout(WS_READY_TIMEOUT, ready_rx).await.is_err() {
            log::warn!(
                "Signaling connection did not become ready in time, aborting remaining tasks"
            );
            self.tasks.abort_all();
            return Err(SignalingError::Timeout(
                "Signaling client did not become ready in time".to_string(),
            ));
        }

        log::debug!("Signaling connection is ready, logging in");
        let clients = match self.client.login(token, WS_LOGIN_TIMEOUT).await {
            Ok(clients) => clients,
            Err(err) => {
                log::warn!("Login failed, aborting connection: {err:?}");
                self.tasks.abort_all();
                return Err(err);
            }
        };
        log::debug!(
            "Successfully connected to signaling server, {} clients connected",
            clients.len()
        );

        app.emit("signaling:connected", "LOVV_CTR").ok(); // TODO: Update display name
        app.emit("signaling:client-list", clients).ok();

        /*match self.tasks.join_next().await {
            Some(Ok(_)) => {
                log::debug!("Signaling connection task ended cleanly");
            }
            Some(Err(err)) => {
                log::error!("Task panicked or failed to join: {err:?}");
            }
            None => {
                log::warn!("All tasks completed unexpectedly");
            }
        }

        log::debug!("Signaling connection task completed, aborting remaining tasks");
        self.tasks.abort_all();*/

        Ok(())
    }

    pub fn disconnect(&mut self) {
        log::trace!("Disconnect requested for signaling connection");
        //let _ = self.shutdown_tx.send(());
        self.client.disconnect();
        self.tasks.abort_all();
    }
    
    pub async fn send(&mut self, msg: SignalingMessage) -> Result<(), SignalingError> {
        self.client.send(msg).await
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
        log::debug!("Signaling connection dropped, sending disconnect signal");
        self.disconnect();
    }
}
