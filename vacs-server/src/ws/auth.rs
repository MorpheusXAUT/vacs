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
use vacs_protocol::ws::{ErrorReason, LoginFailureReason, SignalingMessage};
use vacs_vatsim::{ControllerInfo, FacilityType};

#[instrument(level = "debug", skip_all)]
pub async fn handle_websocket_login(
    state: Arc<AppState>,
    websocket_receiver: &mut SplitStream<WebSocket>,
    websocket_sender: &mut SplitSink<WebSocket, ws::Message>,
) -> Option<ControllerInfo> {
    tracing::trace!("Handling websocket login flow");
    match tokio::time::timeout(Duration::from_millis(state.config.auth.login_flow_timeout_millis), async {
        loop {
            return match receive_message(websocket_receiver).await {
                MessageResult::ApplicationMessage(SignalingMessage::Login { token, protocol_version }) => {
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
                                return Some(ControllerInfo { cid: cid.to_string(), callsign: cid, frequency: "".to_string(), facility_type: FacilityType::Unknown });
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
                                Ok(Some(user_info)) => {
                                    tracing::trace!(?cid, ?user_info, "VATSIM user info found, websocket login flow completed");
                                    Some(user_info)
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
