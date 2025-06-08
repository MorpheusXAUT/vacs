use crate::state::AppState;
use crate::ws::message::send_message;
use crate::ws::ClientSession;
use std::ops::ControlFlow;
use std::sync::Arc;
use vacs_shared::signaling::Message;
use crate::ws::traits::WebSocketSink;

pub async fn handle_application_message<T: WebSocketSink>(
    state: &Arc<AppState>,
    client: &ClientSession,
    websocket_tx: &mut T,
    message: Message,
) -> ControlFlow<(), ()> {
    tracing::trace!(?message, "Handling application message");

    match message {
        Message::ListClients => {
            tracing::trace!("Returning list of clients");
            let clients = state.list_clients().await;
            if let Err(err) = send_message(websocket_tx, Message::ClientList { clients }).await {
                tracing::warn!(?err, "Failed to send client list");
            }
            ControlFlow::Continue(())
        }
        Message::Logout => {
            tracing::trace!("Logging out client");
            ControlFlow::Break(())
        }
        Message::CallOffer { peer_id, sdp } => {
            handle_call_offer(&state, &client, websocket_tx, &peer_id, &sdp).await;
            ControlFlow::Continue(())
        }
        Message::CallAnswer { peer_id, sdp } => {
            handle_call_answer(&state, &client, websocket_tx, &peer_id, &sdp).await;
            ControlFlow::Continue(())
        }
        Message::CallReject { peer_id } => {
            handle_call_reject(&state, &client, websocket_tx, &peer_id).await;
            ControlFlow::Continue(())
        }
        Message::CallIceCandidate { peer_id, candidate } => {
            handle_call_ice_candidate(&state, &client, websocket_tx, &peer_id, &candidate).await;
            ControlFlow::Continue(())
        }
        Message::CallEnd { peer_id } => {
            handle_call_end(&state, &client, websocket_tx, &peer_id).await;
            ControlFlow::Continue(())
        }
        _ => ControlFlow::Continue(()),
    }
}

async fn handle_call_offer<T: WebSocketSink>(
    state: &AppState,
    client: &ClientSession,
    websocket_tx: &mut T,
    peer_id: &str,
    sdp: &str,
) {
    tracing::trace!(?peer_id, "Handling call offer");
    state
        .send_message_to_peer(
            websocket_tx,
            peer_id,
            Message::CallOffer {
                peer_id: client.get_id().to_string(),
                sdp: sdp.to_string(),
            },
        )
        .await;
}

async fn handle_call_answer<T: WebSocketSink>(
    state: &AppState,
    client: &ClientSession,
    websocket_tx: &mut T,
    peer_id: &str,
    sdp: &str,
) {
    tracing::trace!(?peer_id, "Handling call answer");
    state
        .send_message_to_peer(
            websocket_tx,
            peer_id,
            Message::CallAnswer {
                peer_id: client.get_id().to_string(),
                sdp: sdp.to_string(),
            },
        )
        .await;
}

async fn handle_call_reject<T: WebSocketSink>(
    state: &AppState,
    client: &ClientSession,
    websocket_tx: &mut T,
    peer_id: &str,
) {
    tracing::trace!(?peer_id, "Handling call rejection");
    state
        .send_message_to_peer(
            websocket_tx,
            peer_id,
            Message::CallReject {
                peer_id: client.get_id().to_string(),
            },
        )
        .await;
}

async fn handle_call_ice_candidate<T: WebSocketSink>(
    state: &AppState,
    client: &ClientSession,
    websocket_tx: &mut T,
    peer_id: &str,
    candidate: &str,
) {
    tracing::trace!(?peer_id, "Handling call ICE candidate");
    state
        .send_message_to_peer(
            websocket_tx,
            peer_id,
            Message::CallIceCandidate {
                peer_id: client.get_id().to_string(),
                candidate: candidate.to_string(),
            },
        )
        .await;
}

async fn handle_call_end<T: WebSocketSink>(
    state: &AppState,
    client: &ClientSession,
    websocket_tx: &mut T,
    peer_id: &str,
) {
    tracing::trace!(?peer_id, "Handling call end");
    state
        .send_message_to_peer(
            websocket_tx,
            peer_id,
            Message::CallEnd {
                peer_id: client.get_id().to_string(),
            },
        )
        .await;
}
