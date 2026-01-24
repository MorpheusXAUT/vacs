use crate::app::state::http::HttpState;
use crate::app::state::webrtc::{AppStateWebrtcExt, UnansweredCallGuard};
use crate::app::state::{AppState, AppStateInner, sealed};
use crate::audio::manager::{AudioManagerHandle, SourceType};
use crate::config::{BackendEndpoint, WS_LOGIN_TIMEOUT};
use crate::error::{Error, FrontendError};
use crate::signaling::auth::TauriTokenProvider;
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;
use vacs_signaling::client::{SignalingClient, SignalingEvent, State};
use vacs_signaling::error::{SignalingError, SignalingRuntimeError};
use vacs_signaling::protocol::http::webrtc::IceConfig;
use vacs_signaling::protocol::vatsim::{ClientId, PositionId};
use vacs_signaling::protocol::ws::client::{CallRejectReason, ClientMessage};
use vacs_signaling::protocol::ws::server::{
    DisconnectReason, LoginFailureReason, ServerMessage, SessionProfile,
};
use vacs_signaling::protocol::ws::shared::{CallErrorReason, CallId, ErrorReason};
use vacs_signaling::protocol::ws::{client, server, shared};
use vacs_signaling::transport::tokio::TokioTransport;

const INCOMING_CALLS_LIMIT: usize = 5;

pub trait AppStateSignalingExt: sealed::Sealed {
    async fn connect_signaling(
        &self,
        app: &AppHandle,
        position_id: Option<PositionId>,
    ) -> Result<(), Error>;
    async fn disconnect_signaling(&mut self, app: &AppHandle);
    async fn handle_signaling_connection_closed(&mut self, app: &AppHandle);
    async fn send_signaling_message(&mut self, msg: impl Into<ClientMessage>) -> Result<(), Error>;
    fn set_outgoing_call_peer_id(&mut self, peer_id: Option<ClientId>);
    fn remove_outgoing_call_peer_id(&mut self, peer_id: &ClientId) -> bool;
    fn incoming_call_peer_ids_len(&self) -> usize;
    fn add_incoming_call_peer_id(&mut self, peer_id: &ClientId);
    fn remove_incoming_call_peer_id(&mut self, peer_id: &ClientId) -> bool;
    fn add_call_to_call_list(&mut self, app: &AppHandle, peer_id: &ClientId, incoming: bool);
    fn new_signaling_client(
        app: AppHandle,
        ws_url: &str,
        shutdown_token: CancellationToken,
        max_reconnect_attempts: u8,
    ) -> SignalingClient<TokioTransport, TauriTokenProvider>;
    fn start_unanswered_call_timer(&mut self, app: &AppHandle, peer_id: &ClientId);
    fn cancel_unanswered_call_timer(&mut self, peer_id: &ClientId);
    async fn accept_call(
        &mut self,
        app: &AppHandle,
        peer_id: Option<ClientId>,
    ) -> Result<bool, Error>;
    async fn end_call(&mut self, app: &AppHandle, peer_id: Option<ClientId>)
    -> Result<bool, Error>;
}

