use crate::config;
use crate::state::AppState;
use crate::ws::application_message::handle_application_message;
use crate::ws::message::{MessageResult, receive_message, send_message};
use crate::ws::traits::{WebSocketSink, WebSocketStream};
use axum::extract::ws;
use futures_util::SinkExt;
use std::ops::ControlFlow;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tracing::{Instrument, instrument};
use vacs_protocol::ws::{ClientInfo, SignalingMessage};

#[derive(Clone)]
pub struct ClientSession {
    client_info: ClientInfo,
    tx: mpsc::Sender<SignalingMessage>,
    client_shutdown_tx: watch::Sender<()>,
}

impl ClientSession {
    pub fn new(client_info: ClientInfo, tx: mpsc::Sender<SignalingMessage>) -> Self {
        let (client_shutdown_tx, _) = watch::channel(());
        Self {
            client_info,
            tx,
            client_shutdown_tx,
        }
    }

    pub fn get_id(&self) -> &str {
        &self.client_info.id
    }

    pub fn get_client_info(&self) -> &ClientInfo {
        &self.client_info
    }

    #[instrument(level = "debug", skip(self))]
    pub fn disconnect(&self) {
        tracing::trace!("Disconnecting client");
        let _ = self.client_shutdown_tx.send(());
    }

    #[instrument(level = "trace", skip(self), err)]
    pub async fn send_message(&self, message: SignalingMessage) -> anyhow::Result<()> {
        self.tx
            .send(message)
            .await
            .map_err(|err| anyhow::anyhow!(err).context("Failed to send message"))
    }

    #[allow(clippy::too_many_arguments)]
    #[instrument(level = "debug", skip_all, fields(client_info = ?client_info))]
    pub async fn handle_interaction<R: WebSocketStream + 'static, T: WebSocketSink + 'static>(
        &mut self,
        app_state: &Arc<AppState>,
        websocket_rx: R,
        websocket_tx: T,
        broadcast_rx: &mut broadcast::Receiver<SignalingMessage>,
        rx: &mut mpsc::Receiver<SignalingMessage>,
        app_shutdown_rx: &mut watch::Receiver<()>,
        client_info: ClientInfo,
    ) {
        tracing::debug!("Starting to handle client interaction");

        let (pong_update_tx, pong_update_rx) = watch::channel(Instant::now());

        let (writer_handle, ws_outbound_tx) = ClientSession::spawn_writer(
            websocket_tx,
            app_shutdown_rx.clone(),
            self.client_shutdown_tx.subscribe(),
        )
        .await;
        let (reader_handle, mut ws_inbound_rx) = ClientSession::spawn_reader(
            websocket_rx,
            app_shutdown_rx.clone(),
            self.client_shutdown_tx.subscribe(),
            pong_update_tx,
        )
        .await;
        let (ping_handle, mut ping_shutdown_rx) =
            ClientSession::spawn_ping_task(&ws_outbound_tx, pong_update_rx);

        tracing::trace!("Sending initial client info");
        if let Err(err) = send_message(
            &ws_outbound_tx,
            SignalingMessage::ClientInfo {
                own: true,
                info: client_info.clone(),
            },
        )
        .await
        {
            tracing::warn!(?err, "Failed to send initial client info");
        }

        tracing::trace!("Sending initial client list");
        let clients = app_state.list_clients_without_self(&client_info.id).await;
        if let Err(err) =
            send_message(&ws_outbound_tx, SignalingMessage::ClientList { clients }).await
        {
            tracing::warn!(?err, "Failed to send initial client info");
        }

        loop {
            tokio::select! {
                biased;

                _ = app_shutdown_rx.changed() => {
                    tracing::trace!("Shutdown signal received, disconnecting client");
                    break;
                }

                _ = &mut ping_shutdown_rx => {
                    tracing::debug!("Ping task reported client disconnect");
                    break;
                }

                msg = ws_inbound_rx.recv() => {
                    match msg {
                        Some(msg) => {
                            match handle_application_message(app_state, self, &ws_outbound_tx, msg).await {
                                ControlFlow::Continue(()) => continue,
                                ControlFlow::Break(()) => {
                                    tracing::debug!("Breaking interaction loop");
                                    break;
                                },
                            }
                        }
                        None => {
                            tracing::debug!("Application receiver closed, disconnecting client");
                            break;
                        }
                    }
                }

                msg = rx.recv() => {
                    match msg {
                        Some(msg) => {
                            tracing::trace!("Received direct message");
                            if let Err(err) = send_message(&ws_outbound_tx, msg).await {
                                tracing::warn!(?err, "Failed to send direct message");
                            }
                        }
                        None => {
                            tracing::debug!("Client receiver closed, disconnecting client");
                            break;
                        }
                    }
                }

                msg = broadcast_rx.recv() => {
                    match msg {
                        Ok(msg) => {
                            tracing::trace!("Received broadcast message");
                            if let Err(err) = send_message(&ws_outbound_tx, msg).await {
                                tracing::warn!(?err, "Failed to send broadcast message");
                            }
                        }
                        Err(err) => {
                            tracing::debug!(?err, "Broadcast receiver closed, disconnecting client");
                        }
                    }
                }
            }
        }

        writer_handle.abort();
        reader_handle.abort();
        ping_handle.abort();

        tracing::debug!("Finished handling client interaction");
    }

