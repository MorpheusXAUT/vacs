use crate::state::AppState;
use crate::ws::auth::handle_websocket_login;
use crate::ws::message::send_message;
use axum::extract::ws::WebSocket;
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::StreamExt;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::Instrument;
use vacs_protocol::ws::{LoginFailureReason, SignalingMessage};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        let span = tracing::trace_span!("websocket_connection", addr = %addr, client_id = tracing::field::Empty);
        async move {
            handle_socket(socket, state).await;
        }.instrument(span)
    })
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    tracing::trace!("Handling new websocket connection");

    let (mut websocket_tx, mut websocket_rx) = socket.split();

    let client_id =
        match handle_websocket_login(state.clone(), &mut websocket_rx, &mut websocket_tx).await {
            Some(id) => id,
            None => return,
        };

    tracing::Span::current().record("client_id", &client_id);

    let (mut client, mut rx) = match state.register_client(&client_id).await {
        Ok(client) => client,
        Err(_) => {
            if let Err(err) = send_message(
                &mut websocket_tx,
                SignalingMessage::LoginFailure {
                    reason: LoginFailureReason::DuplicateId,
                },
            )
            .await
            {
                tracing::warn!(?err, "Failed to send login failure message");
            }
            return;
        }
    };

    let (mut broadcast_rx, mut shutdown_rx) = state.get_client_receivers();

    client
        .handle_interaction(
            &state,
            websocket_rx,
            websocket_tx,
            &mut broadcast_rx,
            &mut rx,
            &mut shutdown_rx,
            client_id.as_str()
        )
        .await;

    state.unregister_client(&client_id).await;

    tracing::trace!("Finished handling websocket connection");
}
