use crate::error::SignalingError;
use crate::matcher::ResponseMatcher;
use crate::transport::{SignalingReceiver, SignalingSender};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::{Mutex, broadcast, mpsc, oneshot, watch};
use tokio::task::JoinSet;
use tokio_tungstenite::tungstenite;
use tracing::{Instrument, instrument};
use vacs_protocol::ws::{ClientInfo, SignalingMessage};

const BROADCAST_CHANNEL_SIZE: usize = 100;
const SEND_CHANNEL_SIZE: usize = 100;

#[derive(Clone)]
pub struct SignalingClient {
    matcher: ResponseMatcher,
    broadcast_tx: broadcast::Sender<SignalingMessage>,
    send_tx: Arc<Mutex<Option<mpsc::Sender<tungstenite::Message>>>>,
    shutdown_rx: watch::Receiver<()>,
    disconnect_tx: watch::Sender<()>,
    is_connected: Arc<AtomicBool>,
    is_logged_in: Arc<AtomicBool>,
}

impl SignalingClient {
    #[instrument(level = "debug", skip_all)]
    pub fn new(shutdown_rx: watch::Receiver<()>) -> Self {
        Self {
            matcher: ResponseMatcher::new(),
            broadcast_tx: broadcast::channel(BROADCAST_CHANNEL_SIZE).0,
            send_tx: Arc::new(Mutex::new(None)),
            shutdown_rx,
            disconnect_tx: watch::channel(()).0,
            is_connected: Arc::new(AtomicBool::new(false)),
            is_logged_in: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn matcher(&self) -> &ResponseMatcher {
        &self.matcher
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SignalingMessage> {
        self.broadcast_tx.subscribe()
    }

    pub fn status(&self) -> (bool, bool) {
        (
            self.is_connected.load(Ordering::SeqCst),
            self.is_logged_in.load(Ordering::SeqCst),
        )
    }

    #[instrument(level = "info", skip(self))]
    pub fn disconnect(&mut self) {
        tracing::debug!("Disconnecting signaling client");
        let _ = self.disconnect_tx.send(());
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn send(&self, msg: SignalingMessage) -> Result<(), SignalingError> {
        let send_tx_guard = self.send_tx.lock().await;
        let send_tx = send_tx_guard.as_ref().ok_or_else(|| {
            tracing::warn!("Tried to send message before signaling client was started");
            SignalingError::Disconnected
        })?;

        if !self.is_logged_in.load(Ordering::SeqCst)
            && !matches!(msg, SignalingMessage::Login { .. })
        {
            tracing::warn!("Tried to send message before login");
            return Err(SignalingError::ProtocolError("Not logged in".to_string()));
        }

        tracing::debug!("Sending message to send channel");

        let serialized = SignalingMessage::serialize(&msg).map_err(|err| {
            tracing::warn!(?err, "Failed to serialize message");
            SignalingError::SerializationError(err)
        })?;

        send_tx
            .send(tungstenite::Message::from(serialized))
            .await
            .map_err(|err| SignalingError::Transport(anyhow::anyhow!(err)))
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn recv(&mut self) -> Result<SignalingMessage, SignalingError> {
        tracing::debug!("Waiting for message from server");
        self.recv_with_timeout(Duration::MAX).await
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn recv_with_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<SignalingMessage, SignalingError> {
        tracing::debug!("Waiting for message from server with timeout");
        let mut broadcast_rx = self.subscribe();
        let rx_len = broadcast_rx.is_empty();

        // if rx_len && !self.is_connected.load(Ordering::SeqCst) {
        //     tracing::warn!("Client is not connected and there are no remaining messages, aborting receive");
        //     return Err(SignalingError::Disconnected);
        // }

        let recv_result = tokio::select! {
            biased;
            _ = self.shutdown_rx.changed() => {
                tracing::debug!("Shutdown signal received, aborting receive");
                return Err(SignalingError::Timeout("Shutdown signal received".to_string()))
            }
            res = tokio::time::timeout(timeout, broadcast_rx.recv()) => res,
        };

        match recv_result {
            Ok(Ok(msg)) => Ok(msg),
            Ok(Err(err)) => Err(SignalingError::Transport(anyhow::anyhow!(err))),
            Err(_) => {
                tracing::warn!("Timeout waiting for message");
                Err(SignalingError::Timeout(
                    "Timeout waiting for message".to_string(),
                ))
            }
        }
    }

    #[instrument(level = "info", skip(self, token))]
    pub async fn login(
        &mut self,
        token: &str,
        timeout: Duration,
    ) -> Result<Vec<ClientInfo>, SignalingError> {
        tracing::debug!("Sending Login message to server");
        self.send(SignalingMessage::Login {
            token: token.to_string(),
        })
        .await?;

        tracing::debug!("Awaiting authentication response from server");
        match self.recv_with_timeout(timeout).await? {
            SignalingMessage::ClientList { clients } => {
                tracing::info!(num_clients = ?clients.len(), "Login successful, received client list");
                self.is_logged_in.store(true, Ordering::SeqCst);
                Ok(clients)
            }
            SignalingMessage::LoginFailure { reason } => {
                tracing::warn!(?reason, "Login failed");
                self.is_logged_in.store(false, Ordering::SeqCst);
                Err(SignalingError::LoginError(reason))
            }
            SignalingMessage::Error { reason, peer_id } => {
                tracing::error!(?reason, ?peer_id, "Server returned error");
                self.is_logged_in.store(false, Ordering::SeqCst);
                Err(SignalingError::ServerError(reason))
            }
            other => {
                tracing::error!(?other, "Received unexpected message from server");
                self.is_logged_in.store(false, Ordering::SeqCst);
                Err(SignalingError::ProtocolError(
                    "Expected ClientList after Login".to_string(),
                ))
            }
        }
    }

    #[instrument(level = "info", skip(self))]
    pub fn logout(&mut self) -> Result<(), SignalingError> {
        tracing::debug!("Sending Logout message to server");
        self.disconnect();
        Ok(())
    }

    #[instrument(level = "debug", skip_all)]
    pub async fn start<S: SignalingSender + 'static, R: SignalingReceiver + 'static>(
        &mut self,
        sender: S,
        receiver: R,
        ready_tx: oneshot::Sender<()>,
    ) -> InterruptionReason {
        let (send_tx, send_rx) = mpsc::channel::<tungstenite::Message>(SEND_CHANNEL_SIZE);
        let send_tx_clone = send_tx.clone();

        let mut tasks = JoinSet::new();

        let matcher = self.matcher.clone();
        let broadcast_tx = self.broadcast_tx.clone();
        let shutdown_rx = self.shutdown_rx.clone();
        let disconnect_rx = self.disconnect_tx.subscribe();

        tasks.spawn(Self::reader_task(
            receiver,
            send_tx_clone,
            matcher,
            broadcast_tx,
            shutdown_rx,
            disconnect_rx,
        ));

        let shutdown_rx = self.shutdown_rx.clone();
        let disconnect_rx = self.disconnect_tx.subscribe();
        tasks.spawn(Self::writer_task(
            sender,
            send_rx,
            shutdown_rx,
            disconnect_rx,
        ));

        tracing::trace!("Transport tasks started, handling interaction");
        *self.send_tx.lock().await = Some(send_tx);
        self.is_connected.store(true, Ordering::SeqCst);
        let _ = ready_tx.send(());

        let reason = match tasks.join_next().await {
            Some(Ok(reason)) => reason,
            Some(Err(err)) => {
                tracing::error!(?err, "Task panicked or failed to join");
                InterruptionReason::Error(SignalingError::Transport(anyhow::anyhow!(err)))
            }
            None => {
                tracing::warn!("All tasks completed unexpectedly");
                InterruptionReason::Disconnected
            }
        };

        tracing::debug!(
            ?reason,
            "Transport task completed, aborting remaining tasks"
        );
        tasks.abort_all();

        tracing::debug!("Cleaning up after transport tasks");
        *self.send_tx.lock().await = None;
        self.is_connected.store(false, Ordering::SeqCst);
        self.is_logged_in.store(false, Ordering::SeqCst);

        reason
    }

    #[instrument(level = "debug", skip_all)]
    fn reader_task<R: SignalingReceiver + 'static>(
        mut receiver: R,
        send_tx: mpsc::Sender<tungstenite::Message>,
        matcher: ResponseMatcher,
        broadcast_tx: broadcast::Sender<SignalingMessage>,
        mut shutdown_rx: watch::Receiver<()>,
        mut disconnect_rx: watch::Receiver<()>,
    ) -> impl Future<Output = InterruptionReason> + Send {
        async move {
            tracing::debug!("Starting transport reader task");

            loop {
                tokio::select! {
                    biased;

                    _ = shutdown_rx.changed() => {
                        tracing::debug!("Shutdown signal received, exiting transport reader task");
                        return InterruptionReason::ShutdownSignal;
                    }

                    _ = disconnect_rx.changed() => {
                        tracing::debug!("Disconnect signal received, exiting transport reader task");
                        return InterruptionReason::Disconnected;
                    }

                    msg = receiver.recv(&send_tx) => {
                        match msg {
                            Ok(message) => {
                                tracing::trace!(?message, "Received message from transport, trying to match against matcher");
                                matcher.try_match(&message);
                                if broadcast_tx.receiver_count() > 0 {
                                    tracing::trace!(?message, "Broadcasting message");
                                    if let Err(err) = broadcast_tx.send(message.clone()) {
                                        tracing::warn!(?message, ?err, "Failed to broadcast message");
                                    }
                                } else {
                                    tracing::trace!(?message, "No receivers subscribed, not broadcasting message");
                                }
                            }
                            Err(err) => {
                                return match err {
                                    SignalingError::Disconnected => {
                                        tracing::debug!("Transport disconnected, aborting interaction handling");
                                        InterruptionReason::Disconnected
                                    }
                                    err => {
                                        tracing::warn!(?err, "Received error from transport, continuing");
                                        InterruptionReason::Error(err)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }.instrument(tracing::Span::current())
    }

    #[instrument(level = "debug", skip_all)]
    fn writer_task<S: SignalingSender + 'static>(
        mut sender: S,
        mut send_rx: mpsc::Receiver<tungstenite::Message>,
        mut shutdown_rx: watch::Receiver<()>,
        mut disconnect_rx: watch::Receiver<()>,
    ) -> impl Future<Output = InterruptionReason> + Send {
        async move {
            tracing::debug!("Starting transport writer task");

            loop {
                tokio::select! {
                    biased;

                    _ = shutdown_rx.changed() => {
                        tracing::debug!("Shutdown signal received, exiting transport writer task");
                        return InterruptionReason::ShutdownSignal;
                    }

                    _ = disconnect_rx.changed() => {
                        tracing::debug!("Disconnect signal received, logging out");

                        let serialized= match SignalingMessage::serialize(&SignalingMessage::Logout) {
                            Ok(serialized) => serialized,
                            Err(err) => {
                                tracing::warn!(?err, "Failed to serialize Logout message");
                                return InterruptionReason::Error(SignalingError::SerializationError(err));
                            }
                        };

                        tracing::trace!("Sending Logout message to server");
                        if let Err(err) = sender.send(tungstenite::Message::from(serialized)).await {
                            tracing::warn!(?err, "Failed to send Logout message, closing sender anyways");
                        } else {
                            tracing::debug!("Successfully logged out, closing sender");
                        }

                        if let Err(err) = sender.close().await {
                            return InterruptionReason::Error(err);
                        }

                        tracing::debug!("Successfully disconnected, exiting transport writer task");
                        return InterruptionReason::Disconnected;
                    }

                    msg = send_rx.recv() => {
                        match msg {
                            Some(msg) => {
                                if !matches!(msg, tungstenite::Message::Pong(_)) {
                                    tracing::debug!(?msg, "Sending message to transport");
                                }
                                let result = tokio::select! {
                                    biased;
                                    _ = shutdown_rx.changed() => {
                                        tracing::debug!("Shutdown signal received, aborting send");
                                        Err(SignalingError::Timeout("Shutdown signal received".to_string()))
                                    }
                                    result = sender.send(msg) => result,
                                };

                                if let Err(err) = result {
                                    return InterruptionReason::Error(err);
                                }
                            },
                            None => {
                                tracing::debug!("Send channel closed, exiting transport send task");
                                return InterruptionReason::Disconnected;
                            }
                        }
                    }
                }
            }
        }.instrument(tracing::Span::current())
    }
}

#[derive(Debug)]
pub enum InterruptionReason {
    ShutdownSignal,
    Disconnected,
    Error(SignalingError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::mock;
    use pretty_assertions::assert_matches;
    use test_log::test;
    use tokio::sync::watch;
    use vacs_protocol::ws::{ErrorReason, LoginFailureReason};

    fn test_client_list() -> Vec<ClientInfo> {
        vec![
            ClientInfo {
                id: "client1".to_string(),
                display_name: "Client 1".to_string(),
            },
            ClientInfo {
                id: "client2".to_string(),
                display_name: "Client 2".to_string(),
            },
            ClientInfo {
                id: "client3".to_string(),
                display_name: "Client 3".to_string(),
            },
        ]
    }

    #[test(tokio::test)]
    async fn start() {
        let ((sender, receiver), _handle) = mock::create();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let (ready_tx, ready_rx) = oneshot::channel();
        let client = SignalingClient::new(shutdown_rx);
        let mut client_clone = client.clone();

        tokio::spawn(async move {
            client_clone.start(sender, receiver, ready_tx).await;
        });

        assert!(ready_rx.await.is_ok());
        assert_matches!(client.status(), (true, false));
    }

    #[test(tokio::test)]
    async fn start_shutdown() {
        let ((sender, receiver), _handle) = mock::create();
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let (ready_tx, ready_rx) = oneshot::channel();
        let client = SignalingClient::new(shutdown_rx);
        let mut client_clone = client.clone();

        let task = tokio::spawn(async move {
            return client_clone.start(sender, receiver, ready_tx).await;
        });
        assert!(ready_rx.await.is_ok());

        shutdown_tx.send(()).unwrap();

        assert_matches!(task.await, Ok(InterruptionReason::ShutdownSignal));
        assert_matches!(client.status(), (false, false));
    }

    #[test(tokio::test)]
    async fn disconnect() {
        let ((sender, receiver), _handle) = mock::create();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let (ready_tx, ready_rx) = oneshot::channel();
        let mut client = SignalingClient::new(shutdown_rx);
        let mut client_clone = client.clone();

        let task = tokio::spawn(async move {
            return client_clone.start(sender, receiver, ready_tx).await;
        });
        assert!(ready_rx.await.is_ok());

        client.disconnect();

        assert_matches!(task.await, Ok(InterruptionReason::Disconnected));
        assert_matches!(client.status(), (false, false));
    }

    #[test(tokio::test)]
    async fn send() {
        let ((sender, receiver), mut handle) = mock::create();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let (ready_tx, ready_rx) = oneshot::channel();
        let client = SignalingClient::new(shutdown_rx);
        let mut client_clone = client.clone();

        tokio::spawn(async move {
            client_clone.start(sender, receiver, ready_tx).await;
        });
        assert!(ready_rx.await.is_ok());

        let msg = SignalingMessage::Login {
            token: "test".to_string(),
        };

        let result = client.send(msg.clone()).await;
        assert!(result.is_ok());

        let sent_msg = handle.outgoing_rx.recv().await.unwrap();
        assert_eq!(
            sent_msg,
            tungstenite::Message::from(SignalingMessage::serialize(&msg).unwrap())
        );
    }

    #[test(tokio::test)]
    async fn send_without_start() {
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let client = SignalingClient::new(shutdown_rx);

        let msg = SignalingMessage::Login {
            token: "test".to_string(),
        };

        let result = client.send(msg.clone()).await;
        assert_matches!(result, Err(SignalingError::Disconnected));
    }

    #[test(tokio::test)]
    async fn send_without_login() {
        let ((sender, receiver), _handle) = mock::create();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let (ready_tx, ready_rx) = oneshot::channel();
        let client = SignalingClient::new(shutdown_rx);
        let mut client_clone = client.clone();

        tokio::spawn(async move {
            client_clone.start(sender, receiver, ready_tx).await;
        });
        assert!(ready_rx.await.is_ok());

        let msg = SignalingMessage::CallOffer {
            peer_id: "client1".to_string(),
            sdp: "".to_string(),
        };

        let result = client.send(msg.clone()).await;
        assert_matches!(result, Err(SignalingError::ProtocolError { .. }));
    }

    #[test(tokio::test)]
    async fn send_disconnected() {
        let ((sender, receiver), _handle) = mock::create();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let (ready_tx, ready_rx) = oneshot::channel();
        let mut client = SignalingClient::new(shutdown_rx);
        let mut client_clone = client.clone();

        tokio::spawn(async move {
            client_clone.start(sender, receiver, ready_tx).await;
        });
        assert!(ready_rx.await.is_ok());
        assert_matches!(client.status(), (true, false));

        client.disconnect();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_matches!(client.status(), (false, false));

        let msg = SignalingMessage::Login {
            token: "test".to_string(),
        };

        let result = client.send(msg.clone()).await;
        assert_matches!(result, Err(SignalingError::Disconnected));
    }

    #[test(tokio::test)]
    async fn send_shutdown() {
        let ((sender, receiver), _handle) = mock::create();
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let (ready_tx, ready_rx) = oneshot::channel();
        let client = SignalingClient::new(shutdown_rx);
        let mut client_clone = client.clone();

        tokio::spawn(async move {
            client_clone.start(sender, receiver, ready_tx).await;
        });
        assert!(ready_rx.await.is_ok());
        assert_matches!(client.status(), (true, false));

        shutdown_tx.send(()).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_matches!(client.status(), (false, false));

        let msg = SignalingMessage::Login {
            token: "test".to_string(),
        };

        let result = client.send(msg.clone()).await;
        assert_matches!(result, Err(SignalingError::Disconnected));
    }

    #[test(tokio::test)]
    async fn recv() {
        let ((sender, receiver), handle) = mock::create();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let (ready_tx, ready_rx) = oneshot::channel();
        let mut client = SignalingClient::new(shutdown_rx);
        let mut client_clone = client.clone();

        tokio::spawn(async move {
            client_clone.start(sender, receiver, ready_tx).await;
        });
        assert!(ready_rx.await.is_ok());

        let msg = SignalingMessage::ClientList {
            clients: test_client_list(),
        };

        let result = handle
            .incoming_tx
            .send(tungstenite::Message::from(
                SignalingMessage::serialize(&msg).unwrap(),
            ))
            .await;
        assert!(result.is_ok());

        let recv_result = client.recv().await;
        assert!(recv_result.is_ok());
        assert_eq!(recv_result.unwrap(), msg);
    }

    #[test(tokio::test)]
    async fn recv_shutdown_with_message_remaining() {
        let ((sender, receiver), handle) = mock::create();
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let (ready_tx, ready_rx) = oneshot::channel();
        let mut client = SignalingClient::new(shutdown_rx);
        let mut client_clone = client.clone();

        tokio::spawn(async move {
            client_clone.start(sender, receiver, ready_tx).await;
        });
        assert!(ready_rx.await.is_ok());

        let msg = SignalingMessage::ClientList {
            clients: test_client_list(),
        };

        let result = handle
            .incoming_tx
            .send(tungstenite::Message::from(
                SignalingMessage::serialize(&msg).unwrap(),
            ))
            .await;
        assert!(result.is_ok());

        shutdown_tx.send(()).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_matches!(client.status(), (false, false));

        let recv_result = client.recv().await;
        assert!(recv_result.is_err());
        assert_eq!(
            recv_result.unwrap_err().to_string(),
            "timeout: Shutdown signal received".to_string()
        );

        let recv_result = client.recv().await;
        assert_matches!(recv_result, Err(SignalingError::Disconnected));
    }

    // #[test(tokio::test)]
    // async fn recv() {
    //     let (mock, handle) = MockTransport::new();
    //     let (_shutdown_tx, shutdown_rx) = watch::channel(());
    //     let mut client = SignalingClient::new(mock, shutdown_rx);
    //     let msg = SignalingMessage::ClientList {
    //         clients: test_client_list(),
    //     };
    //
    //     let result = handle.incoming_tx.send(msg.clone()).await;
    //     assert!(result.is_ok());
    //
    //     let recv_result = client.recv().await;
    //     assert!(recv_result.is_ok());
    //     assert_eq!(recv_result.unwrap(), msg);
    // }
    //
    // #[test(tokio::test)]
    // async fn recv_shutdown() {
    //     let (mock, handle) = MockTransport::new();
    //     let (shutdown_tx, shutdown_rx) = watch::channel(());
    //     let mut client = SignalingClient::new(mock, shutdown_rx);
    //     let msg = SignalingMessage::Login {
    //         token: "test".to_string(),
    //     };
    //
    //     let result = handle.incoming_tx.send(msg.clone()).await;
    //     assert!(result.is_ok());
    //
    //     shutdown_tx.send(()).unwrap();
    //     let recv_result = client.recv().await;
    //     assert!(recv_result.is_err());
    //     assert_eq!(
    //         recv_result.unwrap_err().to_string(),
    //         "timeout: Shutdown signal received".to_string()
    //     );
    // }
    //
    // #[test(tokio::test)]
    // async fn recv_with_timeout() {
    //     let (mock, handle) = MockTransport::new();
    //     let (_shutdown_tx, shutdown_rx) = watch::channel(());
    //     let mut client = SignalingClient::new(mock, shutdown_rx);
    //     let msg = SignalingMessage::CallAnswer {
    //         peer_id: "client1".to_string(),
    //         sdp: "sdp".to_string(),
    //     };
    //
    //     let result = handle.incoming_tx.send(msg.clone()).await;
    //     assert!(result.is_ok());
    //
    //     let recv_result = client.recv_with_timeout(Duration::from_millis(100)).await;
    //     assert!(recv_result.is_ok());
    //     assert_eq!(recv_result.unwrap(), msg);
    // }
    //
    // #[test(tokio::test)]
    // async fn recv_with_timeout_expired() {
    //     let (mock, _handle) = MockTransport::new();
    //     let (_shutdown_tx, shutdown_rx) = watch::channel(());
    //     let mut client = SignalingClient::new(mock, shutdown_rx);
    //
    //     let recv_result = client.recv_with_timeout(Duration::from_millis(100)).await;
    //     assert!(recv_result.is_err());
    //     assert_eq!(
    //         recv_result.unwrap_err().to_string(),
    //         "timeout: Timeout waiting for message".to_string()
    //     );
    // }
    //
    // #[test(tokio::test)]
    // async fn recv_with_timeout_shutdown() {
    //     let (mock, handle) = MockTransport::new();
    //     let (shutdown_tx, shutdown_rx) = watch::channel(());
    //     let mut client = SignalingClient::new(mock, shutdown_rx);
    //     let msg = SignalingMessage::Login {
    //         token: "test".to_string(),
    //     };
    //
    //     let result = handle.incoming_tx.send(msg.clone()).await;
    //     assert!(result.is_ok());
    //
    //     shutdown_tx.send(()).unwrap();
    //     let recv_result = client.recv_with_timeout(Duration::from_millis(100)).await;
    //     assert!(recv_result.is_err());
    //     assert_eq!(
    //         recv_result.unwrap_err().to_string(),
    //         "timeout: Shutdown signal received".to_string()
    //     );
    // }
    //
    // #[test(tokio::test)]
    // async fn recv_server_error() {
    //     let (mock, handle) = MockTransport::new();
    //     let (_shutdown_tx, shutdown_rx) = watch::channel(());
    //     let mut client = SignalingClient::new(mock, shutdown_rx);
    //     let msg = SignalingMessage::Error {
    //         reason: ErrorReason::Internal("something failed".to_string()),
    //         peer_id: None,
    //     };
    //
    //     let result = handle.incoming_tx.send(msg.clone()).await;
    //     assert!(result.is_ok());
    //
    //     let recv_result = client.recv().await;
    //     assert!(recv_result.is_ok());
    //     assert_eq!(recv_result.unwrap(), msg);
    // }
    //
    // #[test(tokio::test)]
    // async fn recv_peer_connection_error() {
    //     let (mock, handle) = MockTransport::new();
    //     let (_shutdown_tx, shutdown_rx) = watch::channel(());
    //     let mut client = SignalingClient::new(mock, shutdown_rx);
    //     let msg = SignalingMessage::Error {
    //         reason: ErrorReason::PeerConnection,
    //         peer_id: Some("client1".to_string()),
    //     };
    //
    //     let result = handle.incoming_tx.send(msg.clone()).await;
    //     assert!(result.is_ok());
    //
    //     let recv_result = client.recv().await;
    //     assert!(recv_result.is_ok());
    //     assert_eq!(recv_result.unwrap(), msg);
    // }
    //
    // #[test(tokio::test)]
    // async fn recv_disconnected() {
    //     let (mock, handle) = MockTransport::new();
    //     let (_shutdown_tx, shutdown_rx) = watch::channel(());
    //     let mut client = SignalingClient::new(mock, shutdown_rx);
    //
    //     drop(handle.incoming_tx); // Simulate the incoming channel being closed
    //
    //     let recv_result = client.recv().await;
    //     assert!(recv_result.is_err());
    //     assert_matches!(recv_result, Err(SignalingError::Disconnected));
    // }
    //
    // #[test(tokio::test)]
    // async fn login() {
    //     let (mock, mut handle) = MockTransport::new();
    //     let (_shutdown_tx, shutdown_rx) = watch::channel(());
    //     let mut client = SignalingClient::new(mock, shutdown_rx);
    //     let test_clients = test_client_list();
    //     let msg = SignalingMessage::ClientList {
    //         clients: test_clients.clone(),
    //     };
    //
    //     let result = handle.incoming_tx.send(msg.clone()).await;
    //     assert!(result.is_ok());
    //
    //     let login_result = client.login("token1").await;
    //     assert!(login_result.is_ok());
    //     assert_eq!(login_result.unwrap(), test_clients);
    //
    //     let sent_message = handle.outgoing_rx.recv().await;
    //     assert_matches!(sent_message, Some(SignalingMessage::Login { ref token }) if token == "token1");
    // }
    //
    // #[test(tokio::test)]
    // async fn login_timeout() {
    //     let (mock, mut handle) = MockTransport::new();
    //     let (_shutdown_tx, shutdown_rx) = watch::channel(());
    //     let mut client = SignalingClientBuilder::new(mock, shutdown_rx)
    //         .with_login_timeout(Duration::from_millis(100))
    //         .build();
    //
    //     let login_result = client.login("token1").await;
    //     assert!(login_result.is_err());
    //     assert_matches!(login_result, Err(SignalingError::Timeout(_)));
    //
    //     let sent_message = handle.outgoing_rx.recv().await;
    //     assert_matches!(sent_message, Some(SignalingMessage::Login { ref token }) if token == "token1");
    // }
    //
    // #[test(tokio::test)]
    // async fn login_unauthorized() {
    //     let (mock, mut handle) = MockTransport::new();
    //     let (_shutdown_tx, shutdown_rx) = watch::channel(());
    //     let mut client = SignalingClient::new(mock, shutdown_rx);
    //     let msg = SignalingMessage::LoginFailure {
    //         reason: LoginFailureReason::Unauthorized,
    //     };
    //
    //     let result = handle.incoming_tx.send(msg.clone()).await;
    //     assert!(result.is_ok());
    //
    //     let login_result = client.login("token1").await;
    //     assert!(login_result.is_err());
    //     assert_matches!(
    //         login_result,
    //         Err(SignalingError::LoginError(LoginFailureReason::Unauthorized))
    //     );
    //
    //     let sent_message = handle.outgoing_rx.recv().await;
    //     assert_matches!(sent_message, Some(SignalingMessage::Login { ref token }) if token == "token1");
    // }
    //
    // #[test(tokio::test)]
    // async fn login_invalid_credentials() {
    //     let (mock, mut handle) = MockTransport::new();
    //     let (_shutdown_tx, shutdown_rx) = watch::channel(());
    //     let mut client = SignalingClient::new(mock, shutdown_rx);
    //     let msg = SignalingMessage::LoginFailure {
    //         reason: LoginFailureReason::InvalidCredentials,
    //     };
    //
    //     let result = handle.incoming_tx.send(msg.clone()).await;
    //     assert!(result.is_ok());
    //
    //     let login_result = client.login("token1").await;
    //     assert!(login_result.is_err());
    //     assert_matches!(
    //         login_result,
    //         Err(SignalingError::LoginError(
    //             LoginFailureReason::InvalidCredentials
    //         ))
    //     );
    //
    //     let sent_message = handle.outgoing_rx.recv().await;
    //     assert_matches!(sent_message, Some(SignalingMessage::Login { ref token }) if token == "token1");
    // }
    //
    // #[test(tokio::test)]
    // async fn login_duplicate_id() {
    //     let (mock, mut handle) = MockTransport::new();
    //     let (_shutdown_tx, shutdown_rx) = watch::channel(());
    //     let mut client = SignalingClient::new(mock, shutdown_rx);
    //     let msg = SignalingMessage::LoginFailure {
    //         reason: LoginFailureReason::DuplicateId,
    //     };
    //
    //     let result = handle.incoming_tx.send(msg.clone()).await;
    //     assert!(result.is_ok());
    //
    //     let login_result = client.login("token1").await;
    //     assert!(login_result.is_err());
    //     assert_matches!(
    //         login_result,
    //         Err(SignalingError::LoginError(LoginFailureReason::DuplicateId))
    //     );
    //
    //     let sent_message = handle.outgoing_rx.recv().await;
    //     assert_matches!(sent_message, Some(SignalingMessage::Login { ref token }) if token == "token1");
    // }
    //
    // #[test(tokio::test)]
    // async fn login_unexpected_message() {
    //     let (mock, mut handle) = MockTransport::new();
    //     let (_shutdown_tx, shutdown_rx) = watch::channel(());
    //     let mut client = SignalingClient::new(mock, shutdown_rx);
    //     let msg = SignalingMessage::CallAnswer {
    //         peer_id: "client1".to_string(),
    //         sdp: "sdp".to_string(),
    //     };
    //
    //     let result = handle.incoming_tx.send(msg.clone()).await;
    //     assert!(result.is_ok());
    //
    //     let login_result = client.login("token1").await;
    //     assert!(login_result.is_err());
    //     assert_matches!(login_result, Err(SignalingError::ProtocolError(_)));
    //
    //     let sent_message = handle.outgoing_rx.recv().await;
    //     assert_matches!(sent_message, Some(SignalingMessage::Login { ref token }) if token == "token1");
    // }
    //
    // #[test(tokio::test)]
    // async fn login_server_error() {
    //     let (mock, mut handle) = MockTransport::new();
    //     let (_shutdown_tx, shutdown_rx) = watch::channel(());
    //     let mut client = SignalingClient::new(mock, shutdown_rx);
    //     let msg = SignalingMessage::Error {
    //         reason: ErrorReason::Internal("something failed".to_string()),
    //         peer_id: None,
    //     };
    //
    //     let result = handle.incoming_tx.send(msg.clone()).await;
    //     assert!(result.is_ok());
    //
    //     let login_result = client.login("token1").await;
    //     assert!(login_result.is_err());
    //     assert_matches!(login_result, Err(SignalingError::ServerError(_)));
    //
    //     let sent_message = handle.outgoing_rx.recv().await;
    //     assert_matches!(sent_message, Some(SignalingMessage::Login { ref token }) if token == "token1");
    // }
}