impl AppStateSignalingExt for AppStateInner {
    async fn connect_signaling(
        &self,
        app: &AppHandle,
        position_id: Option<PositionId>,
    ) -> Result<(), Error> {
        if self.signaling_client.state() != State::Disconnected {
            log::info!("Already connected and logged in with signaling server");
            return Err(Error::Signaling(Box::from(SignalingError::Other(
                "Already connected".to_string(),
            ))));
        }

        log::info!("Connecting to signaling server with position ID: {position_id:?}");
        match self.signaling_client.connect(position_id).await {
            Ok(()) => {}
            Err(SignalingError::LoginError(LoginFailureReason::AmbiguousVatsimPosition(
                positions,
            ))) => {
                log::warn!(
                    "Connection to signaling server failed, ambiguous VATSIM position: {positions:?}"
                );
                app.emit("signaling:ambiguous-position", &positions).ok();
                return Err(SignalingError::LoginError(
                    LoginFailureReason::AmbiguousVatsimPosition(positions),
                )
                .into());
            }
            Err(err) => return Err(err.into()),
        }

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

    async fn send_signaling_message(&mut self, msg: impl Into<ClientMessage>) -> Result<(), Error> {
        let msg = msg.into();
        log::trace!("Sending signaling message: {msg:?}");

        if let Err(err) = self.signaling_client.send(msg).await {
            log::warn!("Failed to send signaling message: {err:?}");
            return Err(err.into());
        }

        log::trace!("Successfully sent signaling message");
        Ok(())
    }

    fn set_outgoing_call_peer_id(&mut self, peer_id: Option<ClientId>) {
        self.outgoing_call_peer_id = peer_id;
    }

    fn remove_outgoing_call_peer_id(&mut self, peer_id: &ClientId) -> bool {
        if let Some(id) = &self.outgoing_call_peer_id
            && id == peer_id
        {
            self.outgoing_call_peer_id = None;
            self.audio_manager.read().stop(SourceType::Ringback);
            true
        } else {
            false
        }
    }

    fn incoming_call_peer_ids_len(&self) -> usize {
        self.incoming_call_peer_ids.len()
    }

    fn add_incoming_call_peer_id(&mut self, peer_id: &ClientId) {
        self.incoming_call_peer_ids.insert(peer_id.clone());
    }

    fn remove_incoming_call_peer_id(&mut self, peer_id: &ClientId) -> bool {
        let found = self.incoming_call_peer_ids.remove(peer_id);
        if self.incoming_call_peer_ids.is_empty() {
            self.audio_manager.read().stop(SourceType::Ring);
        }
        found
    }

    fn add_call_to_call_list(&mut self, app: &AppHandle, peer_id: &ClientId, incoming: bool) {
        #[derive(Clone, Serialize)]
        #[serde(rename_all = "camelCase")]
        struct CallListEntry<'a> {
            peer_id: &'a ClientId,
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
            false, // TODO custom profile
            WS_LOGIN_TIMEOUT,
            max_reconnect_attempts,
            tauri::async_runtime::handle().inner(),
        )
    }

    fn start_unanswered_call_timer(&mut self, app: &AppHandle, peer_id: &ClientId) {
        self.cancel_unanswered_call_timer(peer_id);

        let timeout = Duration::from_secs(self.config.client.auto_hangup_seconds);
        if timeout.is_zero() {
            return;
        }

        let cancel = self.shutdown_token.child_token();

        let handle = tauri::async_runtime::spawn({
            let app = app.clone();
            let peer_id = peer_id.clone();
            let cancel = cancel.clone();
            async move {
                log::debug!("Starting unanswered call timer of {timeout:?} for peer {peer_id}");
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => {
                        log::debug!("Unanswered call timer cancelled for peer {peer_id}");
                    }
                    _ = tokio::time::sleep(timeout) => {
                        log::debug!("Unanswered call timer expired for peer {peer_id}, hanging up");

                        let state = app.state::<AppState>();
                        let mut state = state.lock().await;

                        // TODO end with route
                        // if let Err(err) = state.send_signaling_message(SignalingMessage::CallEnd { client_id: peer_id.clone() }).await {
                        //     log::warn!("Failed to send call end message after call timer expired: {err:?}");
                        // }

                        state.cleanup_call(&peer_id).await;
                        state.set_outgoing_call_peer_id(None);

                        let audio_manager = app.state::<AudioManagerHandle>();
                        audio_manager.read().stop(SourceType::Ringback);

                        state.emit_call_error(&app, peer_id, false, CallErrorReason::AutoHangup);
                    }
                }
            }
        });

