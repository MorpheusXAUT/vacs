use crate::auth::handle_login;
use crate::state::AppState;
use axum::extract::ws::WebSocket;
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::StreamExt;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::Instrument;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, addr, state))
}

async fn handle_socket(socket: WebSocket, addr: SocketAddr, state: Arc<AppState>) {
    let span = tracing::trace_span!("websocket_connection", addr = %addr, client_id = tracing::field::Empty);
    let _guard = span.enter();

    tracing::trace!("Handling new websocket connection");

    let (mut websocket_sender, mut websocket_receiver) = socket.split();

    let client_id = match handle_login(&mut websocket_receiver, &mut websocket_sender)
        .instrument(span.clone())
        .await
    {
        Some(id) => id,
        None => return,
    };

    let (mut client, mut rx) = match state
        .register_client(&client_id)
        .instrument(span.clone())
        .await
    {
        Ok(client) => client,
        Err(err) => {
            tracing::warn!(?err, "Failed to register client");
            return;
        }
    };

    let (mut broadcast_rx, mut shutdown_rx) = state.get_client_receivers();

    client
        .handle_interaction(
            &state,
            &mut websocket_receiver,
            &mut websocket_sender,
            &mut broadcast_rx,
            &mut rx,
            &mut shutdown_rx,
        )
        .instrument(span.clone())
        .await;

    state
        .unregister_client(&client_id)
        .instrument(span.clone())
        .await;

    tracing::trace!("Finished handling websocket connection");
}
