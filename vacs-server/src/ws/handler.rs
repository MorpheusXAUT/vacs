use crate::metrics::ClientMetrics;
use crate::metrics::guards::ClientConnectionGuard;
use crate::state::AppState;
use crate::ws::auth::handle_websocket_login;
use crate::ws::message::send_message_raw;
use axum::extract::ws::{CloseCode, CloseFrame, Message, Utf8Bytes, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum_client_ip::ClientIp;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode as TungsteniteCloseCode;
use tracing::Instrument;
use vacs_protocol::ws::{LoginFailureReason, SignalingMessage};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    ClientIp(ip): ClientIp,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        let span = tracing::trace_span!("websocket_connection", client_ip = ?ip, client_id = tracing::field::Empty);
        async move {
            handle_socket(socket, state).await;
        }.instrument(span)
    })
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    tracing::trace!("Handling new websocket connection");
    let client_connection_guard = ClientConnectionGuard::new();

    let (mut websocket_tx, mut websocket_rx) = socket.split();

    let Some((client_info, active_profile)) =
        handle_websocket_login(state.clone(), &mut websocket_rx, &mut websocket_tx).await
    else {
        return;
    };

    tracing::Span::current().record("client_id", tracing::field::display(&client_info.id));

    let res = state
        .register_client(client_info, active_profile, client_connection_guard)
        .await;
    let (mut client, mut rx) = match res {
        Ok(client) => client,
        Err(_) => {
            ClientMetrics::login_attempt(false);
            ClientMetrics::login_failure(LoginFailureReason::DuplicateId);

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

    ClientMetrics::login_attempt(true);

    let (mut broadcast_rx, mut shutdown_rx) = state.get_client_receivers();

    client
        .handle_interaction(
            &state,
            websocket_rx,
            websocket_tx,
            &mut broadcast_rx,
            &mut rx,
            &mut shutdown_rx,
        )
        .await;

    state.unregister_client(client.id(), None).await;

    tracing::trace!("Finished handling websocket connection");
}