        self.unanswered_call_guard = Some(UnansweredCallGuard {
            peer_id: peer_id.clone(),
            cancel,
            handle,
        });
    }

    fn cancel_unanswered_call_timer(&mut self, peer_id: &ClientId) {
        if let Some(guard) = self
            .unanswered_call_guard
            .take_if(|g| g.peer_id == *peer_id)
        {
            log::trace!(
                "Cancelling unanswered call timer for peer {}",
                guard.peer_id
            );
            guard.cancel.cancel();
            guard.handle.abort();
        }
    }

    async fn accept_call(
        &mut self,
        app: &AppHandle,
        client_id: Option<ClientId>,
    ) -> Result<bool, Error> {
        let client_id =
            match client_id.or_else(|| self.incoming_call_peer_ids.iter().next().cloned()) {
                Some(id) => id,
                None => return Ok(false),
            };
        log::debug!("Accepting call from {client_id}");

        if !self.config.ice.is_default() && self.is_ice_config_expired() {
            match app
                .state::<HttpState>()
                .http_get::<IceConfig>(BackendEndpoint::IceConfig, None)
                .await
            {
                Ok(config) => {
                    self.config.ice = config;
                }
                Err(err) => {
                    log::warn!("Failed to refresh ICE config, using cached one: {err:?}");
                }
            };
        }

        self.send_signaling_message(shared::CallAccept {
            call_id: CallId::new(), // TODO set actual call ID
            accepting_client_id: client_id.clone(),
        })
        .await?;
        self.remove_incoming_call_peer_id(&client_id);

        self.audio_manager.read().stop(SourceType::Ring);

        app.emit("signaling:call-accept", client_id).ok();

        Ok(true)
    }

    async fn end_call(
        &mut self,
        app: &AppHandle,
        peer_id: Option<ClientId>,
    ) -> Result<bool, Error> {
        let Some(peer_id) = peer_id.or_else(|| {
            self.active_call_peer_id()
                .or(self.outgoing_call_peer_id.as_ref())
                .cloned()
        }) else {
            return Ok(false);
        };
        log::debug!("Ending call with {peer_id}");

        self.send_signaling_message(shared::CallEnd {
            call_id: CallId::new(),            // TODO set actual call ID
            ending_client_id: peer_id.clone(), // TODO set own client ID
        })
        .await?;

        self.cleanup_call(&peer_id).await;

        self.cancel_unanswered_call_timer(&peer_id);
        self.set_outgoing_call_peer_id(None);

        self.audio_manager.read().stop(SourceType::Ringback);

        app.emit("signaling:force-call-end", peer_id).ok();

        Ok(true)
    }
}

impl AppStateInner {
    async fn handle_signaling_event(app: &AppHandle, event: SignalingEvent) {
        match event {
            SignalingEvent::Connected {
                client_info,
                profile,
            } => {
                log::debug!(
                    "Successfully connected to signaling server. Display name: {}, frequency: {}, profile: {profile:?}",
                    &client_info.display_name,
                    &client_info.frequency,
                );

                app.emit(
                    "signaling:connected",
                    server::SessionInfo {
                        client: client_info,
                        profile: SessionProfile::Changed(profile),
                    },
                )
                .ok();
            }
            SignalingEvent::Message(msg) => Self::handle_signaling_message(msg, app).await,
            SignalingEvent::Error(error) => {
                if error.is_fatal() {
                    let state = app.state::<AppState>();
                    let mut state = state.lock().await;
                    state.handle_signaling_connection_closed(app).await;

                    if let SignalingRuntimeError::Disconnected(Some(
                        DisconnectReason::AmbiguousVatsimPosition(positions),
                    )) = error
                    {
                        log::warn!(
                            "Disconnected from signaling server, ambiguous VATSIM position: {positions:?}"
                        );

                        app.emit("signaling:ambiguous-position", &positions).ok();
                    } else if error.can_reconnect() {
                        app.emit("signaling:reconnecting", Value::Null).ok();
                    } else {
                        app.emit::<FrontendError>("error", Error::from(error).into())
                            .ok();
                    }
                }
            }
        }
    }

