use crate::app::state::audio::AppStateAudioExt;
use crate::app::state::webrtc::AppStateWebrtcExt;
use crate::app::state::{AppState, AppStateInner, sealed};
use crate::audio::manager::SourceType;
use crate::config::WS_LOGIN_TIMEOUT;
use crate::error::{Error, FrontendError};
use crate::signaling::auth::TauriTokenProvider;
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;
use vacs_signaling::client::{SignalingClient, SignalingEvent, State};
use vacs_signaling::error::{SignalingError, SignalingRuntimeError};
use vacs_signaling::protocol::ws::{CallErrorReason, ErrorReason, SignalingMessage};
use vacs_signaling::transport::tokio::TokioTransport;

const INCOMING_CALLS_LIMIT: usize = 5;

pub trait AppStateSignalingExt: sealed::Sealed {
    async fn connect_signaling(&self) -> Result<(), Error>;
    async fn disconnect_signaling(&mut self, app: &AppHandle);
    async fn handle_signaling_connection_closed(&mut self, app: &AppHandle);
    async fn send_signaling_message(&mut self, msg: SignalingMessage) -> Result<(), Error>;
    fn set_outgoing_call_peer_id(&mut self, peer_id: Option<String>);
    fn remove_outgoing_call_peer_id(&mut self, peer_id: &str) -> bool;
    fn incoming_call_peer_ids_len(&self) -> usize;
    fn add_incoming_call_peer_id(&mut self, peer_id: &str);
    fn remove_incoming_call_peer_id(&mut self, peer_id: &str) -> bool;
    fn add_call_to_call_list(&mut self, app: &AppHandle, peer_id: &str, incoming: bool);
    fn new_signaling_client(
        app: AppHandle,
        ws_url: &str,
        shutdown_token: CancellationToken,
        max_reconnect_attempts: u8,
    ) -> SignalingClient<TokioTransport, TauriTokenProvider>;
}

impl AppStateSignalingExt for AppStateInner {
    async fn connect_signaling(&self) -> Result<(), Error> {
        log::info!("Connecting to signaling server");

        if self.signaling_client.state() != State::Disconnected {
            log::info!("Already connected and logged in with signaling server");
            return Err(Error::Signaling(Box::from(SignalingError::Other(
                "Already connected".to_string(),
            ))));
        }

        log::debug!("Connecting to signaling server");
        self.signaling_client.connect().await?;

        log::info!("Successfully connected to signaling server");
        Ok(())
    }

    async fn disconnect_signaling(&mut self, app: &AppHandle) {
        log::info!("Disconnecting from signaling server");

        self.cleanup_signaling(app).await;
        app.emit("signaling:disconnected", Value::Null).ok();
        self.signaling_client.disconnect().await;

        log::debug!("Successfully disconnected from signaling server");
    }

    async fn handle_signaling_connection_closed(&mut self, app: &AppHandle) {
        log::info!("Handling signaling server connection closed");

        self.cleanup_signaling(app).await;

        app.emit("signaling:disconnected", Value::Null).ok();
        log::debug!("Successfully handled closed signaling server connection");
    }

    async fn send_signaling_message(&mut self, msg: SignalingMessage) -> Result<(), Error> {
        log::trace!("Sending signaling message: {msg:?}");

        if let Err(err) = self.signaling_client.send(msg).await {
            log::warn!("Failed to send signaling message: {err:?}");
            return Err(err.into());
        }

        log::trace!("Successfully sent signaling message");
        Ok(())
    }

    fn set_outgoing_call_peer_id(&mut self, peer_id: Option<String>) {
        self.outgoing_call_peer_id = peer_id;
    }

    fn remove_outgoing_call_peer_id(&mut self, peer_id: &str) -> bool {
        if let Some(id) = &self.outgoing_call_peer_id
            && id == peer_id
        {
            self.outgoing_call_peer_id = None;
            self.audio_manager.stop(SourceType::Ringback);
            true
        } else {
            false
        }
    }

    fn incoming_call_peer_ids_len(&self) -> usize {
        self.incoming_call_peer_ids.len()
    }

    fn add_incoming_call_peer_id(&mut self, peer_id: &str) {
        self.incoming_call_peer_ids.insert(peer_id.to_string());
    }

    fn remove_incoming_call_peer_id(&mut self, peer_id: &str) -> bool {
        let found = self.incoming_call_peer_ids.remove(peer_id);
        if self.incoming_call_peer_ids.is_empty() {
            self.audio_manager.stop(SourceType::Ring);
        }
        found
    }

    fn add_call_to_call_list(&mut self, app: &AppHandle, peer_id: &str, incoming: bool) {
        #[derive(Clone, Serialize)]
        #[serde(rename_all = "camelCase")]
        struct CallListEntry<'a> {
            peer_id: &'a str,
            incoming: bool,
        }

        app.emit(
            "signaling:add-to-call-list",
            CallListEntry { peer_id, incoming },
        )
        .ok();
    }

    fn new_signaling_client(
        app: AppHandle,
        ws_url: &str,
        shutdown_token: CancellationToken,
        max_reconnect_attempts: u8,
    ) -> SignalingClient<TokioTransport, TauriTokenProvider> {
        SignalingClient::new(
            TokioTransport::new(ws_url),
            TauriTokenProvider::new(app.clone()),
            move |e| {
                let handle = app.clone();
                async move {
                    Self::handle_signaling_event(&handle, e).await;
                }
            },
            shutdown_token,
            WS_LOGIN_TIMEOUT,
            max_reconnect_attempts,
            tauri::async_runtime::handle().inner(),
        )
    }
}

impl AppStateInner {
    async fn handle_signaling_event(app: &AppHandle, event: SignalingEvent) {
        match event {
            SignalingEvent::Connected { client_info } => {
                log::debug!(
                    "Successfully connected to signaling server. Display name: {}, frequency: {}",
                    &client_info.display_name,
                    &client_info.frequency,
                );

                app.emit("signaling:connected", client_info).ok();
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
            SignalingMessage::ClientInfo { own, info } => {
                let event = if own {
                    "signaling:connected"
                } else {
                    "signaling:client-connected"
                };
                app.emit(event, info).ok();
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

    async fn cleanup_signaling(&mut self, app: &AppHandle) {
        self.incoming_call_peer_ids.clear();
        self.outgoing_call_peer_id = None;

        self.audio_manager.stop(SourceType::Ring);
        self.audio_manager.stop(SourceType::Ringback);

        self.audio_manager.detach_call_output();
        self.audio_manager.detach_input_device();

        if let Some(peer_id) = self.active_call_peer_id().cloned() {
            self.end_call(&peer_id).await;
        };
        let peer_ids = self.held_calls.keys().cloned().collect::<Vec<_>>();
        for peer_id in peer_ids {
            self.end_call(&peer_id).await;
            app.emit("signaling:call-end", &peer_id).ok();
        }
    }
}
