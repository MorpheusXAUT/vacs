use crate::error::SignalingError;
use crate::matcher::ResponseMatcher;
use crate::transport::SignalingTransport;
use std::time::Duration;
use tokio::sync::{broadcast, watch};
use tracing::instrument;
use vacs_protocol::{ClientInfo, SignalingMessage};

pub struct SignalingClientBuilder<T: SignalingTransport> {
    transport: T,
    matcher: ResponseMatcher,
    login_timeout: Duration,
    shutdown_rx: watch::Receiver<()>,
    broadcast_tx: broadcast::Sender<SignalingMessage>,
}

impl<T: SignalingTransport> SignalingClientBuilder<T> {
    #[instrument(level = "debug", skip(transport, shutdown_rx))]
    pub fn new(transport: T, shutdown_rx: watch::Receiver<()>) -> Self {
        Self {
            transport,
            matcher: ResponseMatcher::new(),
            login_timeout: Duration::from_secs(5),
            shutdown_rx,
            broadcast_tx: broadcast::channel(10).0,
        }
    }

    #[instrument(level = "debug", skip(self))]
    pub fn with_login_timeout(mut self, timeout: Duration) -> Self {
        self.login_timeout = timeout;
        self
    }

    #[instrument(level = "debug", skip(self))]
    pub fn build(self) -> SignalingClient<T> {
        SignalingClient {
            transport: self.transport,
            matcher: self.matcher,
            login_timeout: self.login_timeout,
            shutdown_rx: self.shutdown_rx,
            broadcast_tx: self.broadcast_tx,
            is_connected: true,
            is_logged_in: false,
        }
    }
}

pub struct SignalingClient<T: SignalingTransport> {
    transport: T,
    matcher: ResponseMatcher,
    login_timeout: Duration,
    shutdown_rx: watch::Receiver<()>,
    broadcast_tx: broadcast::Sender<SignalingMessage>,
    is_connected: bool,
    is_logged_in: bool,
}

impl<T: SignalingTransport> SignalingClient<T> {
    #[instrument(level = "debug", skip(transport, shutdown_rx))]
    pub fn new(transport: T, shutdown_rx: watch::Receiver<()>) -> Self {
        SignalingClientBuilder::new(transport, shutdown_rx).build()
    }

    pub fn builder(transport: T, shutdown_rx: watch::Receiver<()>) -> SignalingClientBuilder<T> {
        SignalingClientBuilder::new(transport, shutdown_rx)
    }

