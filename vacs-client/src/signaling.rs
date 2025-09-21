pub(crate) mod commands;

use crate::app::state::AppState;
use crate::app::state::audio::AppStateAudioExt;
use crate::app::state::http::HttpState;
use crate::app::state::signaling::AppStateSignalingExt;
use crate::app::state::webrtc::AppStateWebrtcExt;
use crate::audio::manager::SourceType;
use crate::config::{BackendEndpoint, WS_LOGIN_TIMEOUT};
use crate::error::{Error, FrontendError};
use async_trait::async_trait;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;
use vacs_signaling::auth::TokenProvider;
use vacs_signaling::client::{SignalingClient, SignalingEvent, State};
use vacs_signaling::error::{SignalingError, SignalingRuntimeError};
use vacs_signaling::protocol::http::ws::WebSocketToken;
use vacs_signaling::protocol::ws::{CallErrorReason, ErrorReason, SignalingMessage};
use vacs_signaling::transport::tokio::TokioTransport;

const INCOMING_CALLS_LIMIT: usize = 5;

pub struct Connection {
    client: SignalingClient<TokioTransport, TauriTokenProvider>,
    shutdown_token: CancellationToken,
}

impl Connection {
    pub fn new(handle: AppHandle, ws_url: &str, reconnect_max_tries: u8) -> Self {
        let shutdown_token = CancellationToken::new(); // TODO use child of global shutdown token
        let client = SignalingClient::new(
            TokioTransport::new(ws_url),
            TauriTokenProvider::new(handle.clone()),
            move |e| {
                let handle = handle.clone();
                async move {
                    Self::handle_signaling_event(&handle, e).await;
                }
            },
            shutdown_token.clone(),
            WS_LOGIN_TIMEOUT,
            reconnect_max_tries,
            tauri::async_runtime::handle().inner(),
        );

        Self {
            client,
            shutdown_token,
        }
    }

    pub async fn connect(&self) -> Result<(), SignalingError> {
        log::info!("Connecting to signaling server");
        self.client.connect().await
    }

    pub async fn disconnect(&self) {
        log::trace!("Disconnecting from signaling server");
        self.client.disconnect().await;
    }

    pub async fn send(&self, msg: SignalingMessage) -> Result<(), SignalingError> {
        self.client.send(msg).await
    }

    async fn handle_signaling_event(app: &AppHandle, event: SignalingEvent) {
        match event {
            SignalingEvent::Connected {
                display_name,
                clients,
            } => {
                log::debug!(
                    "Successfully connected to signaling server, {} clients connected",
                    clients.len()
                );

                app.emit("signaling:connected", display_name).ok();
                app.emit("signaling:client-list", clients).ok();
            }
            SignalingEvent::Message(msg) => Self::handle_signaling_message(msg, app).await,
            SignalingEvent::Error(error) => {
                if error.is_fatal() {
                    let state = app.state::<AppState>();
                    let mut state = state.lock().await;
                    state.handle_signaling_connection_closed(app).await;

                    if error.can_reconnect() {
                        app.emit("signaling:reconnecting", Value::Null).ok();
                    } else {
                        app.emit::<FrontendError>("error", Error::from(error).into())
                            .ok();
                    }
                }
            }
        }
    }

