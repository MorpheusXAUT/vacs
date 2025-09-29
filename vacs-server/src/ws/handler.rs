use crate::state::AppState;
use crate::ws::auth::handle_websocket_login;
use crate::ws::message::send_message_raw;
use axum::extract::ws::{CloseCode, CloseFrame, Message, Utf8Bytes, WebSocket};
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode as TungsteniteCloseCode;
use tracing::Instrument;
use vacs_protocol::ws::{ClientInfo, LoginFailureReason, SignalingMessage};

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

    let (client_id, user_info) =
        match handle_websocket_login(state.clone(), &mut websocket_rx, &mut websocket_tx).await {
            Some(id) => id,
            None => return,
        };

    tracing::Span::current()
        .record("client_id", &client_id)
        .record("display_name", &user_info.callsign)
        .record("frequency", &user_info.frequency);

    let client_info = ClientInfo {
        id: client_id.to_string(),
        display_name: user_info.callsign,
        frequency: user_info.frequency,
    };

    let (mut client, mut rx) = match state.register_client(client_info.clone()).await {
        Ok(client) => client,
        Err(_) => {
            if let Err(err) = send_message_raw(
                &mut websocket_tx,
                SignalingMessage::LoginFailure {
                    reason: LoginFailureReason::DuplicateId,
                },
            )
            .await
            {
                tracing::warn!(?err, "Failed to send login failure message");
            }

            if let Err(err) = websocket_tx
                .send(Message::Close(Some(CloseFrame {
                    code: CloseCode::from(TungsteniteCloseCode::Protocol),
                    reason: Utf8Bytes::from("Login failure"),
                })))
                .await
            {
                tracing::warn!(?err, "Failed to send close frame");
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
            client_info,
        )
        .await;

    state.unregister_client(&client_id).await;

    tracing::trace!("Finished handling websocket connection");
}
