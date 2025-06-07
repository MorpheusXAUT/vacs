use crate::config;
use crate::ws::message::{MessageResult, receive_message, send_message};
use axum::extract::ws;
use axum::extract::ws::WebSocket;
use futures_util::stream::{SplitSink, SplitStream};
use vacs_shared::signaling;

pub async fn verify_token(_client_id: &str, token: &str) -> anyhow::Result<()> {
    tracing::trace!("Verifying auth token");

    // TODO actual token verification
    if token.is_empty() {
        return Err(anyhow::anyhow!("Invalid token"));
    }

    Ok(())
}

pub async fn handle_login(
    websocket_receiver: &mut SplitStream<WebSocket>,
    websocket_sender: &mut SplitSink<WebSocket, ws::Message>,
) -> Option<String> {
    tracing::trace!("Handling login flow");
    tokio::time::timeout(config::LOGIN_FLOW_TIMEOUT, async {
        loop {
            return match receive_message(websocket_receiver).await {
                MessageResult::ApplicationMessage(signaling::Message::Login { id, token }) => {
                    if verify_token(&id, &token).await.is_err() {
                        let login_failure_message = signaling::Message::LoginFailure {
                            reason: signaling::LoginFailureReason::InvalidCredentials,
                        };
                        if let Err(err) =
                            send_message(websocket_sender, login_failure_message).await
                        {
                            tracing::warn!(?err, "Failed to send login failure message");
                        }
                        return None;
                    }
                    tracing::trace!("Login flow completed");
                    Some(id)
                }
                MessageResult::ApplicationMessage(message) => {
                    tracing::debug!(msg = ?message, "Received unexpected message during login flow");
                    let login_failure_message = signaling::Message::LoginFailure {
                        reason: signaling::LoginFailureReason::InvalidLoginFlow,
                    };
                    if let Err(err) = send_message(websocket_sender, login_failure_message).await {
                        tracing::warn!(?err, "Failed to send login failure message");
                    }
                    None
                }
                MessageResult::ControlMessage => {
                    tracing::trace!("Skipping control message during login");
                    continue;
                }
                MessageResult::Disconnected => {
                    tracing::debug!("Client disconnected during login flow");
                    None
                }
                MessageResult::Error(err) => {
                    tracing::warn!(?err, "Received error while handling login flow");
                    None
                }
            };
        }
    })
    .await
    .unwrap_or_else(|_| {
        tracing::debug!("Login timed out");
        None
    })
}