    pub fn matcher(&self) -> &ResponseMatcher {
        &self.matcher
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SignalingMessage> {
        self.broadcast_tx.subscribe()
    }

    pub fn status(&self) -> (bool, bool) {
        (self.is_connected, self.is_logged_in)
    }

    #[instrument(level = "info", skip(self))]
    pub async fn disconnect(&mut self) -> Result<(), SignalingError> {
        tracing::debug!("Disconnecting signaling client");
        self.is_connected = false;
        self.is_logged_in = false;
        self.transport.close().await
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn send(&mut self, msg: SignalingMessage) -> Result<(), SignalingError> {
        tracing::debug!("Sending message to server");
        let result = tokio::select! {
            biased;
            _ = self.shutdown_rx.changed() => {
                tracing::debug!("Shutdown signal received, aborting send");
                Err(SignalingError::Timeout("Shutdown signal received".to_string()))
            }
            result = self.transport.send(msg) => result,
        };

        if result.is_err() {
            self.disconnect().await?;
        }
        result
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn recv(&mut self) -> Result<SignalingMessage, SignalingError> {
        tracing::debug!("Waiting for message from server");
        let result = tokio::select! {
            biased;
            _ = self.shutdown_rx.changed() => {
                tracing::debug!("Shutdown signal received, aborting recv");
                Err(SignalingError::Timeout("Shutdown signal received".to_string()))
            }
            msg = self.transport.recv() => msg,
        };

        if let Err(SignalingError::Disconnected) = result {
            self.disconnect().await?;
        }
        result
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn recv_with_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<SignalingMessage, SignalingError> {
        tracing::debug!("Waiting for message from server");
        let recv_result = tokio::select! {
            biased;
            _ = self.shutdown_rx.changed() => {
                tracing::debug!("Shutdown signal received, aborting recv");
                return Err(SignalingError::Timeout("Shutdown signal received".to_string()))
            }
            res = tokio::time::timeout(timeout, self.transport.recv()) => res,
        };

        match recv_result {
            Ok(Ok(msg)) => Ok(msg),
            Ok(Err(SignalingError::Disconnected)) => {
                self.disconnect().await?;
                Err(SignalingError::Disconnected)
            }
            Ok(Err(err)) => Err(err),
            Err(_) => {
                tracing::warn!("Timeout waiting for message");
                Err(SignalingError::Timeout(
                    "Timeout waiting for message".to_string(),
                ))
            }
        }
    }

    #[instrument(level = "info", skip(self, token), fields(%id))]
    pub async fn login(
        &mut self,
        id: &str,
        token: &str,
    ) -> Result<Vec<ClientInfo>, SignalingError> {
        tracing::debug!("Sending Login message to server");
        self.send(SignalingMessage::Login {
            id: id.to_string(),
            token: token.to_string(),
        })
        .await?;

        tracing::debug!(login_timeout = ?self.login_timeout, "Awaiting authentication response from server");
        match self.recv_with_timeout(self.login_timeout).await? {
            SignalingMessage::ClientList { clients } => {
                tracing::info!(num_clients = ?clients.len(), "Login successful, received client list");
                self.is_logged_in = true;
                Ok(clients)
            }
            SignalingMessage::LoginFailure { reason } => {
                tracing::warn!(?reason, "Login failed");
                self.is_logged_in = false;
                Err(SignalingError::LoginError(reason))
            }
            SignalingMessage::Error { reason, peer_id } => {
                tracing::error!(?reason, ?peer_id, "Server returned error");
                self.is_logged_in = false;
                Err(SignalingError::ServerError(reason))
            }
            other => {
                tracing::error!(?other, "Received unexpected message from server");
                self.is_logged_in = false;
                Err(SignalingError::ProtocolError(
                    "Expected ClientList after Login".to_string(),
                ))
            }
        }
    }

    #[instrument(level = "info", skip(self))]
    pub async fn logout(&mut self) -> Result<(), SignalingError> {
        tracing::debug!("Sending Logout message to server");
        self.send(SignalingMessage::Logout).await?;
        self.disconnect().await
    }

    #[instrument(level = "info", skip(self))]
    pub async fn handle_interaction(&mut self) {
        tracing::debug!("Handling interaction");

        loop {
            tokio::select! {
                biased;

                _ = self.shutdown_rx.changed() => {
                    tracing::debug!("Shutdown signal received, aborting interaction handling");
                    break;
                }

                msg = self.transport.recv() => {
                    match msg {
                        Ok(message) => {
                            tracing::debug!(?message, "Received message from transport");
                            tracing::trace!(?message, "Trying to match message against matcher");
                            self.matcher.try_match(&message);
                            if self.broadcast_tx.receiver_count() > 0 {
                                tracing::trace!(?message, "Broadcasting message");
                                if let Err(err) = self.broadcast_tx.send(message.clone()) {
                                    tracing::warn!(?message, ?err, "Failed to broadcast message");
                                }
                            } else {
                                tracing::trace!(?message, "No receivers subscribed, not broadcasting message");
                            }
                        }
                        Err(err) => {
                            match err {
                                SignalingError::Disconnected => {
                                    tracing::debug!("Transport disconnected, aborting interaction handling");
                                    break;
                                }
                                err => {
                                    tracing::warn!(?err, "Received error from transport, continuing");
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
        }

        tracing::debug!("Interaction handling complete");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::mock::MockTransport;
    use pretty_assertions::assert_matches;
    use test_log::test;
    use tokio::sync::watch;
    use vacs_protocol::{ErrorReason, LoginFailureReason};

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
    async fn send() {
        let (mock, mut handle) = MockTransport::new();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);
        let msg = SignalingMessage::Login {
            id: "test".to_string(),
            token: "test".to_string(),
        };

        let result = client.send(msg.clone()).await;
        assert!(result.is_ok());

        let sent_msg = handle.outgoing_rx.recv().await.unwrap();
        assert_eq!(sent_msg, msg);
    }

    #[test(tokio::test)]
    async fn send_shutdown() {
        let (mock, _handle) = MockTransport::new();
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);
        let msg = SignalingMessage::Login {
            id: "test".to_string(),
            token: "test".to_string(),
        };

        shutdown_tx.send(()).unwrap();
        let result = client.send(msg.clone()).await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "timeout: Shutdown signal received".to_string()
        );
    }

    #[test(tokio::test)]
    async fn send_disconnected() {
        let (mock, handle) = MockTransport::new();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);
        let msg = SignalingMessage::Login {
            id: "test".to_string(),
            token: "test".to_string(),
        };

        drop(handle.outgoing_rx); // Simulate the outgoing channel being closed

        let send_result = client.send(msg).await;
        assert!(send_result.is_err());
        assert_matches!(send_result, Err(SignalingError::Transport(_)));
    }

    #[test(tokio::test)]
    async fn recv() {
        let (mock, handle) = MockTransport::new();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);
        let msg = SignalingMessage::ClientList {
            clients: test_client_list(),
        };

        let result = handle.incoming_tx.send(msg.clone()).await;
        assert!(result.is_ok());

        let recv_result = client.recv().await;
        assert!(recv_result.is_ok());
        assert_eq!(recv_result.unwrap(), msg);
    }

    #[test(tokio::test)]
    async fn recv_shutdown() {
        let (mock, handle) = MockTransport::new();
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);
        let msg = SignalingMessage::Login {
            id: "test".to_string(),
            token: "test".to_string(),
        };

        let result = handle.incoming_tx.send(msg.clone()).await;
        assert!(result.is_ok());

        shutdown_tx.send(()).unwrap();
        let recv_result = client.recv().await;
        assert!(recv_result.is_err());
        assert_eq!(
            recv_result.unwrap_err().to_string(),
            "timeout: Shutdown signal received".to_string()
        );
    }

    #[test(tokio::test)]
    async fn recv_with_timeout() {
        let (mock, handle) = MockTransport::new();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);
        let msg = SignalingMessage::CallAnswer {
            peer_id: "client1".to_string(),
            sdp: "sdp".to_string(),
        };

        let result = handle.incoming_tx.send(msg.clone()).await;
        assert!(result.is_ok());

        let recv_result = client.recv_with_timeout(Duration::from_millis(100)).await;
        assert!(recv_result.is_ok());
        assert_eq!(recv_result.unwrap(), msg);
    }

    #[test(tokio::test)]
    async fn recv_with_timeout_expired() {
        let (mock, _handle) = MockTransport::new();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);

        let recv_result = client.recv_with_timeout(Duration::from_millis(100)).await;
        assert!(recv_result.is_err());
        assert_eq!(
            recv_result.unwrap_err().to_string(),
            "timeout: Timeout waiting for message".to_string()
        );
    }

    #[test(tokio::test)]
    async fn recv_with_timeout_shutdown() {
        let (mock, handle) = MockTransport::new();
        let (shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);
        let msg = SignalingMessage::Login {
            id: "test".to_string(),
            token: "test".to_string(),
        };

        let result = handle.incoming_tx.send(msg.clone()).await;
        assert!(result.is_ok());

        shutdown_tx.send(()).unwrap();
        let recv_result = client.recv_with_timeout(Duration::from_millis(100)).await;
        assert!(recv_result.is_err());
        assert_eq!(
            recv_result.unwrap_err().to_string(),
            "timeout: Shutdown signal received".to_string()
        );
    }

    #[test(tokio::test)]
    async fn recv_server_error() {
        let (mock, handle) = MockTransport::new();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);
        let msg = SignalingMessage::Error {
            reason: ErrorReason::Internal("something failed".to_string()),
            peer_id: None,
        };

        let result = handle.incoming_tx.send(msg.clone()).await;
        assert!(result.is_ok());

        let recv_result = client.recv().await;
        assert!(recv_result.is_ok());
        assert_eq!(recv_result.unwrap(), msg);
    }

    #[test(tokio::test)]
    async fn recv_peer_connection_error() {
        let (mock, handle) = MockTransport::new();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);
        let msg = SignalingMessage::Error {
            reason: ErrorReason::PeerConnection,
            peer_id: Some("client1".to_string()),
        };

        let result = handle.incoming_tx.send(msg.clone()).await;
        assert!(result.is_ok());

        let recv_result = client.recv().await;
        assert!(recv_result.is_ok());
        assert_eq!(recv_result.unwrap(), msg);
    }

    #[test(tokio::test)]
    async fn recv_disconnected() {
        let (mock, handle) = MockTransport::new();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);

        drop(handle.incoming_tx); // Simulate the incoming channel being closed

        let recv_result = client.recv().await;
        assert!(recv_result.is_err());
        assert_matches!(recv_result, Err(SignalingError::Disconnected));
    }

