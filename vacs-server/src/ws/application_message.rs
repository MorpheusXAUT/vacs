use crate::state::AppState;
use crate::ws::ClientSession;
use crate::ws::message::send_message;
use crate::ws::traits::WebSocketSink;
use std::ops::ControlFlow;
use std::sync::Arc;
use vacs_protocol::SignalingMessage;

pub async fn handle_application_message<T: WebSocketSink>(
    state: &Arc<AppState>,
    client: &ClientSession,
    websocket_tx: &mut T,
    message: SignalingMessage,
) -> ControlFlow<(), ()> {
    tracing::trace!(?message, "Handling application message");

    match message {
        SignalingMessage::ListClients => {
            tracing::trace!("Returning list of clients");
            let clients = state.list_clients().await;
            if let Err(err) =
                send_message(websocket_tx, SignalingMessage::ClientList { clients }).await
            {
                tracing::warn!(?err, "Failed to send client list");
            }
            ControlFlow::Continue(())
        }
        SignalingMessage::Logout => {
            tracing::trace!("Logging out client");
            ControlFlow::Break(())
        }
        SignalingMessage::CallOffer { peer_id, sdp } => {
            handle_call_offer(state, client, &peer_id, &sdp).await;
            ControlFlow::Continue(())
        }
        SignalingMessage::CallAnswer { peer_id, sdp } => {
            handle_call_answer(state, client, &peer_id, &sdp).await;
            ControlFlow::Continue(())
        }
        SignalingMessage::CallReject { peer_id } => {
            handle_call_reject(state, client, &peer_id).await;
            ControlFlow::Continue(())
        }
        SignalingMessage::CallIceCandidate { peer_id, candidate } => {
            handle_call_ice_candidate(state, client, &peer_id, &candidate).await;
            ControlFlow::Continue(())
        }
        SignalingMessage::CallEnd { peer_id } => {
            handle_call_end(state, client, &peer_id).await;
            ControlFlow::Continue(())
        }
        _ => ControlFlow::Continue(()),
    }
}

async fn handle_call_offer(state: &AppState, client: &ClientSession, peer_id: &str, sdp: &str) {
    tracing::trace!(?peer_id, "Handling call offer");
    state
        .send_message_to_peer(
            client,
            peer_id,
            SignalingMessage::CallOffer {
                peer_id: client.get_id().to_string(),
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
                peer_id: client.get_id().to_string(),
                sdp: sdp.to_string(),
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
                peer_id: client.get_id().to_string(),
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
                peer_id: client.get_id().to_string(),
                candidate: candidate.to_string(),
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
                peer_id: client.get_id().to_string(),
            },
        )
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws::test_util::TestSetup;
    use axum::extract::ws;
    use axum::extract::ws::Utf8Bytes;
    use pretty_assertions::assert_eq;
    use test_log::test;
    use vacs_protocol::LoginFailureReason;

    #[test(tokio::test)]
    async fn handle_application_message_list_clients() {
        let mut setup = TestSetup::new();
        setup.register_client("client1").await;

        let control_flow = handle_application_message(
            &setup.app_state,
            &setup.session,
            &mut setup.mock_sink,
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
                r#"{"ClientList":{"clients":[{"id":"client1","display_name":"client1"}]}}"#
            ))
        )
    }

    #[test(tokio::test)]
    async fn handle_application_message_logout() {
        let mut setup = TestSetup::new();
        setup.register_client("client1").await;

        let control_flow = handle_application_message(
            &setup.app_state,
            &setup.session,
            &mut setup.mock_sink,
            SignalingMessage::Logout,
        )
        .await;
        assert_eq!(control_flow, ControlFlow::Break(()));
    }

    #[test(tokio::test)]
    async fn handle_application_message_call_offer() {
        let mut setup = TestSetup::new();
        let mut clients = setup.register_clients(vec!["client1", "client2"]).await;

        let control_flow = handle_application_message(
            &setup.app_state,
            &setup.session,
            &mut setup.mock_sink,
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
        let mut setup = TestSetup::new();

        let control_flow = handle_application_message(
            &setup.app_state,
            &setup.session,
            &mut setup.mock_sink,
            SignalingMessage::LoginFailure {
                reason: LoginFailureReason::DuplicateId,
            },
        )
        .await;
        assert_eq!(control_flow, ControlFlow::Continue(()));
    }
}
