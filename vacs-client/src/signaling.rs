pub(crate) mod commands;

use crate::app::state::AppState;
use crate::audio::manager::SourceType;
use crate::config::{WS_LOGIN_TIMEOUT, WS_READY_TIMEOUT};
use crate::error::FrontendError;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{oneshot, watch};
use vacs_protocol::ws::{ErrorReason, SignalingMessage};
use vacs_signaling::client::{InterruptionReason, SignalingClient};
use vacs_signaling::error::SignalingError;
use vacs_signaling::transport;

pub struct Connection {
    client: SignalingClient,
    shutdown_tx: watch::Sender<()>,
}

impl Connection {
    pub fn new() -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let client = SignalingClient::new(shutdown_rx);

        Self {
            client,
            shutdown_tx,
        }
    }

    pub async fn connect(
        &mut self,
        app: AppHandle,
        ws_url: &str,
        token: &str,
        on_disconnect: oneshot::Sender<bool>,
    ) -> Result<(), SignalingError> {
        log::info!("Connecting to signaling server");

        log::debug!("Creating signaling connection");
        let (sender, receiver) = transport::tokio::create(ws_url).await?;

        let (ready_tx, ready_rx) = oneshot::channel();
        let mut client = self.client.clone();

        let (cancel_tx, _) = watch::channel(());
        let cancel_tx_clone = cancel_tx.clone();

        let client_task = tauri::async_runtime::spawn(async move {
            log::trace!("Signaling client interaction task started");

            let mut cancel_rx = cancel_tx.subscribe();

            tokio::select! {
                biased;

                _ = cancel_rx.changed() => {
                    log::info!("Cancel signal received, stopping signaling connection client");
                }

                reason = client.start(sender, receiver, ready_tx) => {
                    match reason {
                        InterruptionReason::Disconnected(requested) => {
                            log::debug!(
                                "Signaling client interaction ended due to disconnect. Requested: {requested}"
                            );
                            on_disconnect.send(requested).ok();
                        }
                        InterruptionReason::ShutdownSignal => {
                            log::trace!(
                                "Signaling client interaction ended due to shutdown signal"
                            );
                            on_disconnect.send(false).ok();
                        }
                        InterruptionReason::Error(err) => {
                            log::warn!("Signaling client interaction ended due to error: {err:?}");
                            on_disconnect.send(false).ok();
                        }
                    };
                    cancel_tx.send(()).ok();
                }
            }

            log::trace!("Signaling client task finished");
        });

        let app_clone = app.clone();
        let mut broadcast_rx = self.client.subscribe();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let interaction_task = tauri::async_runtime::spawn(async move {
            log::trace!("Signaling connection interaction task started");

            let mut cancel_rx = cancel_tx_clone.subscribe();

            loop {
                tokio::select! {
                    biased;

                    _ = cancel_rx.changed() => {
                        log::info!("Cancel signal received, stopping signaling connection interaction handling");
                        break;
                    }

                    _ = shutdown_rx.changed() => {
                        log::info!("Shutdown signal received, stopping signaling connection interaction handling");
                        break;
                    }

                    msg = broadcast_rx.recv() => {
                        match msg {
                            Ok(msg) => Self::handle_signaling_message(msg, &app_clone).await,
                            Err(err) => {
                                log::warn!("Received error from signaling client broadcast receiver: {err:?}");
                                break;
                            }
                        }
                    }
                }
            }

            cancel_tx_clone.send(()).ok();

            log::trace!("Signaling connection interaction task finished");
        });

        log::debug!("Waiting for signaling connection to be ready");
        if tokio::time::timeout(WS_READY_TIMEOUT, ready_rx)
            .await
            .is_err()
        {
            log::warn!(
                "Signaling connection did not become ready in time, aborting remaining tasks"
            );
            client_task.abort();
            interaction_task.abort();
            return Err(SignalingError::Timeout(
                "Signaling client did not become ready in time".to_string(),
            ));
        }

        log::debug!("Signaling connection is ready, logging in");
        let clients = match self.client.login(token, WS_LOGIN_TIMEOUT).await {
            Ok(clients) => clients,
            Err(err) => {
                log::warn!("Login failed, aborting connection: {err:?}");
                client_task.abort();
                interaction_task.abort();
                return Err(err);
            }
        };
        log::debug!(
            "Successfully connected to signaling server, {} clients connected",
            clients.len()
        );

        app.emit("signaling:connected", "LOVV_CTR").ok(); // TODO: Update display name
        app.emit("signaling:client-list", clients).ok();

        Ok(())
    }

    pub fn disconnect(&mut self) {
        log::trace!("Disconnect requested for signaling connection");
        self.client.disconnect();
    }

    pub async fn send(&mut self, msg: SignalingMessage) -> Result<(), SignalingError> {
        self.client.send(msg).await
    }

    async fn handle_signaling_message(msg: SignalingMessage, app: &AppHandle) {
        match msg {
            ref call_offer @ SignalingMessage::CallOffer { ref peer_id, .. } => {
                log::trace!("Call offer received from {peer_id}");
                app.emit("signaling:call-offer", call_offer).ok();
                app.state::<AppState>()
                    .lock()
                    .await
                    .audio_manager
                    .lock()
                    .await
                    .restart(SourceType::Ring);
            }
            SignalingMessage::CallAnswer { peer_id, .. } => {
                log::trace!("Call answer received from {peer_id}");
                // TODO start call in webrtc/audio
                app.emit("signaling:call-answer", peer_id).ok();
                app.state::<AppState>()
                    .lock()
                    .await
                    .audio_manager
                    .lock()
                    .await
                    .stop(SourceType::Ringback);
            }
            SignalingMessage::CallReject { peer_id } => {
                log::trace!("Call reject received from {peer_id}");
                app.emit("signaling:call-reject", peer_id).ok();
                app.state::<AppState>()
                    .lock()
                    .await
                    .audio_manager
                    .lock()
                    .await
                    .stop(SourceType::Ringback);
            }
            SignalingMessage::CallEnd { peer_id } => {
                log::trace!("Call end received from {peer_id}");
                // TODO end call in webrtc/audio
                app.emit("signaling:call-end", peer_id).ok();
                app.state::<AppState>()
                    .lock()
                    .await
                    .audio_manager
                    .lock()
                    .await
                    .stop(SourceType::Ring);
            }
            SignalingMessage::CallIceCandidate { peer_id, .. } => {
                log::trace!("ICE candidate received from {peer_id}");
                // TODO pass to webrtc
            }
            SignalingMessage::PeerNotFound { peer_id } => {
                log::trace!("Received peer not found: {peer_id}");
                // TODO end call in webrtc/audio if in call with peer
                app.emit("signaling:peer-not-found", peer_id).ok();
            }
            SignalingMessage::ClientConnected { client } => {
                log::trace!("Client connected: {client:?}");
                app.emit("signaling:client-connected", client).ok();
            }
            SignalingMessage::ClientDisconnected { id } => {
                log::trace!("Client disconnected: {id:?}");
                app.emit("signaling:client-disconnected", id).ok();
            }
            SignalingMessage::ClientList { clients } => {
                log::trace!("Received client list: {} clients connected", clients.len());
                app.emit("signaling:client-list", clients).ok();
            }
            SignalingMessage::Error { reason, peer_id } => match reason {
                ErrorReason::MalformedMessage => {
                    log::warn!("Received malformed error message from signaling server");
                    app.emit::<FrontendError>(
                        "error",
                        FrontendError::new_with_timeout(
                            "Signaling error",
                            "Malformed message",
                            5000,
                        ),
                    )
                    .ok();
                }
                ErrorReason::Internal(msg) => {
                    log::warn!("Received internal error message from signaling server");
                    app.emit::<FrontendError>(
                        "error",
                        FrontendError::new("Signaling error", format!("Internal: {msg}")),
                    )
                    .ok();
                }
                ErrorReason::PeerConnection => {
                    log::warn!(
                        "Received peer connection error from signaling server with peer {}",
                        peer_id.unwrap_or("NONE".to_string())
                    );
                    app.emit::<FrontendError>(
                        "error",
                        FrontendError::new("Signaling error", "Peer connection error".to_string()),
                    )
                    .ok();
                    // TODO: cleanup or set error state in frontend
                }
                ErrorReason::UnexpectedMessage(msg) => {
                    log::warn!("Received unexpected message error from signaling server");
                    app.emit::<FrontendError>(
                        "error",
                        FrontendError::new("Signaling error", format!("Unexpected message: {msg}")),
                    )
                    .ok();
                }
            },
            _ => {}
        }
    }

    pub fn is_connected(&self) -> bool {
        self.client.status().0
    }

    pub fn is_logged_in(&self) -> bool {
        self.client.status().1
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        log::debug!("Signaling connection dropped, sending disconnect signal");
        self.disconnect();
    }
}