    async fn handle_signaling_message(msg: ServerMessage, app: &AppHandle) {
        match msg {
            ServerMessage::CallInvite(shared::CallInvite {
                call_id,
                source,
                target,
            }) => {
                let caller_id = &source.client_id;
                {
                    if app
                        .state::<AppState>()
                        .lock()
                        .await
                        .config
                        .client
                        .ignored
                        .contains(caller_id)
                    {
                        log::trace!("Ignoring call invite from {caller_id}");
                        return;
                    }
                }
                log::trace!("Call invite received from {caller_id} for target {target:?}");

                let state = app.state::<AppState>();
                let mut state = state.lock().await;

                state.add_call_to_call_list(app, caller_id, true);

                if state.incoming_call_peer_ids_len() >= INCOMING_CALLS_LIMIT {
                    if let Err(err) = state
                        .send_signaling_message(client::CallReject {
                            call_id,
                            rejecting_client_id: ClientId::new(""), // TODO set actual call ID
                            reason: Some(CallRejectReason::Busy),
                        })
                        .await
                    {
                        log::warn!("Failed to reject call invite: {err:?}");
                    }
                    return;
                }

                state.add_incoming_call_peer_id(caller_id);
                app.emit("signaling:call-invite", caller_id).ok();

                state.audio_manager.read().restart(SourceType::Ring);
            }
            ServerMessage::CallAccept(shared::CallAccept {
                call_id,
                accepting_client_id,
            }) => {
                log::trace!("Call accept received for call {call_id} from {accepting_client_id}");

                let state = app.state::<AppState>();
                let mut state = state.lock().await;

                state.cancel_unanswered_call_timer(&accepting_client_id);
                let res = if state.remove_outgoing_call_peer_id(&accepting_client_id) {
                    app.emit("signaling:call-accept", accepting_client_id.clone())
                        .ok();

                    match state
                        .init_call(app.clone(), accepting_client_id.clone(), None)
                        .await
                    {
                        Ok(sdp) => {
                            state
                                .send_signaling_message(shared::WebrtcOffer {
                                    call_id,
                                    from_client_id: ClientId::new(""), // TODO set actual client ID
                                    to_client_id: accepting_client_id.clone(),
                                    sdp,
                                })
                                .await
                        }
                        Err(err) => {
                            log::warn!("Failed to start call: {err:?}");

                            let reason: CallErrorReason = err.into();
                            state.emit_call_error(app, accepting_client_id.clone(), true, reason);
                            state
                                .send_signaling_message(shared::CallError {
                                    call_id,
                                    reason,
                                    message: None,
                                })
                                .await
                        }
                    }
                } else {
                    log::warn!("Received call accept message for peer that is not set as outgoing");
                    state
                        .send_signaling_message(shared::CallError {
                            call_id,
                            reason: CallErrorReason::CallFailure,
                            message: None,
                        })
                        .await
                };

                if let Err(err) = res {
                    log::warn!("Failed to send call message: {err:?}");
                }
            }
            ServerMessage::WebrtcOffer(shared::WebrtcOffer {
                call_id,
                from_client_id,
                to_client_id,
                sdp,
            }) => {
                log::trace!("WebRTC offer for call {call_id} received from {from_client_id}");

                let state = app.state::<AppState>();
                let mut state = state.lock().await;

                let res = match state
                    .init_call(app.clone(), from_client_id.clone(), Some(sdp))
                    .await
                {
                    Ok(sdp) => {
                        state
                            .send_signaling_message(shared::WebrtcAnswer {
                                call_id,
                                to_client_id: from_client_id,
                                from_client_id: to_client_id,
                                sdp,
                            })
                            .await
                    }
                    Err(err) => {
                        log::warn!("Failed to accept call offer: {err:?}");
                        let reason: CallErrorReason = err.into();
                        state.emit_call_error(app, from_client_id.clone(), true, reason);
                        state
                            .send_signaling_message(shared::CallError {
                                call_id,
                                reason,
                                message: None,
                            })
                            .await
                    }
                };

                if let Err(err) = res {
                    log::warn!("Failed to send call message: {err:?}");
                }
            }
            ServerMessage::WebrtcAnswer(shared::WebrtcAnswer {
                call_id,
                from_client_id,
                sdp,
                ..
            }) => {
                log::trace!("WebRTC answer for call {call_id} received from {from_client_id}");

                let state = app.state::<AppState>();
                let mut state = state.lock().await;

                if let Err(err) = state.accept_call_answer(&from_client_id, sdp).await {
                    log::warn!("Failed to accept answer: {err:?}");
                    if let Err(err) = state
                        .send_signaling_message(shared::CallError {
                            call_id,
                            reason: err.into(),
                            message: None,
                        })
                        .await
                    {
                        log::warn!("Failed to send call end message: {err:?}");
                    }
                };
            }
            ServerMessage::CallEnd(shared::CallEnd {
                call_id,
                ending_client_id,
            }) => {
                log::trace!("Call end for call {call_id} received from {ending_client_id}");

                let state = app.state::<AppState>();
                let mut state = state.lock().await;

                if !state.cleanup_call(&ending_client_id).await {
                    log::debug!("Received call end message for peer that is not active");
                }

                state.remove_incoming_call_peer_id(&ending_client_id);

                app.emit("signaling:call-end", &ending_client_id).ok();
            }
            ServerMessage::CallError(shared::CallError {
                call_id,
                reason,
                message,
            }) => {
                log::trace!(
                    "Call error for call {call_id} received. Reason: {reason:?}, message: {message:?}"
                );

                // let state = app.state::<AppState>();
                // let mut state = state.lock().await;

                // TODO cleanup call with call ID?
                // if !state.cleanup_call(&peer_id).await {
                //     log::debug!("Received call end message for peer that is not active");
                // }

                // state.remove_outgoing_call_peer_id(&peer_id);
                // state.remove_incoming_call_peer_id(&peer_id);

                // state.cancel_unanswered_call_timer(&peer_id);

                // TODO emit call ID?
                // state.emit_call_error(app, peer_id, false, reason);
            }
            ServerMessage::CallCancelled(server::CallCancelled { call_id, reason }) => {
                log::trace!("Call {call_id} cancelled. Reason: {reason:?}");

                // let state = app.state::<AppState>();
                // let mut state = state.lock().await;

                // TODO everything
                // state.cancel_unanswered_call_timer(&peer_id);
                // if state.remove_outgoing_call_peer_id(&peer_id) {
                //     app.emit("signaling:call-reject", peer_id).ok();
                // } else {
                //     log::warn!("Received call reject message for peer that is not set as outgoing");
                // }
            }
            ServerMessage::WebrtcIceCandidate(shared::WebrtcIceCandidate {
                call_id,
                from_client_id,
                candidate,
                ..
            }) => {
                log::trace!("ICE candidate for call {call_id} received from {from_client_id}");

                let state = app.state::<AppState>();
                let state = state.lock().await;

                state
                    .set_remote_ice_candidate(&from_client_id, candidate)
                    .await;
            }
            ServerMessage::ClientConnected(server::ClientConnected { client }) => {
                log::trace!("Client connected: {client:?}");

                app.emit("signaling:client-connected", client).ok();
            }
            ServerMessage::ClientDisconnected(server::ClientDisconnected { client_id }) => {
                log::trace!("Client disconnected: {client_id:?}");

                let state = app.state::<AppState>();
                let mut state = state.lock().await;

                // Stop any active webrtc call
                state.cleanup_call(&client_id).await;

                // Remove from outgoing and incoming states
                state.remove_outgoing_call_peer_id(&client_id);
                state.remove_incoming_call_peer_id(&client_id);

                state.cancel_unanswered_call_timer(&client_id);

                app.emit("signaling:client-disconnected", client_id).ok();
            }
            ServerMessage::ClientList(server::ClientList { clients }) => {
                log::trace!("Received client list: {} clients connected", clients.len());

                app.emit("signaling:client-list", clients).ok();
            }
            ServerMessage::ClientInfo(info) => {
                log::trace!("Received client info: {info:?}");

                app.emit("signaling:client-connected", info).ok();
            }
            ref msg @ ServerMessage::SessionInfo(server::SessionInfo {
                ref client,
                ref profile,
            }) => {
                log::trace!("Received session info: {client:?}");

                if let SessionProfile::Changed(profile) = profile {
                    log::trace!("Active profile changed: {profile:?}");
                } else {
                    log::trace!("Active profile unchanged");
                }

                app.emit("signaling:connected", msg).ok();
            }
            ServerMessage::StationList(server::StationList { stations }) => {
                log::trace!(
                    "Received station list: {} stations covered ({} by self)",
                    stations.len(),
                    stations.iter().filter(|s| s.own).count()
                );

                app.emit("signaling:station-list", stations).ok();
            }
            ServerMessage::StationChanges(server::StationChanges { changes }) => {
                log::trace!(
                    "Received station changes: {} stations changed",
                    changes.len()
                );

                app.emit("signaling:station-changes", changes).ok();
            }
            ServerMessage::Error(shared::Error { reason, client_id }) => match reason {
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
                    let client_id = client_id.unwrap_or_default();
                    log::warn!(
                        "Received peer connection error from signaling server with peer {client_id}"
                    );

                    let state = app.state::<AppState>();
                    let mut state = state.lock().await;

                    if !state.cleanup_call(&client_id).await {
                        log::debug!(
                            "Received peer connection error message for peer that is not active"
                        );
                    }

                    state.remove_outgoing_call_peer_id(&client_id);
                    state.remove_incoming_call_peer_id(&client_id);

                    state.cancel_unanswered_call_timer(&client_id);

                    state.emit_call_error(app, client_id, false, CallErrorReason::SignalingFailure);
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
                ErrorReason::RateLimited { retry_after_secs } => {
                    log::warn!(
                        "Received rate limited error from signaling server, rate limited for {retry_after_secs}"
                    );

                    if let Some(peer_id) = client_id {
                        let state = app.state::<AppState>();
                        let mut state = state.lock().await;

                        state.cleanup_call(&peer_id).await;
                        state.remove_outgoing_call_peer_id(&peer_id);
                        state.remove_incoming_call_peer_id(&peer_id);

                        app.emit("signaling:force-call-end", peer_id).ok();
                    }
                    app.emit::<FrontendError>(
                        "error",
                        FrontendError::from(Error::from(SignalingRuntimeError::RateLimited(
                            retry_after_secs.into(),
                        ))),
                    )
                    .ok();
                }
                ErrorReason::ClientNotFound => {
                    let client_id = client_id.unwrap_or_default();
                    log::warn!(
                        "Received client not found error from signaling server with peer {client_id}"
                    );
                    // TODO everything
                }
            },
            _ => {}
        }
    }

    async fn cleanup_signaling(&mut self, app: &AppHandle) {
        self.incoming_call_peer_ids.clear();
        self.outgoing_call_peer_id = None;

        {
            let mut audio_manager = self.audio_manager.write();
            audio_manager.stop(SourceType::Ring);
            audio_manager.stop(SourceType::Ringback);

            audio_manager.detach_call_output();
            audio_manager.detach_input_device();
        }

        self.keybind_engine.read().await.set_call_active(false);

        if let Some(peer_id) = self.active_call_peer_id().cloned() {
            self.cleanup_call(&peer_id).await;
        };
        let peer_ids = self.held_calls.keys().cloned().collect::<Vec<_>>();
        for peer_id in peer_ids {
            self.cleanup_call(&peer_id).await;
            app.emit("signaling:call-end", &peer_id).ok();
        }

        if let Some(guard) = self.unanswered_call_guard.take() {
            log::trace!(
                "Cancelling unanswered call timer for peer {}",
                guard.peer_id
            );
            guard.cancel.cancel();
            guard.handle.abort();
        }
    }
}
