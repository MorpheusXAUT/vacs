use crate::metrics::{ClientMetrics, ErrorMetrics};
use crate::state::AppState;
use crate::ws::message::{MessageResult, receive_message, send_message_raw};
use axum::extract::ws;
use axum::extract::ws::WebSocket;
use futures_util::stream::{SplitSink, SplitStream};
use semver::Version;
use std::sync::Arc;
use std::time::Duration;
use tracing::instrument;
use vacs_protocol::vatsim::{ActiveProfile, ProfileId};
use vacs_protocol::ws::{ClientInfo, ErrorReason, LoginFailureReason, SignalingMessage};
use vacs_vatsim::{ControllerInfo, FacilityType};

#[instrument(level = "debug", skip_all)]
pub async fn handle_websocket_login(
    state: Arc<AppState>,
    websocket_receiver: &mut SplitStream<WebSocket>,
    websocket_sender: &mut SplitSink<WebSocket, ws::Message>,
) -> Option<(ClientInfo, ActiveProfile<ProfileId>)> {
    tracing::trace!("Handling websocket login flow");
    match tokio::time::timeout(Duration::from_millis(state.config.auth.login_flow_timeout_millis), async {
        loop {
            return match receive_message(websocket_receiver).await {
                MessageResult::ApplicationMessage(SignalingMessage::Login { token, protocol_version, custom_profile, position_id }) => {
                    let is_compatible_protocol = Version::parse(&protocol_version)
                        .map(|version| state.updates.is_compatible_protocol(version)).unwrap_or(false);
                    if !is_compatible_protocol {
                        tracing::debug!("Websocket login flow failed, due to incompatible protocol version");
                        ClientMetrics::login_attempt(false);
                        ClientMetrics::login_failure(LoginFailureReason::IncompatibleProtocolVersion);
                        let login_failure_message = SignalingMessage::LoginFailure {
                            reason: LoginFailureReason::IncompatibleProtocolVersion,
                        };
                        if let Err(err) =
                            send_message_raw(websocket_sender, login_failure_message).await
                        {
                            tracing::warn!(?err, "Failed to send websocket login failure message");
                        }
                        return None;
                    }

                    match state.verify_ws_auth_token(token.as_str()).await {
                        Ok(cid) => {
                            if !state.config.vatsim.require_active_connection {
                                tracing::trace!(?cid, "Websocket token verified, no active VATSIM connection required, websocket login flow completed");
                                let client_info = ClientInfo {
                                    id: cid.clone(),
                                    position_id: None,
                                    display_name: cid.to_string(),
                                    frequency: "".to_string()
                                };
                                return Some((client_info, ActiveProfile::None));
                            }

                            tracing::trace!(?cid, "Websocket token verified, checking for active VATSIM connection");
                            match state.get_vatsim_controller_info(&cid).await {
                                Ok(None) | Ok(Some(ControllerInfo { facility_type: FacilityType::Unknown, ..})) => {
                                    tracing::trace!(?cid, "No active VATSIM connection found, rejecting login");
                                    ClientMetrics::login_attempt(false);
                                    ClientMetrics::login_failure(LoginFailureReason::NoActiveVatsimConnection);
                                    let login_failure_message = SignalingMessage::LoginFailure {
                                        reason: LoginFailureReason::NoActiveVatsimConnection,
                                    };
                                    if let Err(err) =
                                        send_message_raw(websocket_sender, login_failure_message).await
                                    {
                                        tracing::warn!(?err, "Failed to send websocket login failure message");
                                    }
                                    None
                                }
                                Ok(Some(controller_info)) => {
                                    tracing::trace!(?cid, ?controller_info, "VATSIM user info found, resolving matching positions");

                                    let positions = state.clients.find_positions(&controller_info);
                                    let position = if positions.is_empty() {
                                        // TODO: Allow connection with no profile if no position could be found
                                        tracing::trace!(?cid, ?controller_info, "No matching position found");
                                        None
                                    } else if positions.len() == 1 {
                                        tracing::trace!(?cid, ?controller_info, position_id = ?positions[0], "Found matching position");
                                        Some(positions[0])
                                    } else if let Some(target_pid) = position_id.as_ref() {
                                        if let Some(position) = positions.into_iter().find(|p| &p.id == target_pid) {
                                            tracing::trace!(?cid, ?controller_info, ?position, "Found multiple matching positions, user selection is included, assigning selection");
                                            Some(position)
                                        } else {
                                            tracing::trace!(?cid, ?controller_info, ?target_pid, "Found multiple matching positions, but user selection is not included, rejecting login as invalid");

                                            ClientMetrics::login_attempt(false);
                                            ClientMetrics::login_failure(LoginFailureReason::InvalidVatsimPosition);
                                            let login_failure_message = SignalingMessage::LoginFailure {
                                                reason: LoginFailureReason::InvalidVatsimPosition,
                                            };
                                            if let Err(err) =
                                                send_message_raw(websocket_sender, login_failure_message).await
                                            {
                                                tracing::warn!(?err, "Failed to send websocket login failure message");
                                            }
                                            return None;
                                        }
                                    } else {
                                        tracing::trace!(?cid, ?controller_info, positions = positions.len(), "Found multiple matching positions, rejecting login as ambiguous");
                                        let position_ids = positions.into_iter().map(|p| p.id.clone()).collect::<Vec<_>>();

                                        ClientMetrics::login_attempt(false);
                                        ClientMetrics::login_failure(LoginFailureReason::AmbiguousVatsimPosition(position_ids.clone()));
                                        let login_failure_message = SignalingMessage::LoginFailure {
                                            reason: LoginFailureReason::AmbiguousVatsimPosition(position_ids),
                                        };
                                        if let Err(err) =
                                            send_message_raw(websocket_sender, login_failure_message).await
                                        {
                                            tracing::warn!(?err, "Failed to send websocket login failure message");
                                        }
                                        return None
                                    };

                                    let client_info = ClientInfo {
                                        id: cid,
                                        position_id: position.map(|p| p.id.clone()),
                                        display_name: controller_info.callsign.clone(),
                                        frequency: controller_info.frequency.clone(),
                                    };
                                    let active_profile = if custom_profile {
                                        ActiveProfile::Custom
                                    } else {
                                        position.and_then(|p|p.profile_id.as_ref().map(|p| ActiveProfile::Specific(p.clone()))).unwrap_or(ActiveProfile::None)
                                    };

                                    Some((client_info, active_profile))
                                }
                                Err(err) => {
                                    tracing::warn!(?cid, ?err, "Failed to retrieve VATSIM user info");
                                    let reason = ErrorReason::Internal("Failed to retrieve VATSIM connection info".to_string());
                                    ClientMetrics::login_attempt(false);
                                    ErrorMetrics::error(&reason);
                                    let login_failure_message = SignalingMessage::Error {
                                        reason,
                                        peer_id: None,
                                    };
                                    if let Err(err) =
                                        send_message_raw(websocket_sender, login_failure_message).await
                                    {
                                        tracing::warn!(?err, "Failed to send websocket login failure message");
                                    }
                                    None
                                }
                            }
                        }
                        Err(err) => {
                            tracing::debug!(?err, "Websocket login flow failed");
                            ClientMetrics::login_attempt(false);
                            ClientMetrics::login_failure(LoginFailureReason::InvalidCredentials);
                            let login_failure_message = SignalingMessage::LoginFailure {
                                reason: LoginFailureReason::InvalidCredentials,
                            };
                            if let Err(err) =
                                send_message_raw(websocket_sender, login_failure_message).await
                            {
                                tracing::warn!(?err, "Failed to send websocket login failure message");
                            }
                            None
                        }
                    }
                }
                MessageResult::ApplicationMessage(message) => {
                    tracing::debug!(msg = ?message, "Received unexpected message during websocket login flow");
                    ClientMetrics::login_attempt(false);
                    ClientMetrics::login_failure(LoginFailureReason::Unauthorized);
                    let login_failure_message = SignalingMessage::LoginFailure {
                        reason: LoginFailureReason::Unauthorized,
                    };
                    if let Err(err) = send_message_raw(websocket_sender, login_failure_message).await {
                        tracing::warn!(?err, "Failed to send websocket login failure message");
                    }
                    None
                }
                MessageResult::ControlMessage => {
                    tracing::trace!("Skipping control message during websocket login flow");
                    continue;
                }
                MessageResult::Disconnected => {
                    tracing::debug!("Client disconnected during websocket login flow");
                    ClientMetrics::login_attempt(false);
                    None
                }
                MessageResult::Error(err) => {
                    tracing::warn!(?err, "Received error while handling websocket login flow");
                    ClientMetrics::login_attempt(false);
                    None
                }
            };
        }
    }).await {
        Ok(Some(info)) => Some(info),
        Ok(None) => None,
        Err(_) => {
            tracing::debug!("Websocket login flow timed out");
            ClientMetrics::login_attempt(false);
            ClientMetrics::login_failure(LoginFailureReason::Timeout);
            let login_timeout_message = SignalingMessage::LoginFailure {
                reason: LoginFailureReason::Timeout,
            };
            if let Err(err) = send_message_raw(websocket_sender, login_timeout_message).await {
                tracing::warn!(?err, "Failed to send websocket login timeout message");
            }
            None
        }
    }
}
