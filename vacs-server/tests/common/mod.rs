use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::watch;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite, MaybeTlsStream, WebSocketStream};
use vacs_server::app::create_app;
use vacs_server::config::{AppConfig, AuthConfig};
use vacs_server::state::AppState;
use vacs_shared::signaling;

#[allow(unused)]
pub struct TestApp {
    pub state: Arc<AppState>,
    addr: String,
    shutdown_tx: watch::Sender<()>,
}

impl TestApp {
    #[allow(unused)]
    pub async fn new() -> Self {
        let config = AppConfig {
            auth: AuthConfig {
                login_flow_timeout_millis: 250,
            },
            ..Default::default()
        };
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let app_state = Arc::new(AppState::new(config, shutdown_rx));

        let app = create_app();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let app_state_clone = app_state.clone();
        tokio::spawn(async move {
            axum::serve(
                listener,
                app.with_state(app_state_clone)
                    .into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });

        TestApp {
            state: app_state,
            addr: format!("ws://{}/ws", addr),
            shutdown_tx,
        }
    }

    #[allow(unused)]
    pub fn addr(&self) -> &str {
        &self.addr
    }

    pub fn shutdown(&self) {
        self.shutdown_tx.send(()).unwrap();
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[allow(unused)]
pub struct TestClient {
    id: String,
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl TestClient {
    #[allow(unused)]
    /// Initializes and authenticates a client, asserting a successful server connection and login.
    pub async fn new<F>(
        addr: &str,
        id: &str,
        token: &str,
        client_list_predicate: F,
    ) -> anyhow::Result<Self>
    where
        F: FnOnce(&[signaling::ClientInfo]),
    {
        let mut ws_stream = connect_to_websocket(addr).await;

        let login_message = signaling::Message::Login {
            id: id.to_string(),
            token: token.to_string(),
        };
        ws_stream
            .send(tungstenite::Message::from(signaling::Message::serialize(
                &login_message,
            )?))
            .await?;

        if let Some(Ok(tungstenite::Message::Text(response_text))) = ws_stream.next().await {
            let response = signaling::Message::deserialize(&response_text)?;
            match response {
                signaling::Message::ClientList { clients } => client_list_predicate(&clients),
                signaling::Message::LoginFailure { reason } => {
                    return Err(anyhow::anyhow!("Login failed: {:?}", reason));
                }
                _ => return Err(anyhow::anyhow!("Unexpected response: {:?}", response)),
            }
        }

        Ok(Self {
            id: id.to_string(),
            ws_stream,
        })
    }

    #[allow(unused)]
    /// Sends a message through the WebSocket connection.
    pub async fn send(&mut self, message: signaling::Message) -> anyhow::Result<()> {
        self.ws_stream
            .send(tungstenite::Message::from(signaling::Message::serialize(
                &message,
            )?))
            .await?;
        Ok(())
    }

    #[allow(unused)]
    /// Waits for a single WebSocket message with a timeout, returning the deserialized message.
    pub async fn receive_with_timeout(
        &mut self,
        timeout_duration: Duration,
    ) -> Option<signaling::Message> {
        match timeout(timeout_duration, self.ws_stream.next()).await {
            Ok(Some(Ok(tungstenite::Message::Text(response_text)))) => {
                signaling::Message::deserialize(&response_text).ok()
            }
            _ => None,
        }
    }

    #[allow(unused)]
    /// Waits for multiple WebSocket messages until a timeout has been reached, returning all received messages.
    pub async fn receive_until_timeout(
        &mut self,
        timeout_duration: Duration,
    ) -> Vec<signaling::Message> {
        let mut messages = Vec::new();
        while let Some(message) = self.receive_with_timeout(timeout_duration).await {
            messages.push(message);
        }
        messages
    }

    #[allow(unused)]
    /// Sends a message and waits for an expected response using pattern matching.
    pub async fn send_and_expect<F>(
        &mut self,
        message: signaling::Message,
        verifier: F,
    ) -> anyhow::Result<()>
    where
        F: FnOnce(signaling::Message),
    {
        self.send(message).await?;
        let response = self.receive_with_timeout(Duration::from_secs(1)).await;
        match response {
            Some(msg) => verifier(msg),
            None => panic!("Expected a response, but none was received"),
        }
        Ok(())
    }

    #[allow(unused)]
    /// Cleanly closes the WebSocket connection.
    pub async fn close(&mut self) {
        self.ws_stream.close(None).await.unwrap();
    }
}

#[allow(unused)]
pub async fn connect_to_websocket(addr: &str) -> WebSocketStream<MaybeTlsStream<TcpStream>> {
    let (ws_stream, response) = connect_async(addr)
        .await
        .expect("Failed to connect to WebSocket server");
    assert_eq!(
        response.status(),
        axum::http::StatusCode::SWITCHING_PROTOCOLS,
        "WebSocket handshake failed"
    );
    ws_stream
}

#[allow(unused)]
pub async fn setup_test_clients(
    addr: &str,
    clients: &[(&str, &str)],
) -> HashMap<String, TestClient> {
    let mut test_clients = HashMap::new();
    for (id, token) in clients {
        let client = TestClient::new(addr, id, token, |clients| {
            assert!(
                clients.iter().any(|c| c.id == id.to_string()),
                "Client {} not found in client list",
                id
            );
        })
        .await
        .expect("Failed to create test client");
        test_clients.insert(client.id.clone(), client);
    }
    test_clients
}

#[allow(unused)]
pub fn init_test_tracing() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "trace".parse().unwrap()),
            )
            .with_test_writer()
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set global default tracing_subscriber");
    });
}

#[allow(unused)]
pub fn assert_raw_message_matches<F>(
    message: Option<Result<tungstenite::Message, tungstenite::Error>>,
    predicate: F,
) where
    F: FnOnce(signaling::Message),
{
    match message {
        Some(Ok(tungstenite::Message::Text(raw_message))) => {
            match signaling::Message::deserialize(&raw_message) {
                Ok(message) => predicate(message),
                Err(err) => panic!("Failed to deserialize message: {:?}", err),
            }
        }
        Some(Ok(_)) => panic!("Expected a text message, but got {:?}", message),
        Some(Err(err)) => panic!("Failed to receive message: {:?}", err),
        None => panic!("No message received"),
    }
}

#[allow(unused)]
pub fn assert_message_matches<F>(message: Option<signaling::Message>, predicate: F)
where
    F: FnOnce(signaling::Message),
{
    match message {
        Some(message) => predicate(message),
        None => panic!("No message received"),
    }
}