    async fn handle_signaling_message(msg: SignalingMessage, app: &AppHandle) {
        match msg {
            SignalingMessage::CallInvite { peer_id } => {
                log::trace!("Call invite received from {peer_id}");

                let state = app.state::<AppState>();
                let mut state = state.lock().await;

                state.add_call_to_call_list(app, &peer_id, true);

                if state.incoming_call_peer_ids_len() >= INCOMING_CALLS_LIMIT {
                    if let Err(err) = state
                        .send_signaling_message(SignalingMessage::CallReject {
                            peer_id: peer_id.clone(),
                        })
                        .await
                    {
                        log::warn!("Failed to reject call invite: {err:?}");
                    }
                    return;
                }

                state.add_incoming_call_peer_id(&peer_id);
                app.emit("signaling:call-invite", &peer_id).ok();

                state.audio_manager().restart(SourceType::Ring);
            }
            SignalingMessage::CallAccept { peer_id } => {
                log::trace!("Call accept received from {peer_id}");

                let state = app.state::<AppState>();
                let mut state = state.lock().await;

                let res = if state.remove_outgoing_call_peer_id(&peer_id) {
                    app.emit("signaling:call-accept", peer_id.clone()).ok();

                    match state.init_call(app.clone(), peer_id.clone(), None).await {
                        Ok(sdp) => {
                            state
                                .send_signaling_message(SignalingMessage::CallOffer {
                                    peer_id,
                                    sdp,
                                })
                                .await
                        }
                        Err(err) => {
                            log::warn!("Failed to start call: {err:?}");

                            let reason: CallErrorReason = err.into();
                            state.emit_call_error(app, peer_id.clone(), true, reason.clone());
                            state
                                .send_signaling_message(SignalingMessage::CallError {
                                    peer_id,
                                    reason,
                                })
                                .await
                        }
                    }
                } else {
                    log::warn!("Received call accept message for peer that is not set as outgoing");
                    state
                        .send_signaling_message(SignalingMessage::CallError {
                            peer_id,
                            reason: CallErrorReason::CallFailure,
                        })
                        .await
                };

                if let Err(err) = res {
                    log::warn!("Failed to send call message: {err:?}");
                }
            }
            SignalingMessage::CallOffer { peer_id, sdp } => {
                log::trace!("Call offer received from {peer_id}");

                let state = app.state::<AppState>();
                let mut state = state.lock().await;

                let res = match state
                    .init_call(app.clone(), peer_id.clone(), Some(sdp))
                    .await
                {
                    Ok(sdp) => {
                        state
                            .send_signaling_message(SignalingMessage::CallAnswer { peer_id, sdp })
                            .await
                    }
                    Err(err) => {
                        log::warn!("Failed to accept call offer: {err:?}");
                        let reason: CallErrorReason = err.into();
                        state.emit_call_error(app, peer_id.clone(), true, reason.clone());
                        state
                            .send_signaling_message(SignalingMessage::CallError { peer_id, reason })
                            .await
                    }
                };

                if let Err(err) = res {
                    log::warn!("Failed to send call message: {err:?}");
                }
            }
            SignalingMessage::CallAnswer { peer_id, sdp } => {
                log::trace!("Call answer received from {peer_id}");

                let state = app.state::<AppState>();
                let mut state = state.lock().await;

                if let Err(err) = state.accept_call_answer(&peer_id, sdp).await {
                    log::warn!("Failed to accept answer: {err:?}");
                    if let Err(err) = state
                        .send_signaling_message(SignalingMessage::CallError {
                            peer_id,
                            reason: err.into(),
                        })
                        .await
                    {
                        log::warn!("Failed to send call end message: {err:?}");
                    }
                };
            }
            SignalingMessage::CallEnd { peer_id } => {
                log::trace!("Call end received from {peer_id}");

                let state = app.state::<AppState>();
                let mut state = state.lock().await;

                if !state.end_call(&peer_id).await {
                    log::debug!("Received call end message for peer that is not active");
                }

                state.remove_incoming_call_peer_id(&peer_id);

                app.emit("signaling:call-end", &peer_id).ok();
            }
            SignalingMessage::CallError { peer_id, reason } => {
                log::trace!("Call error received from {peer_id}. Reason: {reason:?}");

                let state = app.state::<AppState>();
                let mut state = state.lock().await;

                if !state.end_call(&peer_id).await {
                    log::debug!("Received call end message for peer that is not active");
                }

                state.remove_outgoing_call_peer_id(&peer_id);
                state.remove_incoming_call_peer_id(&peer_id);

                state.emit_call_error(app, peer_id, false, reason);
            }
            SignalingMessage::CallReject { peer_id } => {
                log::trace!("Call reject received from {peer_id}");

                let state = app.state::<AppState>();
                let mut state = state.lock().await;

                if state.remove_outgoing_call_peer_id(&peer_id) {
                    app.emit("signaling:call-reject", peer_id).ok();
                } else {
                    log::warn!("Received call reject message for peer that is not set as outgoing");
                }
            }
            SignalingMessage::CallIceCandidate { peer_id, candidate } => {
                log::trace!("ICE candidate received from {peer_id}");

                let state = app.state::<AppState>();
                let state = state.lock().await;

                state.set_remote_ice_candidate(&peer_id, candidate).await;
            }
            SignalingMessage::PeerNotFound { peer_id } => {
                log::trace!("Received peer not found: {peer_id}");

                let state = app.state::<AppState>();
                let mut state = state.lock().await;

                // Stop any active webrtc call
                state.end_call(&peer_id).await;

                // Remove from outgoing and incoming states
                state.remove_outgoing_call_peer_id(&peer_id);
                state.remove_incoming_call_peer_id(&peer_id);

                app.emit("signaling:peer-not-found", peer_id).ok();
            }
            SignalingMessage::ClientConnected { client } => {
                log::trace!("Client connected: {client:?}");

                app.emit("signaling:client-connected", client).ok();
            }
            SignalingMessage::ClientDisconnected { id } => {
                log::trace!("Client disconnected: {id:?}");

                let state = app.state::<AppState>();
                let mut state = state.lock().await;

                // Stop any active webrtc call
                state.end_call(&id).await;

                // Remove from outgoing and incoming states
                state.remove_outgoing_call_peer_id(&id);
                state.remove_incoming_call_peer_id(&id);

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
                        FrontendError::from(Error::from(SignalingRuntimeError::ServerError(
                            reason,
                        )))
                        .timeout(5000),
                    )
                    .ok();
                }
                ErrorReason::Internal(ref msg) => {
                    log::warn!("Received internal error message from signaling server: {msg}");

                    app.emit::<FrontendError>(
                        "error",
                        FrontendError::from(Error::from(SignalingRuntimeError::ServerError(
                            reason,
                        ))),
                    )
                    .ok();
                }
                ErrorReason::PeerConnection => {
                    let peer_id = peer_id.unwrap_or_default();
                    log::warn!(
                        "Received peer connection error from signaling server with peer {}",
                        peer_id
                    );

                    let state = app.state::<AppState>();
                    let mut state = state.lock().await;

                    if !state.end_call(&peer_id).await {
                        log::debug!(
                            "Received peer connection error message for peer that is not active"
                        );
                    }

                    state.remove_outgoing_call_peer_id(&peer_id);
                    state.remove_incoming_call_peer_id(&peer_id);

                    state.emit_call_error(app, peer_id, false, CallErrorReason::SignalingFailure);
                }
                ErrorReason::UnexpectedMessage(ref msg) => {
                    log::warn!("Received unexpected message error from signaling server: {msg}");

                    app.emit::<FrontendError>(
                        "error",
                        FrontendError::from(Error::from(SignalingRuntimeError::ServerError(
                            reason,
                        ))),
                    )
                    .ok();
                }
            },
            _ => {}
        }
    }

    pub fn state(&self) -> State {
        self.client.state()
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        log::debug!("Signaling connection dropped, sending disconnect signal");
        self.shutdown_token.cancel();
    }
}

#[derive(Debug, Clone)]
pub struct TauriTokenProvider {
    handle: AppHandle,
}

impl TauriTokenProvider {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }
}

#[async_trait]
impl TokenProvider for TauriTokenProvider {
    async fn get_token(&self) -> Result<String, SignalingError> {
        log::debug!("Retrieving WebSocket auth token");
        let http_state = self.handle.state::<HttpState>();

        let token = http_state
            .http_get::<WebSocketToken>(BackendEndpoint::WsToken, None)
            .await
            .map_err(|err| SignalingError::ProtocolError(err.to_string()))?
            .token;

        log::debug!("Successfully retrieved WebSocket auth token");
        Ok(token)
    }
}
