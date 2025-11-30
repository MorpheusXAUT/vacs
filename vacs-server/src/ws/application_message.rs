use crate::state::AppState;
use crate::ws::ClientSession;
use crate::ws::message::send_message;
use axum::extract::ws;
use std::ops::ControlFlow;
use std::sync::Arc;
use tokio::sync::mpsc;
use vacs_protocol::ws::{CallErrorReason, ErrorReason, SignalingMessage};

pub async fn handle_application_message(
    state: &Arc<AppState>,
    client: &ClientSession,
    ws_outbound_tx: &mpsc::Sender<ws::Message>,
    message: SignalingMessage,
) -> ControlFlow<(), ()> {
    tracing::trace!(?message, "Handling application message");

    match message {
        SignalingMessage::ListClients => {
            tracing::trace!("Returning list of clients");
            let clients = state.list_clients_without_self(client.id()).await;
            if let Err(err) =
                send_message(ws_outbound_tx, SignalingMessage::ClientList { clients }).await
            {
                tracing::warn!(?err, "Failed to send client list");
            }
            ControlFlow::Continue(())
        }
        SignalingMessage::Logout => {
            tracing::trace!("Logging out client");
            ControlFlow::Break(())
        }
        SignalingMessage::CallInvite { peer_id } => {
            if check_self_message(ws_outbound_tx, client, peer_id.clone()).await {
                return ControlFlow::Continue(());
            }
            if let Err(until) = state.rate_limiters().check_call_invite(client.id()) {
                tracing::debug!(?until, "Rate limit exceeded, rejecting call invite");
                if let Err(err) = send_message(
                    ws_outbound_tx,
                    SignalingMessage::Error {
                        reason: ErrorReason::RateLimited {
                            retry_after_secs: until.as_secs(),
                        },
                        peer_id: Some(peer_id.to_string()),
                    },
                )
                .await
                {
                    tracing::warn!(?err, "Failed to send rate limit error message");
                }
            } else {
                handle_call_invite(state, client, &peer_id).await;
            }
            ControlFlow::Continue(())
        }
        SignalingMessage::CallAccept { peer_id } => {
            if check_self_message(ws_outbound_tx, client, peer_id.clone()).await {
                return ControlFlow::Continue(());
            }
            handle_call_accept(state, client, &peer_id).await;
            ControlFlow::Continue(())
        }
        SignalingMessage::CallReject { peer_id } => {
            if check_self_message(ws_outbound_tx, client, peer_id.clone()).await {
                return ControlFlow::Continue(());
            }
            handle_call_reject(state, client, &peer_id).await;
            ControlFlow::Continue(())
        }
        SignalingMessage::CallOffer { peer_id, sdp } => {
            if check_self_message(ws_outbound_tx, client, peer_id.clone()).await {
                return ControlFlow::Continue(());
            }
            handle_call_offer(state, client, &peer_id, &sdp).await;
            ControlFlow::Continue(())
        }
        SignalingMessage::CallAnswer { peer_id, sdp } => {
            if check_self_message(ws_outbound_tx, client, peer_id.clone()).await {
                return ControlFlow::Continue(());
            }
            handle_call_answer(state, client, &peer_id, &sdp).await;
            ControlFlow::Continue(())
        }
        SignalingMessage::CallEnd { peer_id } => {
            if check_self_message(ws_outbound_tx, client, peer_id.clone()).await {
                return ControlFlow::Continue(());
            }
            handle_call_end(state, client, &peer_id).await;
            ControlFlow::Continue(())
        }
        SignalingMessage::CallError { peer_id, reason } => {
            if check_self_message(ws_outbound_tx, client, peer_id.clone()).await {
                return ControlFlow::Continue(());
            }
            handle_call_error(state, client, &peer_id, reason).await;
            ControlFlow::Continue(())
        }
        SignalingMessage::CallIceCandidate { peer_id, candidate } => {
            if check_self_message(ws_outbound_tx, client, peer_id.clone()).await {
                return ControlFlow::Continue(());
            }
            handle_call_ice_candidate(state, client, &peer_id, &candidate).await;
            ControlFlow::Continue(())
        }
        _ => ControlFlow::Continue(()),
    }
}

