use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, tungstenite};
use vacs_protocol::ws::SignalingMessage;

pub async fn connect_to_websocket(addr: &str) -> WebSocketStream<MaybeTlsStream<TcpStream>> {
    let (ws_stream, response) = tokio_tungstenite::connect_async(addr)
        .await
        .expect("Failed to connect to WebSocket server");
    assert_eq!(
        response.status(),
        101,
        "WebSocket handshake failed: {response:?}"
    );
    ws_stream
}

pub fn assert_raw_message_matches<F>(
    message: Option<Result<tungstenite::Message, tungstenite::Error>>,
    predicate: F,
) where
    F: FnOnce(SignalingMessage),
{
    match message {
        Some(Ok(tungstenite::Message::Text(raw_message))) => {
            match SignalingMessage::deserialize(&raw_message) {
                Ok(message) => predicate(message),
                Err(err) => panic!("Failed to deserialize message: {err:?}"),
            }
        }
        Some(Ok(_)) => panic!("Expected a text message, but got {message:?}"),
        Some(Err(err)) => panic!("Failed to receive message: {err:?}"),
        None => panic!("No message received"),
    }
}

pub fn assert_message_matches<F>(message: Option<SignalingMessage>, predicate: F)
where
    F: FnOnce(SignalingMessage),
{
    match message {
        Some(message) => predicate(message),
        None => panic!("No message received"),
    }
}
