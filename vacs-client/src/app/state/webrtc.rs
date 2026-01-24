use crate::app::state::signaling::AppStateSignalingExt;
use crate::app::state::{AppState, AppStateInner, sealed};
use crate::config::{ENCODED_AUDIO_FRAME_BUFFER_SIZE, ICE_CONFIG_EXPIRY_LEEWAY};
use crate::error::{CallError, Error};
use anyhow::Context;
use std::fmt::{Debug, Formatter};
use std::time::UNIX_EPOCH;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use vacs_signaling::protocol::http::webrtc::IceConfig;
use vacs_signaling::protocol::vatsim::ClientId;
use vacs_signaling::protocol::ws::shared;
use vacs_signaling::protocol::ws::shared::{CallErrorReason, CallId};
use vacs_webrtc::error::WebrtcError;
use vacs_webrtc::{Peer, PeerConnectionState, PeerEvent};

#[derive(Debug)]
pub struct UnansweredCallGuard {
    pub peer_id: ClientId,
    pub cancel: CancellationToken,
    pub handle: JoinHandle<()>,
}

pub struct Call {
    pub(super) peer_id: ClientId,
    peer: Peer,
}

impl Debug for Call {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Call")
            .field("peer_id", &self.peer_id)
            .finish()
    }
}

pub trait AppStateWebrtcExt: sealed::Sealed {
    async fn init_call(
        &mut self,
        app: AppHandle,
        peer_id: ClientId,
        offer_sdp: Option<String>,
    ) -> Result<String, Error>;
    async fn accept_call_answer(&self, peer_id: &ClientId, answer_sdp: String)
    -> Result<(), Error>;
    async fn set_remote_ice_candidate(&self, peer_id: &ClientId, candidate: String);
    async fn cleanup_call(&mut self, peer_id: &ClientId) -> bool;
    fn emit_call_error(
        &self,
        app: &AppHandle,
        peer_id: ClientId,
        is_local: bool,
        reason: CallErrorReason,
    );
    fn active_call_peer_id(&self) -> Option<&ClientId>;
    fn outgoing_call_peer_id(&self) -> Option<&ClientId>;
    fn set_ice_config(&mut self, config: IceConfig);
    fn is_ice_config_expired(&self) -> bool;
}

