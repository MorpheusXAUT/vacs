use crate::config::AppConfig;
use crate::state::AppState;
use crate::store::Store;
use crate::store::memory::MemoryStore;
use crate::ws::ClientSession;
use axum::extract::ws;
use futures_util::{Sink, Stream};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::{Mutex, broadcast, mpsc, watch};
use vacs_protocol::ws::{ClientInfo, SignalingMessage};

pub struct MockSink {
    tx: mpsc::Sender<ws::Message>,
}

impl MockSink {
    pub fn new(tx: mpsc::Sender<ws::Message>) -> Self {
        Self { tx }
    }
}

impl Sink<ws::Message> for MockSink {
    type Error = axum::Error;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: ws::Message) -> Result<(), Self::Error> {
        self.tx.try_send(item).map_err(axum::Error::new)
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

pub struct MockStream {
    messages: Vec<Result<ws::Message, axum::Error>>,
}

impl MockStream {
    pub fn new(messages: Vec<Result<ws::Message, axum::Error>>) -> Self {
        Self { messages }
    }
}

impl Stream for MockStream {
    type Item = Result<ws::Message, axum::Error>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.messages.is_empty() {
            Poll::Ready(None)
        } else {
            Poll::Ready(Some(self.messages.remove(0)))
        }
    }
}

pub struct TestSetup {
    pub app_state: Arc<AppState>,
    pub session: ClientSession,
    pub mock_stream: MockStream,
    pub mock_sink: MockSink,
    pub websocket_tx: Arc<Mutex<mpsc::Sender<ws::Message>>>,
    pub websocket_rx: Arc<Mutex<mpsc::Receiver<ws::Message>>>,
    pub rx: mpsc::Receiver<SignalingMessage>,
    pub broadcast_rx: broadcast::Receiver<SignalingMessage>,
    pub shutdown_tx: watch::Sender<()>,
}

impl TestSetup {
    pub fn new() -> Self {
        let mut vatsim_users = HashMap::new();
        for i in 0..=5 {
            vatsim_users.insert(format!("token{i}"), format!("client{i}"));
        }
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let app_state = Arc::new(AppState::new(
            AppConfig::default(),
            Store::Memory(MemoryStore::default()),
            shutdown_rx,
        ));
        let client_info = ClientInfo {
            id: "client1".to_string(),
            display_name: "Client 1".to_string(),
        };
        let (tx, rx) = mpsc::channel(10);
        let session = ClientSession::new(client_info, tx);
        let (websocket_tx, websocket_rx) = mpsc::channel(100);
        let mock_stream = MockStream::new(vec![]);
        let mock_sink = MockSink::new(websocket_tx.clone());
        let (_broadcast_tx, broadcast_rx) = broadcast::channel(10);

        Self {
            app_state,
            session,
            mock_stream,
            mock_sink,
            websocket_tx: Arc::new(Mutex::new(websocket_tx)),
            websocket_rx: Arc::new(Mutex::new(websocket_rx)),
            rx,
            broadcast_rx,
            shutdown_tx,
        }
    }

    pub fn with_messages(mut self, messages: Vec<Result<ws::Message, axum::Error>>) -> Self {
        self.mock_stream = MockStream::new(messages);
        self
    }

    pub async fn register_client(
        &self,
        client_id: &str,
    ) -> (ClientSession, mpsc::Receiver<SignalingMessage>) {
        self.app_state
            .register_client(client_id)
            .await
            .expect("Failed to register client")
    }

    pub async fn register_clients(
        &self,
        client_ids: Vec<&str>,
    ) -> HashMap<String, (ClientSession, mpsc::Receiver<SignalingMessage>)> {
        futures_util::future::join_all(client_ids.iter().map(|&client_id| async move {
            (client_id.to_string(), self.register_client(client_id).await)
        }))
        .await
        .into_iter()
        .collect()
    }

    pub async fn take_last_websocket_message(&mut self) -> Option<ws::Message> {
        self.websocket_rx.lock().await.recv().await
    }

    pub fn spawn_session_handle_interaction(
        mut self,
        client_id: String,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.session
                .handle_interaction(
                    &self.app_state,
                    self.mock_stream,
                    self.mock_sink,
                    &mut self.broadcast_rx,
                    &mut self.rx,
                    &mut self.shutdown_tx.subscribe(),
                    client_id.as_str(),
                )
                .await
        })
    }
}