async fn check_self_message(
    ws_outbound_tx: &mpsc::Sender<ws::Message>,
    client: &ClientSession,
    peer_id: String,
) -> bool {
    if peer_id == client.id() {
        tracing::debug!(?peer_id, "Rejecting message to self");
        if let Err(err) = send_message(
            ws_outbound_tx,
            SignalingMessage::Error {
                reason: ErrorReason::UnexpectedMessage("Rejecting message to self".to_string()),
                peer_id: Some(peer_id.to_string()),
            },
        )
        .await
        {
            tracing::warn!(?err, ?peer_id, "Failed to send self message error message");
        }
        return true;
    }
    false
}

async fn handle_call_invite(state: &AppState, client: &ClientSession, peer_id: &str) {
    tracing::trace!(?peer_id, "Handling call invite");
    state
        .send_message_to_peer(
            client,
            peer_id,
            SignalingMessage::CallInvite {
                peer_id: client.id().to_string(),
            },
        )
        .await;
}

async fn handle_call_accept(state: &AppState, client: &ClientSession, peer_id: &str) {
    tracing::trace!(?peer_id, "Handling call acceptance");
    state
        .send_message_to_peer(
            client,
            peer_id,
            SignalingMessage::CallAccept {
                peer_id: client.id().to_string(),
            },
        )
        .await;
}

async fn handle_call_reject(state: &AppState, client: &ClientSession, peer_id: &str) {
    tracing::trace!(?peer_id, "Handling call rejection");
    state
        .send_message_to_peer(
            client,
            peer_id,
            SignalingMessage::CallReject {
                peer_id: client.id().to_string(),
            },
        )
        .await;
}

async fn handle_call_offer(state: &AppState, client: &ClientSession, peer_id: &str, sdp: &str) {
    tracing::trace!(?peer_id, "Handling call offer");
    state
        .send_message_to_peer(
            client,
            peer_id,
            SignalingMessage::CallOffer {
                peer_id: client.id().to_string(),
                sdp: sdp.to_string(),
            },
        )
        .await;
}

async fn handle_call_answer(state: &AppState, client: &ClientSession, peer_id: &str, sdp: &str) {
    tracing::trace!(?peer_id, "Handling call answer");
    state
        .send_message_to_peer(
            client,
            peer_id,
            SignalingMessage::CallAnswer {
                peer_id: client.id().to_string(),
                sdp: sdp.to_string(),
            },
        )
        .await;
}

async fn handle_call_end(state: &AppState, client: &ClientSession, peer_id: &str) {
    tracing::trace!(?peer_id, "Handling call end");
    state
        .send_message_to_peer(
            client,
            peer_id,
            SignalingMessage::CallEnd {
                peer_id: client.id().to_string(),
            },
        )
        .await;
}

async fn handle_call_error(
    state: &AppState,
    client: &ClientSession,
    peer_id: &str,
    reason: CallErrorReason,
) {
    tracing::trace!(?peer_id, "Handling call error");
    state
        .send_message_to_peer(
            client,
            peer_id,
            SignalingMessage::CallError {
                peer_id: client.id().to_string(),
                reason,
            },
        )
        .await;
}