    #[instrument(level = "debug", skip_all)]
    pub async fn spawn_writer<T: WebSocketSink + 'static>(
        mut websocket_tx: T,
        mut app_shutdown_rx: watch::Receiver<()>,
        mut client_shutdown_rx: watch::Receiver<()>,
    ) -> (JoinHandle<()>, mpsc::Sender<ws::Message>) {
        let (ws_outbound_tx, mut ws_outbound_rx) =
            mpsc::channel::<ws::Message>(config::CLIENT_WEBSOCKET_TASK_CHANNEL_CAPACITY);

        let join_handle = tokio::spawn(async move {
            tracing::trace!("WebSocket writer task started");
            let _guard = TaskDropLogger::new("writer");

            loop {
                tokio::select! {
                    biased;

                    _ = app_shutdown_rx.changed() => {
                        tracing::trace!("App shutdown signal received, stopping WebSocket reader task");
                        break;
                    }

                    _ = client_shutdown_rx.changed() => {
                        tracing::trace!("Client shutdown signal received, stopping WebSocket reader task");
                        break;
                    }

                    msg = ws_outbound_rx.recv() => {
                        match msg {
                            Some(msg) => {
                                if let Err(err) = websocket_tx.send(msg).await {
                                    tracing::warn!(?err, "Failed to send message to client");
                                    break;
                                }
                            },
                            None => {
                                tracing::debug!("Outbound WebSocket channel closed, stopping WebSocket writer task");
                                break;
                            }
                        }
                    }
                }
            }

            tracing::trace!("Sending close message to client");
            if let Err(err) = websocket_tx.send(ws::Message::Close(None)).await {
                tracing::warn!(?err, "Failed to send close message to client");
            }

            tracing::trace!("WebSocket writer task finished");
        }.instrument(tracing::Span::current()));

        (join_handle, ws_outbound_tx)
    }

    #[instrument(level = "debug", skip_all)]
    pub async fn spawn_reader<R: WebSocketStream + 'static>(
        mut websocket_rx: R,
        mut app_shutdown_rx: watch::Receiver<()>,
        mut client_shutdown_rx: watch::Receiver<()>,
        pong_update_tx: watch::Sender<Instant>,
    ) -> (JoinHandle<()>, mpsc::Receiver<SignalingMessage>) {
        let (ws_inbound_tx, ws_inbound_rx) =
            mpsc::channel::<SignalingMessage>(config::CLIENT_WEBSOCKET_TASK_CHANNEL_CAPACITY);

        let join_handle = tokio::spawn(async move {
            tracing::trace!("WebSocket reader task started");
            let _guard = TaskDropLogger::new("reader");

            loop {
                tokio::select! {
                    biased;

                    _ = app_shutdown_rx.changed() => {
                        tracing::trace!("App shutdown signal received, stopping WebSocket reader task");
                        break;
                    }

                    _ = client_shutdown_rx.changed() => {
                        tracing::trace!("Client shutdown signal received, stopping WebSocket reader task");
                        break;
                    }

                    msg = receive_message(&mut websocket_rx) => {
                        match msg {
                            MessageResult::ApplicationMessage(message) => {
                                if let Err(err) = ws_inbound_tx.send(message).await {
                                    tracing::warn!(?err, "Failed to forward message to application");
                                    break;
                                }
                            }
                            MessageResult::ControlMessage => {
                                if let Err(err) = pong_update_tx.send(Instant::now()) {
                                    tracing::warn!(?err, "Failed to propagate last pong response, continuing");
                                    continue;
                                }
                            },
                            MessageResult::Disconnected => {
                                tracing::debug!("Client disconnected");
                                break;
                            }
                            MessageResult::Error(err) => {
                                tracing::warn!(?err, "Error while receiving message from client");
                                break;
                            }
                        }
                    }
                }
            }
            tracing::trace!("WebSocket reader task finished");
        }.instrument(tracing::Span::current()));

        (join_handle, ws_inbound_rx)
    }

    #[instrument(level = "debug", skip_all)]
    pub fn spawn_ping_task(
        ws_outbound_tx: &mpsc::Sender<ws::Message>,
        pong_update_rx: watch::Receiver<Instant>,
    ) -> (JoinHandle<()>, oneshot::Receiver<()>) {
        let (ping_shutdown_tx, ping_shutdown_rx) = oneshot::channel();

        let ws_outbound_tx = ws_outbound_tx.clone();
        let join_handle = tokio::spawn(
            async move {
                tracing::trace!("WebSocket ping task started");
                let _guard = TaskDropLogger::new("ping");

                let mut interval = tokio::time::interval(config::CLIENT_WEBSOCKET_PING_INTERVAL);
                loop {
                    interval.tick().await;

                    if Instant::now().duration_since(*pong_update_rx.borrow())
                        > config::CLIENT_WEBSOCKET_PONG_TIMEOUT
                    {
                        tracing::warn!("Pong timeout exceeded, disconnecting client");
                        let _ = ping_shutdown_tx.send(());
                        break;
                    }

                    if let Err(err) = ws_outbound_tx
                        .send(ws::Message::Ping(bytes::Bytes::new()))
                        .await
                    {
                        tracing::warn!(?err, "Failed to send ping to client");
                        let _ = ping_shutdown_tx.send(());
                        break;
                    }
                }
                tracing::trace!("WebSocket ping task finished");
            }
            .instrument(tracing::Span::current()),
        );

        (join_handle, ping_shutdown_rx)
    }
}