impl AppStateWebrtcExt for AppStateInner {
    async fn init_call(
        &mut self,
        app: AppHandle,
        peer_id: ClientId,
        offer_sdp: Option<String>,
    ) -> Result<String, Error> {
        if self.active_call.is_some() {
            return Err(WebrtcError::CallActive.into());
        }

        let (peer, mut events_rx) = Peer::new(self.config.ice.clone())
            .await
            .context("Failed to create WebRTC peer")?;

        let sdp = if let Some(sdp) = offer_sdp {
            peer.accept_offer(sdp)
                .await
                .context("Failed to accept WebRTC offer")?
        } else {
            peer.create_offer()
                .await
                .context("Failed to create WebRTC offer")?
        };

        let peer_id_clone = peer_id.clone();

        tauri::async_runtime::spawn(async move {
            loop {
                match events_rx.recv().await {
                    Ok(peer_event) => match peer_event {
                        PeerEvent::ConnectionState(state) => match state {
                            PeerConnectionState::Connected => {
                                log::info!("Connected to peer");

                                let app_state = app.state::<AppState>();
                                let mut state = app_state.lock().await;
                                if let Err(err) =
                                    state.on_peer_connected(&app, &peer_id_clone).await
                                {
                                    let reason: CallErrorReason = err.into();
                                    state.cleanup_call(&peer_id_clone).await;
                                    if let Err(err) = state
                                        .send_signaling_message(shared::CallError {
                                            call_id: CallId::new(), // TODO set actual call ID
                                            reason,
                                            message: None,
                                        })
                                        .await
                                    {
                                        log::warn!("Failed to send call message: {err:?}");
                                    }
                                    state.emit_call_error(
                                        &app,
                                        peer_id_clone.clone(),
                                        true,
                                        reason,
                                    );
                                }
                            }
                            PeerConnectionState::Disconnected => {
                                log::info!("Disconnected from peer");

                                let app_state = app.state::<AppState>();
                                let mut state = app_state.lock().await;

                                if let Some(call) = &mut state.active_call
                                    && call.peer_id == peer_id_clone
                                {
                                    call.peer.pause();
                                    let mut audio_manager = state.audio_manager.write();
                                    audio_manager.detach_call_output();
                                    audio_manager.detach_input_device();
                                }

                                app.emit("webrtc:call-disconnected", &peer_id_clone).ok();
                            }
                            PeerConnectionState::Failed => {
                                log::info!("Connection to peer failed");

                                let app_state = app.state::<AppState>();
                                let mut state = app_state.lock().await;
                                state.cleanup_call(&peer_id_clone).await;

                                state.emit_call_error(
                                    &app,
                                    peer_id_clone.clone(),
                                    true,
                                    CallErrorReason::WebrtcFailure,
                                );
                            }
                            PeerConnectionState::Closed => {
                                // Graceful close
                                log::info!("Peer closed connection");

                                let app_state = app.state::<AppState>();
                                let mut state = app_state.lock().await;
                                state.cleanup_call(&peer_id_clone).await;
                                app.emit("signaling:call-end", &peer_id_clone).ok();
                            }
                            state => {
                                log::trace!("Received connection state: {state:?}");
                            }
                        },
                        PeerEvent::IceCandidate(candidate) => {
                            let app_state = app.state::<AppState>();
                            let mut state = app_state.lock().await;
                            if let Err(err) = state
                                .send_signaling_message(shared::WebrtcIceCandidate {
                                    call_id: CallId::new(),            // TODO set actual call ID
                                    from_client_id: ClientId::new(""), // TODO set actual client ID
                                    to_client_id: peer_id_clone.clone(),
                                    candidate,
                                })
                                .await
                            {
                                log::warn!("Failed to send ICE candidate: {err:?}");
                            }
                        }
                        PeerEvent::Error(err) => {
                            log::warn!("Received error peer event: {err}");
                        }
                    },
                    Err(err) => {
                        log::warn!("Failed to receive peer event: {err:?}");
                        if err == RecvError::Closed {
                            break;
                        }
                    }
                }
            }

            log::trace!("WebRTC events task finished");
        });

        self.active_call = Some(Call { peer_id, peer });

        Ok(sdp)
    }

    async fn accept_call_answer(
        &self,
        peer_id: &ClientId,
        answer_sdp: String,
    ) -> Result<(), Error> {
        if let Some(call) = &self.active_call {
            if call.peer_id == *peer_id {
                call.peer.accept_answer(answer_sdp).await?;
                return Ok(());
            } else {
                log::warn!(
                    "Tried to accept answer, but peer_id does not match. Peer id: {peer_id}"
                );
            }
        }

        Err(WebrtcError::NoCallActive.into())
    }

    async fn set_remote_ice_candidate(&self, peer_id: &ClientId, candidate: String) {
        let res = if let Some(call) = &self.active_call
            && call.peer_id == *peer_id
        {
            call.peer.add_remote_ice_candidate(candidate).await
        } else if let Some(call) = self.held_calls.get(peer_id) {
            call.peer.add_remote_ice_candidate(candidate).await
        } else {
            Err(anyhow::anyhow!("Unknown peer {peer_id}").into())
        };

        if let Err(err) = res {
            log::warn!("Failed to add remote ICE candidate: {err:?}");
        }
    }

    async fn cleanup_call(&mut self, peer_id: &ClientId) -> bool {
        log::debug!(
            "Cleaning up call with peer {peer_id} (active: {:?})",
            self.active_call.as_ref()
        );
        let res = if let Some(call) = &mut self.active_call
            && call.peer_id == *peer_id
        {
            {
                let mut audio_manager = self.audio_manager.write();
                audio_manager.detach_call_output();
                audio_manager.detach_input_device();
            }

            self.keybind_engine.read().await.set_call_active(false);

            let result = call.peer.close().await;
            self.active_call = None;
            result
        } else if let Some(mut call) = self.held_calls.remove(peer_id) {
            call.peer.close().await
        } else {
            Err(anyhow::anyhow!("Unknown peer {peer_id}").into())
        };

        if let Err(err) = &res {
            log::warn!("Failed to cleanup call: {err:?}");
            return false;
        }

        true
    }