async fn handle_call_ice_candidate(
    state: &AppState,
    client: &ClientSession,
    peer_id: &str,
    candidate: &str,
) {
    tracing::trace!(?peer_id, "Handling call ICE candidate");
    state
        .send_message_to_peer(
            client,
            peer_id,
            SignalingMessage::CallIceCandidate {
                peer_id: client.id().to_string(),
                candidate: candidate.to_string(),
            },
        )
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws::test_util::{TestSetup, create_client_info};
    use axum::extract::ws;
    use axum::extract::ws::Utf8Bytes;
    use pretty_assertions::assert_eq;
    use std::ops::Deref;
    use test_log::test;
    use vacs_protocol::ws::LoginFailureReason;

    #[test(tokio::test)]
    async fn handle_application_message_list_clients_without_self() {
        let mut setup = TestSetup::new();
        setup.register_client(create_client_info(1)).await;

        let control_flow = handle_application_message(
            &setup.app_state,
            &setup.session,
            setup.websocket_tx.lock().await.deref(),
            SignalingMessage::ListClients,
        )
        .await;
        assert_eq!(control_flow, ControlFlow::Continue(()));

        let message = setup
            .take_last_websocket_message()
            .await
            .expect("No message received");
        assert_eq!(
            message,
            ws::Message::Text(Utf8Bytes::from_static(
                r#"{"type":"ClientList","clients":[]}"#
            ))
        )
    }

    #[test(tokio::test)]
    async fn handle_application_message_list_clients() {
        let mut setup = TestSetup::new();
        setup.register_client(create_client_info(1)).await;
        setup.register_client(create_client_info(2)).await;

        let control_flow = handle_application_message(
            &setup.app_state,
            &setup.session,
            setup.websocket_tx.lock().await.deref(),
            SignalingMessage::ListClients,
        )
        .await;
        assert_eq!(control_flow, ControlFlow::Continue(()));

        let message = setup
            .take_last_websocket_message()
            .await
            .expect("No message received");
        assert_eq!(
            message,
            ws::Message::Text(Utf8Bytes::from_static(
                r#"{"type":"ClientList","clients":[{"id":"client2","displayName":"Client 2","frequency":"200.000"}]}"#
            ))
        )
    }

    #[test(tokio::test)]
    async fn handle_application_message_logout() {
        let setup = TestSetup::new();
        setup.register_client(create_client_info(1)).await;

        let control_flow = handle_application_message(
            &setup.app_state,
            &setup.session,
            setup.websocket_tx.lock().await.deref(),
            SignalingMessage::Logout,
        )
        .await;
        assert_eq!(control_flow, ControlFlow::Break(()));
    }

    #[test(tokio::test)]
    async fn handle_application_message_call_offer() {
        let setup = TestSetup::new();
        let client_info_1 = create_client_info(1);
        let client_info_2 = create_client_info(2);
        let mut clients = setup
            .register_clients(vec![client_info_1, client_info_2])
            .await;

        let control_flow = handle_application_message(
            &setup.app_state,
            &setup.session,
            setup.websocket_tx.lock().await.deref(),
            SignalingMessage::CallOffer {
                peer_id: "client2".to_string(),
                sdp: "sdp1".to_string(),
            },
        )
        .await;
        assert_eq!(control_flow, ControlFlow::Continue(()));

        let message = clients
            .get_mut("client2")
            .unwrap()
            .1
            .recv()
            .await
            .expect("Failed to receive message");
        assert_eq!(
            message,
            SignalingMessage::CallOffer {
                peer_id: "client1".to_string(),
                sdp: "sdp1".to_string()
            }
        );
    }

    #[test(tokio::test)]
    async fn handle_application_message_unknown() {
        let setup = TestSetup::new();

        let control_flow = handle_application_message(
            &setup.app_state,
            &setup.session,
            setup.websocket_tx.lock().await.deref(),
            SignalingMessage::LoginFailure {
                reason: LoginFailureReason::DuplicateId,
            },
        )
        .await;
        assert_eq!(control_flow, ControlFlow::Continue(()));
    }

    #[test(tokio::test)]
    async fn check_self_message_allows_regular_message() {
        let setup = TestSetup::new();

        let is_self_message = check_self_message(
            setup.websocket_tx.lock().await.deref(),
            &setup.session,
            "client2".to_string(),
        )
        .await;
        assert_eq!(is_self_message, false);
    }

    #[test(tokio::test)]
    async fn check_self_message_rejects_message_to_self() {
        let setup = TestSetup::new();

        let is_self_message = check_self_message(
            setup.websocket_tx.lock().await.deref(),
            &setup.session,
            "client1".to_string(),
        )
        .await;
        assert_eq!(is_self_message, true);
    }
}