struct TaskDropLogger {
    name: &'static str,
}

impl TaskDropLogger {
    pub fn new(name: &'static str) -> Self {
        Self { name }
    }
}

impl Drop for TaskDropLogger {
    fn drop(&mut self) {
        tracing::trace!(task_name = ?self.name, "Task dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws::test_util::{TestSetup, create_client_info};
    use axum::extract::ws;
    use axum::extract::ws::Utf8Bytes;
    use pretty_assertions::assert_eq;
    use test_log::test;

    #[test(tokio::test)]
    async fn new_client_session() {
        let client_info_1 = create_client_info(1);
        let (tx, _rx) = mpsc::channel(10);
        let session = ClientSession::new(client_info_1.clone(), tx);

        assert_eq!(session.get_id(), "client1");
        assert_eq!(session.get_client_info(), &client_info_1);
    }

    #[test(tokio::test)]
    async fn send_message() {
        let client_info_1 = create_client_info(1);
        let (tx, mut rx) = mpsc::channel(10);
        let session = ClientSession::new(client_info_1, tx);

        let client_info_2 = create_client_info(2);
        let message = SignalingMessage::ClientList {
            clients: vec![client_info_2],
        };
        let result = session.send_message(message.clone()).await;

        assert!(result.is_ok());
        let received = rx.recv().await.expect("Expected message to be received");
        assert_eq!(received, message);
    }

    #[test(tokio::test)]
    async fn send_message_error() {
        let client_info_1 = create_client_info(1);
        let (tx, _) = mpsc::channel(10);
        let session = ClientSession::new(client_info_1, tx.clone());
        drop(tx); // Drop the sender to simulate the client disconnecting

        let client_info_2 = create_client_info(2);
        let message = SignalingMessage::ClientList {
            clients: vec![client_info_2],
        };
        let result = session.send_message(message.clone()).await;

        assert!(result.is_err_and(|err| err.to_string().contains("Failed to send message")));
    }

    #[test(tokio::test)]
    async fn initial_client_list_without_self() {
        let setup = TestSetup::new();
        let client_info_1 = create_client_info(1);
        setup.register_client(client_info_1.clone()).await;
        let websocket_rx = setup.websocket_rx.clone();

        let handle_task = setup.spawn_session_handle_interaction(client_info_1);

        let _ = websocket_rx.lock().await.recv().await; // skip client info message
        let message = websocket_rx.lock().await.recv().await;
        match message {
            Some(ws::Message::Text(text)) => {
                assert_eq!(
                    text,
                    Utf8Bytes::from_static(r#"{"type":"ClientList","clients":[]}"#)
                );
            }
            _ => panic!("Expected client list message"),
        }

        handle_task.await.unwrap();
    }

    #[test(tokio::test)]
    async fn initial_client_info() {
        let setup = TestSetup::new();
        let client_info_1 = create_client_info(1);
        setup.register_client(client_info_1.clone()).await;
        let websocket_rx = setup.websocket_rx.clone();

        let handle_task = setup.spawn_session_handle_interaction(client_info_1);

        let message = websocket_rx.lock().await.recv().await;
        match message {
            Some(ws::Message::Text(text)) => {
                assert_eq!(
                    text,
                    Utf8Bytes::from_static(
                        r#"{"type":"ClientInfo","own":true,"info":{"id":"client1","displayName":"Client 1","frequency":"100.000"}}"#
                    )
                );
            }
            _ => panic!("Expected client info message"),
        }

        handle_task.await.unwrap();
    }

    #[test(tokio::test)]
    async fn initial_client_list() {
        let setup = TestSetup::new();
        let client_info_1 = create_client_info(1);
        let client_info_2 = create_client_info(2);
        setup.register_client(client_info_1.clone()).await;
        setup.register_client(client_info_2.clone()).await;
        let websocket_rx = setup.websocket_rx.clone();

        let handle_task = setup.spawn_session_handle_interaction(client_info_2);

        let _ = websocket_rx.lock().await.recv().await; // skip client info message
        let message = websocket_rx.lock().await.recv().await;
        match message {
            Some(ws::Message::Text(text)) => {
                assert_eq!(
                    text,
                    Utf8Bytes::from_static(
                        r#"{"type":"ClientList","clients":[{"id":"client1","displayName":"Client 1","frequency":"100.000"}]}"#
                    )
                );
            }
            _ => panic!("Expected client list message"),
        }

        handle_task.await.unwrap();
    }

    #[test(tokio::test)]
    async fn handle_interaction() {
        let client_info_1 = create_client_info(1);
        let client_info_2 = create_client_info(2);
        let setup = TestSetup::new().with_messages(vec![Ok(ws::Message::Text(
            Utf8Bytes::from_static(r#"{"type":"CallOffer","peerId":"client2","sdp":"sdp1"}"#),
        ))]);
        let (_, mut client2_rx) = setup.register_client(client_info_2).await;
        let websocket_rx = setup.websocket_rx.clone();

        let handle_task = setup.spawn_session_handle_interaction(client_info_1);

        let _ = websocket_rx.lock().await.recv().await; // skip client info message
        let message = websocket_rx.lock().await.recv().await;
        match message {
            Some(ws::Message::Text(text)) => {
                assert_eq!(
                    text,
                    Utf8Bytes::from_static(
                        r#"{"type":"ClientList","clients":[{"id":"client2","displayName":"Client 2","frequency":"200.000"}]}"#
                    )
                );
            }
            _ => panic!("Expected client list message"),
        }

        let call_offer = client2_rx.recv().await.unwrap();
        assert_eq!(
            call_offer,
            SignalingMessage::CallOffer {
                peer_id: "client1".to_string(),
                sdp: "sdp1".to_string()
            }
        );

        handle_task.await.unwrap();
    }

    #[test(tokio::test)]
    async fn handle_interaction_websocket_error() {
        let client_info_1 = create_client_info(1);
        let setup = TestSetup::new().with_messages(vec![Err(axum::Error::new("Test error"))]);
        let websocket_rx = setup.websocket_rx.clone();

        let handle_task = setup.spawn_session_handle_interaction(client_info_1);

        let _ = websocket_rx.lock().await.recv().await; // skip client info message
        let message = websocket_rx.lock().await.recv().await;
        match message {
            Some(ws::Message::Text(text)) => {
                assert_eq!(
                    text,
                    Utf8Bytes::from_static(r#"{"type":"ClientList","clients":[]}"#)
                );
            }
            _ => panic!("Expected client list message"),
        }

        assert!(websocket_rx.lock().await.is_closed());

        handle_task.await.unwrap();
    }
}