    #[test(tokio::test)]
    async fn login() {
        let (mock, mut handle) = MockTransport::new();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);
        let test_clients = test_client_list();
        let msg = SignalingMessage::ClientList {
            clients: test_clients.clone(),
        };

        let result = handle.incoming_tx.send(msg.clone()).await;
        assert!(result.is_ok());

        let login_result = client.login("client1", "token1").await;
        assert!(login_result.is_ok());
        assert_eq!(login_result.unwrap(), test_clients);

        let sent_message = handle.outgoing_rx.recv().await;
        assert_matches!(sent_message, Some(SignalingMessage::Login { ref id, ref token }) if id == "client1" && token == "token1");
    }

    #[test(tokio::test)]
    async fn login_timeout() {
        let (mock, mut handle) = MockTransport::new();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClientBuilder::new(mock, shutdown_rx)
            .with_login_timeout(Duration::from_millis(100))
            .build();

        let login_result = client.login("client1", "token1").await;
        assert!(login_result.is_err());
        assert_matches!(login_result, Err(SignalingError::Timeout(_)));

        let sent_message = handle.outgoing_rx.recv().await;
        assert_matches!(sent_message, Some(SignalingMessage::Login { ref id, ref token }) if id == "client1" && token == "token1");
    }

    #[test(tokio::test)]
    async fn login_unauthorized() {
        let (mock, mut handle) = MockTransport::new();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);
        let msg = SignalingMessage::LoginFailure {
            reason: LoginFailureReason::Unauthorized,
        };

        let result = handle.incoming_tx.send(msg.clone()).await;
        assert!(result.is_ok());

        let login_result = client.login("client1", "token1").await;
        assert!(login_result.is_err());
        assert_matches!(
            login_result,
            Err(SignalingError::LoginError(LoginFailureReason::Unauthorized))
        );

        let sent_message = handle.outgoing_rx.recv().await;
        assert_matches!(sent_message, Some(SignalingMessage::Login { ref id, ref token }) if id == "client1" && token == "token1");
    }

    #[test(tokio::test)]
    async fn login_invalid_credentials() {
        let (mock, mut handle) = MockTransport::new();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);
        let msg = SignalingMessage::LoginFailure {
            reason: LoginFailureReason::InvalidCredentials,
        };

        let result = handle.incoming_tx.send(msg.clone()).await;
        assert!(result.is_ok());

        let login_result = client.login("client1", "token1").await;
        assert!(login_result.is_err());
        assert_matches!(
            login_result,
            Err(SignalingError::LoginError(
                LoginFailureReason::InvalidCredentials
            ))
        );

        let sent_message = handle.outgoing_rx.recv().await;
        assert_matches!(sent_message, Some(SignalingMessage::Login { ref id, ref token }) if id == "client1" && token == "token1");
    }

    #[test(tokio::test)]
    async fn login_duplicate_id() {
        let (mock, mut handle) = MockTransport::new();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);
        let msg = SignalingMessage::LoginFailure {
            reason: LoginFailureReason::DuplicateId,
        };

        let result = handle.incoming_tx.send(msg.clone()).await;
        assert!(result.is_ok());

        let login_result = client.login("client1", "token1").await;
        assert!(login_result.is_err());
        assert_matches!(
            login_result,
            Err(SignalingError::LoginError(LoginFailureReason::DuplicateId))
        );

        let sent_message = handle.outgoing_rx.recv().await;
        assert_matches!(sent_message, Some(SignalingMessage::Login { ref id, ref token }) if id == "client1" && token == "token1");
    }

    #[test(tokio::test)]
    async fn login_unexpected_message() {
        let (mock, mut handle) = MockTransport::new();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);
        let msg = SignalingMessage::CallAnswer {
            peer_id: "client1".to_string(),
            sdp: "sdp".to_string(),
        };

        let result = handle.incoming_tx.send(msg.clone()).await;
        assert!(result.is_ok());

        let login_result = client.login("client1", "token1").await;
        assert!(login_result.is_err());
        assert_matches!(login_result, Err(SignalingError::ProtocolError(_)));

        let sent_message = handle.outgoing_rx.recv().await;
        assert_matches!(sent_message, Some(SignalingMessage::Login { ref id, ref token }) if id == "client1" && token == "token1");
    }

    #[test(tokio::test)]
    async fn login_server_error() {
        let (mock, mut handle) = MockTransport::new();
        let (_shutdown_tx, shutdown_rx) = watch::channel(());
        let mut client = SignalingClient::new(mock, shutdown_rx);
        let msg = SignalingMessage::Error {
            reason: ErrorReason::Internal("something failed".to_string()),
            peer_id: None,
        };

        let result = handle.incoming_tx.send(msg.clone()).await;
        assert!(result.is_ok());

        let login_result = client.login("client1", "token1").await;
        assert!(login_result.is_err());
        assert_matches!(login_result, Err(SignalingError::ServerError(_)));

        let sent_message = handle.outgoing_rx.recv().await;
        assert_matches!(sent_message, Some(SignalingMessage::Login { ref id, ref token }) if id == "client1" && token == "token1");
    }
}