    fn emit_call_error(
        &self,
        app: &AppHandle,
        peer_id: ClientId,
        is_local: bool,
        reason: CallErrorReason,
    ) {
        app.emit(
            "webrtc:call-error",
            CallError::new(peer_id, is_local, reason),
        )
        .ok();
    }

    fn active_call_peer_id(&self) -> Option<&ClientId> {
        self.active_call.as_ref().map(|call| &call.peer_id)
    }

    fn outgoing_call_peer_id(&self) -> Option<&ClientId> {
        self.outgoing_call_peer_id.as_ref()
    }

    fn set_ice_config(&mut self, config: IceConfig) {
        self.config.ice = config;
    }

    fn is_ice_config_expired(&self) -> bool {
        if self.config.ice.is_default() {
            return false;
        }

        let expires_at = match self.config.ice.expires_at {
            Some(expires_at) => expires_at,
            None => return false,
        };

        let now = UNIX_EPOCH.elapsed().unwrap_or_default().as_secs();
        if now >= expires_at.saturating_sub(ICE_CONFIG_EXPIRY_LEEWAY.as_secs()) {
            log::debug!(
                "ICE config is expired, expiry {} is less than leeway of {:?}",
                expires_at,
                ICE_CONFIG_EXPIRY_LEEWAY
            );
            true
        } else {
            log::debug!(
                "ICE config is still valid, expiry {} is greater than leeway of {:?}",
                expires_at,
                ICE_CONFIG_EXPIRY_LEEWAY
            );
            false
        }
    }
}

impl AppStateInner {
    async fn on_peer_connected(
        &mut self,
        app: &AppHandle,
        peer_id: &ClientId,
    ) -> Result<(), Error> {
        if let Some(call) = &mut self.active_call
            && call.peer_id == *peer_id
        {
            let (output_tx, output_rx) = mpsc::channel(ENCODED_AUDIO_FRAME_BUFFER_SIZE);
            let (input_tx, input_rx) = mpsc::channel(ENCODED_AUDIO_FRAME_BUFFER_SIZE);

            log::debug!("Starting peer {peer_id} in WebRTC manager");
            if let Err(err) = call.peer.start(input_rx, output_tx) {
                log::warn!("Failed to start peer in WebRTC manager: {err:?}");
                return Err(err.into());
            }

            let attach_muted = {
                let keybind_engine = self.keybind_engine.read().await;
                keybind_engine.set_call_active(true);
                keybind_engine.should_attach_input_muted()
            };

            let audio_config = self.config.audio.clone();
            let mut audio_manager = self.audio_manager.write();
            log::debug!("Attaching call to audio manager");
            if let Err(err) = audio_manager.attach_call_output(
                output_rx,
                audio_config.output_device_volume,
                audio_config.output_device_volume_amp,
            ) {
                log::warn!("Failed to attach call to audio manager: {err:?}");
                return Err(err);
            }

            log::debug!("Attaching input device to audio manager");
            if let Err(err) = audio_manager.attach_input_device(
                app.clone(),
                &audio_config,
                input_tx,
                attach_muted,
            ) {
                log::warn!("Failed to attach input device to audio manager: {err:?}");
                return Err(err);
            }

            log::info!("Successfully established call to peer");
            app.emit("webrtc:call-connected", peer_id).ok();
        } else {
            log::debug!("Peer connected is not the active call, checking held calls");
            if self.held_calls.contains_key(peer_id) {
                log::info!("Held peer connection with peer {peer_id} reconnected");
                app.emit("webrtc:call-connected", peer_id).ok();
            } else {
                log::debug!("Peer {peer_id} is not held, ignoring");
            }
        }
        Ok(())
    }
}
