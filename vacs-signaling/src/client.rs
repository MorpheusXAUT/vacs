use crate::auth::TokenProvider;
use crate::error::{SignalingError, SignalingRuntimeError};
use crate::matcher::ResponseMatcher;
use crate::transport::{SignalingReceiver, SignalingSender, SignalingTransport};
use parking_lot::Mutex;
use rand::{Rng, SeedableRng};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::{JoinHandle, JoinSet};
use tokio_tungstenite::tungstenite;
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, instrument};
use vacs_protocol::VACS_PROTOCOL_VERSION;
use vacs_protocol::ws::{ClientInfo, SignalingMessage};

const BROADCAST_CHANNEL_SIZE: usize = 100;
const SEND_CHANNEL_SIZE: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Default state, no connection to the server, messages cannot be sent or received.
    /// This state will also be set when a disconnect is requested, a websocket timeout/error is received,
    /// or one of the processing tasks encounters an error.
    Disconnected,
    /// Connected to the server but not logged in yet.
    /// This state is set after the [`SignalingClient`] has successfully established a websocket connection
    /// to the server but has not performed authentication yet.
    /// The only message that can be sent is [`SignalingMessage::Login`], as the server will reject all others.
    /// The [`SignalingClient`] will automatically perform a login using the TokenProvider's auth token.
    /// Depending on the result of the login, the [`State`] will either change to [`State::LoggedIn`] (on success)
    /// or [`State::Disconnected`] (on receiving a login failure).
    Connected,
    /// Connected to the server and successfully authenticated.
    /// This state is set after a successful login. Messages can be sent and received.
    LoggedIn,
}

#[derive(Debug, Clone)]
pub enum SignalingEvent {
    /// Emitted after the [`SignalingClient`] successfully connected to the server, including authentication.
    /// The client is ready to send and receive messages.
    Connected {
        clients: Vec<ClientInfo>,
        display_name: String,
    },
    /// Emitted for every [`SignalingMessage`] received by a connected and authenticated [`SignalingClientInner`].
    Message(SignalingMessage),
    /// Emitted for every [`SignalingRuntimeError`] handled by the [`SignalingClientInner`].
    /// This includes issues during transmission or other errors received from the server.
    Error(SignalingRuntimeError),
}

type BoxFutUnit = Pin<Box<dyn Future<Output = ()> + Send>>;
type OnEventCb = Arc<dyn Fn(SignalingEvent) -> BoxFutUnit + Send + Sync>;

#[derive(Clone)]
pub struct SignalingClient<ST: SignalingTransport, TP: TokenProvider> {
    inner: Arc<SignalingClientInner<ST, TP>>,
    supervisor_task: Arc<JoinHandle<()>>,
}

impl<ST: SignalingTransport, TP: TokenProvider> SignalingClient<ST, TP> {
    pub fn new<F, Fut>(
        transport: ST,
        token_provider: TP,
        on_event: F,
        shutdown_token: CancellationToken,
        login_timeout: Duration,
        reconnect_max_tries: u8,
        handle: &tokio::runtime::Handle,
    ) -> Self
    where
        F: Fn(SignalingEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let inner = Arc::new(SignalingClientInner::new(
            transport,
            token_provider,
            Arc::new(move |e| Box::pin(on_event(e))),
            shutdown_token,
            login_timeout,
            reconnect_max_tries,
        ));

        let inner_clone = inner.clone();
        let supervisor_task = Arc::new(handle.spawn(async move {
            inner_clone.supervisor_task().await;
        }));

        Self {
            inner,
            supervisor_task,
        }
    }

    /// Subscribes to a broadcast channel emitting [`SignalingEvent`]s.
    pub fn subscribe(&self) -> broadcast::Receiver<SignalingEvent> {
        self.inner.subscribe()
    }

    /// Subscribes to a watch containing the current [`SignalingClient`]'s [`State`].
    pub fn subscribe_state(&self) -> watch::Receiver<State> {
        self.inner.subscribe_state()
    }

    pub fn state(&self) -> State {
        self.inner.state()
    }

    pub async fn connect(&self) -> Result<(), SignalingError> {
        self.inner.connect().await
    }

    pub async fn disconnect(&self) {
        self.inner.disconnect(true).await;
    }

    pub async fn send(&self, msg: SignalingMessage) -> Result<(), SignalingError> {
        self.inner.send(msg).await
    }

    pub async fn recv(&self) -> Result<SignalingMessage, SignalingError> {
        self.inner.recv().await
    }

    pub fn matcher(&self) -> &ResponseMatcher {
        self.inner.matcher()
    }

    pub async fn recv_with_timeout(
        &self,
        timeout: Duration,
    ) -> Result<SignalingMessage, SignalingError> {
        self.inner.recv_with_timeout(timeout).await
    }
}

impl<ST: SignalingTransport, TP: TokenProvider> Drop for SignalingClient<ST, TP> {
    fn drop(&mut self) {
        self.inner.shutdown_token.cancel();
        self.supervisor_task.abort();
    }
}

#[derive(Clone)]
struct SignalingClientInner<ST: SignalingTransport, TP: TokenProvider> {
    transport: ST,
    token_provider: TP,

    on_event: OnEventCb,

    state_tx: watch::Sender<State>,
    state_rx: watch::Receiver<State>,

    disconnect_token: Arc<Mutex<CancellationToken>>,
    shutdown_token: CancellationToken,

    matcher: ResponseMatcher,
    broadcast_tx: broadcast::Sender<SignalingEvent>,

    send_tx: Arc<Mutex<Option<mpsc::Sender<tungstenite::Message>>>>,

    login_timeout: Duration,
    reconnect_max_tries: u8,

    worker_tasks: Arc<Mutex<JoinSet<()>>>,
}

impl<ST: SignalingTransport, TP: TokenProvider> SignalingClientInner<ST, TP> {
    #[instrument(level = "debug", skip_all)]
    fn new(
        transport: ST,
        token_provider: TP,
        on_event: OnEventCb,
        shutdown_token: CancellationToken,
        login_timeout: Duration,
        reconnect_max_tries: u8,
    ) -> Self {
        let (state_tx, state_rx) = watch::channel(State::Disconnected);
        Self {
            transport,
            token_provider,

            on_event,

            state_tx,
            state_rx,

            disconnect_token: Arc::new(Mutex::new(shutdown_token.child_token())),
            shutdown_token,

            matcher: ResponseMatcher::new(),
            broadcast_tx: broadcast::channel(BROADCAST_CHANNEL_SIZE).0,

            send_tx: Arc::new(Mutex::new(None)),

            login_timeout,
            reconnect_max_tries,

            worker_tasks: Arc::new(Mutex::new(JoinSet::new())),
        }
    }

    fn matcher(&self) -> &ResponseMatcher {
        &self.matcher
    }

    /// Subscribes to a broadcast channel emitting [`SignalingEvent`]s.
    fn subscribe(&self) -> broadcast::Receiver<SignalingEvent> {
        self.broadcast_tx.subscribe()
    }

    /// Subscribes to a watch containing the current [`SignalingClient`] [`State`].
    fn subscribe_state(&self) -> watch::Receiver<State> {
        self.state_tx.subscribe()
    }

    fn state(&self) -> State {
        *self.state_rx.borrow()
    }

    #[instrument(level = "debug", skip_all)]
    async fn disconnect(&self, requested: bool) {
        tracing::trace!(?requested, "Sending logout message before disconnecting");
        if let Err(err) = self.send(SignalingMessage::Logout).await {
            tracing::warn!(?err, "Failed to send Logout message before disconnecting");
        }
        self.disconnect_token.lock().cancel();
        self.set_state(State::Disconnected);
        self.cleanup().await;
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn send(&self, msg: SignalingMessage) -> Result<(), SignalingError> {
        match self.state() {
            State::Disconnected => {
                tracing::warn!("Tried to send message before signaling client was started");
                return Err(SignalingError::Runtime(SignalingRuntimeError::Disconnected));
            }
            State::Connected if !matches!(msg, SignalingMessage::Login { .. }) => {
                tracing::warn!("Tried to send message before login");
                return Err(SignalingError::Runtime(SignalingRuntimeError::Disconnected));
            }
            _ => {}
        };

        let send_tx = {
            self.send_tx.lock().as_ref().cloned().ok_or_else(|| {
                tracing::error!("Client is connected, but send_tx is not initialized");
                SignalingError::Runtime(SignalingRuntimeError::Disconnected)
            })?
        };

        tracing::debug!("Sending message to send channel");
        let serialized = SignalingMessage::serialize(&msg).map_err(|err| {
            tracing::warn!(?err, "Failed to serialize message");
            SignalingError::Runtime(SignalingRuntimeError::SerializationError(err.to_string()))
        })?;

        send_tx
            .send(tungstenite::Message::from(serialized))
            .await
            .map_err(|_| SignalingError::Runtime(SignalingRuntimeError::Disconnected))
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn recv(&self) -> Result<SignalingMessage, SignalingError> {
        tracing::debug!("Waiting for message from server");
        self.recv_with_timeout(Duration::MAX).await
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn recv_with_timeout(
        &self,
        timeout: Duration,
    ) -> Result<SignalingMessage, SignalingError> {
        tracing::debug!("Waiting for message from server with timeout");
        let mut broadcast_rx = self.subscribe();

        if self.state() == State::Disconnected {
            tracing::warn!("Tried to receive message without transport being connected");
            return Err(SignalingError::Runtime(SignalingRuntimeError::Disconnected));
        }

        let disconnect_token = self.disconnect_token.lock().clone();
        let recv_result = tokio::select! {
            biased;
            _ = disconnect_token.cancelled() => {
                tracing::debug!("Shutdown signal received, aborting receive");
                return Err(SignalingError::Timeout("Shutdown signal received".to_string()))
            }
            res = tokio::time::timeout(timeout, async {
                loop {
                    match broadcast_rx.recv().await {
                        Ok(SignalingEvent::Message(msg)) => return Ok(msg),
                        Err(err) => return Err(err),
                        _ => continue,
                    }
                }
            }) => res,
        };

        match recv_result {
            Ok(Ok(msg)) => Ok(msg),
            Ok(Err(err)) => Err(SignalingError::Other(err.to_string())),
            Err(_) => {
                tracing::warn!("Timeout waiting for message");
                Err(SignalingError::Timeout(
                    "Timeout waiting for message".to_string(),
                ))
            }
        }
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn login(&self) -> Result<(String, Vec<ClientInfo>), SignalingError> {
        tracing::trace!("Retrieving auth token from token provider");
        let token = self.token_provider.get_token().await?;
        tracing::debug!("Sending Login message to server");
        self.send(SignalingMessage::Login {
            token: token.to_string(),
            protocol_version: VACS_PROTOCOL_VERSION.to_string(),
        })
        .await?;

        tracing::debug!("Awaiting authentication response from server");
        match self.recv_with_timeout(self.login_timeout).await? {
            SignalingMessage::ClientList { clients } => {
                tracing::info!(num_clients = ?clients.len(), "Login successful, received client list");
                Ok(("LOVV_CTR".to_string(), clients))
            }
            SignalingMessage::LoginFailure { reason } => {
                tracing::warn!(?reason, "Login failed");
                Err(SignalingError::LoginError(reason))
            }
            SignalingMessage::Error { reason, peer_id } => {
                tracing::error!(?reason, ?peer_id, "Server returned error");
                Err(SignalingError::Runtime(SignalingRuntimeError::ServerError(
                    reason,
                )))
            }
            other => {
                tracing::error!(?other, "Received unexpected message from server");
                Err(SignalingError::ProtocolError(
                    "Expected ClientList after Login".to_string(),
                ))
            }
        }
    }

    #[instrument(level = "debug", skip(self), err)]
    pub async fn connect(&self) -> Result<(), SignalingError> {
        tracing::trace!("Connecting to signaling server");
        let (sender, receiver) = self.transport.connect().await?;

        let (send_tx, send_rx) = mpsc::channel::<tungstenite::Message>(SEND_CHANNEL_SIZE);
        tracing::trace!("Successfully connected to signaling server, starting worker tasks");
        {
            let mut tasks = self.worker_tasks.lock();
            let rt_handle = tokio::runtime::Handle::current();

            let matcher = self.matcher.clone();
            let broadcast_tx = self.broadcast_tx.clone();
            tasks.spawn_on(
                Self::reader_task(
                    receiver,
                    send_tx.clone(),
                    matcher,
                    broadcast_tx,
                    self.disconnect_token.lock().clone(),
                    self.subscribe_state(),
                ),
                &rt_handle,
            );

            let broadcast_tx = self.broadcast_tx.clone();
            tasks.spawn_on(
                Self::writer_task(
                    sender,
                    send_rx,
                    broadcast_tx,
                    self.disconnect_token.lock().clone(),
                    self.subscribe_state(),
                ),
                &rt_handle,
            );
        }

        *self.send_tx.lock() = Some(send_tx);
        self.set_state(State::Connected);

        tracing::trace!("Successfully started worker tasks, logging in");
        match self.login().await {
            Ok((display_name, clients)) => {
                tracing::trace!("Successfully logged in to server");

                self.set_state(State::LoggedIn);
                if let Err(err) = self.broadcast_tx.send(SignalingEvent::Connected {
                    display_name,
                    clients,
                }) {
                    tracing::warn!(?err, "Failed to broadcast connected event");
                }

                Ok(())
            }
            Err(err) => {
                tracing::warn!(?err, "Failed to login to server");
                self.disconnect(false).await;
                Err(err)
            }
        }
    }

    #[instrument(level = "debug", skip(self))]
    async fn cleanup(&self) {
        tracing::debug!("Cleaning up after disconnect");

        let mut worker_tasks = {
            let mut worker_tasks = self.worker_tasks.lock();
            std::mem::replace(&mut *worker_tasks, JoinSet::new())
        };

        tracing::trace!("Aborting worker tasks");
        worker_tasks.abort_all();
        tracing::trace!("Waiting for worker tasks to finish");
        while let Some(res) = worker_tasks.join_next().await {
            if let Err(err) = res
                && !err.is_cancelled()
            {
                tracing::warn!(?err, "Failed to join worker task");
            }
        }

        self.matcher.clear().await;
        *self.disconnect_token.lock() = self.shutdown_token.child_token();
        self.send_tx.lock().take();

        tracing::debug!("Finished cleaning up after disconnect");
    }

    #[instrument(level = "debug", skip(self))]
    async fn supervisor_task(self: Arc<Self>) {
        tracing::debug!("Starting supervisor task");

        let mut broadcast_rx = self.subscribe();

        loop {
            tokio::select! {
                biased;

                _ = self.shutdown_token.cancelled() => {
                    tracing::debug!("Shutdown signal received, exiting supervisor task");
                    break;
                }

                event = broadcast_rx.recv() => {
                    match event {
                        Ok(event) => {
                            if let SignalingEvent::Error(err) = &event && err.is_fatal() {
                                (self.on_event)(event.clone()).await;

                                tracing::debug!(?err, "Received error event, disconnecting");
                                self.disconnect(false).await;

                                if err.can_reconnect() {
                                    // TODO prevent endless reconnect loop within short timeframe
                                    tracing::info!("Reconnecting after error");
                                    if let Err(err) = self.reconnect().await {
                                        tracing::warn!(?err, "Received error while reconnecting");
                                        if let Err(err) = self.broadcast_tx.send(SignalingEvent::Error(err)) {
                                            tracing::warn!(?err, "Failed to broadcast reconnect error event");
                                        }
                                    }
                                }
                            } else {
                                (self.on_event)(event).await;
                            }
                        },
                        Err(err) => {
                            tracing::warn!(?err, "Failed to receive broadcast event, exiting supervisor task");
                            self.disconnect(false).await;
                            break;
                        }
                    }
                }
            }
        }

        tracing::debug!("Supervisor task finished");
    }

    fn set_state(&self, state: State) {
        if let Err(err) = self.state_tx.send(state) {
            tracing::warn!(?err, "Failed to update client state");
        }
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn reconnect(&self) -> Result<(), SignalingRuntimeError> {
        if self.reconnect_max_tries == 0 {
            tracing::debug!("Reconnecting disabled");
            return Ok(());
        }

        let mut retry_strategy = RetryStrategy::default();

        let mut reconnect_error = SignalingError::Other("Unknown".to_string());
        for attempt in 1..=self.reconnect_max_tries {
            tracing::trace!(?attempt, "Reconnecting");
            reconnect_error = {
                match self.connect().await {
                    Ok(()) => return Ok(()),
                    Err(err) => {
                        let timeout = retry_strategy.timeout(attempt as u32);
                        tracing::warn!(?err, ?attempt, ?timeout, "Failed to reconnect");
                        tokio::time::sleep(timeout).await;
                        err
                    }
                }
            }
        }

        Err(SignalingRuntimeError::ReconnectFailed(
            reconnect_error.into(),
        ))
    }

    #[instrument(level = "debug", skip(state_rx, broadcast_tx))]
    fn emit_task_error(
        state_rx: &watch::Receiver<State>,
        broadcast_tx: &broadcast::Sender<SignalingEvent>,
        err: SignalingRuntimeError,
    ) {
        let state = *state_rx.borrow();
        tracing::warn!(?state, "Received error from task");
        if state == State::LoggedIn
            && broadcast_tx
                .send(SignalingEvent::Error(err.clone()))
                .is_err()
        {
            tracing::warn!("Failed to broadcast task error signaling event");
        }
    }

    #[instrument(level = "debug", skip_all)]
    fn reader_task<R: SignalingReceiver>(
        mut receiver: R,
        send_tx: mpsc::Sender<tungstenite::Message>,
        matcher: ResponseMatcher,
        broadcast_tx: broadcast::Sender<SignalingEvent>,
        disconnect_token: CancellationToken,
        state_rx: watch::Receiver<State>,
    ) -> impl Future<Output = ()> + Send {
        async move {
            tracing::debug!("Starting transport reader task");
            let _guard = TaskDropLogger::new("reader");

            loop {
                tokio::select! {
                    biased;

                    _ = disconnect_token.cancelled() => {
                        tracing::debug!("Disconnect signal received, exiting transport reader task");
                        break;
                    }

                    msg = receiver.recv(&send_tx) => {
                        match msg {
                            Ok(message) => {
                                tracing::trace!(?message, "Received message from transport, trying to match against matcher");
                                matcher.try_match(&message);
                                if broadcast_tx.receiver_count() > 0 {
                                    tracing::trace!(?message, "Broadcasting message");
                                    if let Err(err) = broadcast_tx.send(SignalingEvent::Message(message.clone())) {
                                        tracing::warn!(?message, ?err, "Failed to broadcast message");
                                    }
                                } else {
                                    tracing::trace!(?message, "No receivers subscribed, not broadcasting message");
                                }
                            }
                            Err(err) => {
                                Self::emit_task_error(&state_rx, &broadcast_tx, err);
                                break;
                            }
                        }
                    }
                }
            }
        }.instrument(tracing::Span::current())
    }

    #[instrument(level = "debug", skip_all)]
    fn writer_task<S: SignalingSender>(
        mut sender: S,
        mut send_rx: mpsc::Receiver<tungstenite::Message>,
        broadcast_tx: broadcast::Sender<SignalingEvent>,
        disconnect_token: CancellationToken,
        state_rx: watch::Receiver<State>,
    ) -> impl Future<Output = ()> + Send {
        async move {
            tracing::debug!("Starting transport writer task");
            let _guard = TaskDropLogger::new("writer");

            loop {
                tokio::select! {
                    biased;

                    _ = disconnect_token.cancelled() => {
                        tracing::debug!("Disconnect signal received, closing sender");

                        if let Err(err) = sender.close().await {
                            tracing::warn!(?err, "Failed to close transport");
                        }

                        tracing::debug!("Successfully closed sender, exiting transport writer task");
                        break;
                    }

                    msg = send_rx.recv() => {
                        match msg {
                            Some(msg) => {
                                if !matches!(msg, tungstenite::Message::Pong(_)) {
                                    tracing::debug!(?msg, "Sending message to transport");
                                }

                                if let Err(err) = sender.send(msg).await {
                                    Self::emit_task_error(&state_rx, &broadcast_tx, err);
                                    break;
                                }
                            },
                            None => {
                                Self::emit_task_error(&state_rx, &broadcast_tx, SignalingRuntimeError::Disconnected);
                                break;
                            }
                        }
                    }
                }
            }
        }.instrument(tracing::Span::current())
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

pub struct RetryStrategy {
    base: Duration,
    cap: Duration,
    rng: rand::rngs::StdRng,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            base: Duration::from_millis(100),
            cap: Duration::from_secs(5),
            rng: rand::rngs::StdRng::from_os_rng(),
        }
    }
}

impl RetryStrategy {
    fn timeout(&mut self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        }

        // exp = base * 2^(attempt - 1), capped
        let exp_nanos = self
            .base
            .as_nanos()
            .saturating_mul(1u128 << attempt.saturating_sub(1).min(63));
        let max_delay_nanos = exp_nanos.min(self.cap.as_nanos());

        let jitter_nanos = if max_delay_nanos == 0 {
            0
        } else {
            // full jitter
            self.rng.random_range(0..=max_delay_nanos)
        };

        Duration::from_nanos(jitter_nanos.min(u128::from(u64::MAX)) as u64)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::transport::mock;
//     use pretty_assertions::{assert_eq, assert_matches};
//     use test_log::test;
//     use tokio::sync::watch;
//     use vacs_protocol::ws::{ErrorReason, LoginFailureReason};
//
//     fn test_client_list() -> Vec<ClientInfo> {
//         vec![
//             ClientInfo {
//                 id: "client1".to_string(),
//                 display_name: "Client 1".to_string(),
//             },
//             ClientInfo {
//                 id: "client2".to_string(),
//                 display_name: "Client 2".to_string(),
//             },
//             ClientInfo {
//                 id: "client3".to_string(),
//                 display_name: "Client 3".to_string(),
//             },
//         ]
//     }
//
//     #[test(tokio::test)]
//     async fn start() {
//         let ((sender, receiver), _handle) = mock::create();
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//
//         assert!(ready_rx.await.is_ok());
//         assert_matches!(client.status(), (true, false));
//     }
//
//     #[test(tokio::test)]
//     async fn start_shutdown() {
//         let ((sender, receiver), _handle) = mock::create();
//         let (shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         let task = tokio::spawn(async move {
//             return client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//
//         shutdown_tx.send(()).unwrap();
//
//         assert_matches!(task.await, Ok(InterruptionReason::ShutdownSignal));
//         assert_matches!(client.status(), (false, false));
//     }
//
//     #[test(tokio::test)]
//     async fn disconnect() {
//         let ((sender, receiver), _handle) = mock::create();
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let mut client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         let task = tokio::spawn(async move {
//             return client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//
//         client.disconnect();
//
//         assert_matches!(task.await, Ok(InterruptionReason::Disconnected(true)));
//         assert_matches!(client.status(), (false, false));
//     }
//
//     #[test(tokio::test)]
//     async fn send() {
//         let ((sender, receiver), mut handle) = mock::create();
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//
//         let msg = SignalingMessage::Login {
//             token: "test".to_string(),
//             protocol_version: VACS_PROTOCOL_VERSION.to_string(),
//         };
//
//         let result = client.send(msg.clone()).await;
//         assert!(result.is_ok());
//
//         let sent_msg = handle.outgoing_rx.recv().await.unwrap();
//         assert_eq!(
//             sent_msg,
//             tungstenite::Message::from(SignalingMessage::serialize(&msg).unwrap())
//         );
//     }
//
//     #[test(tokio::test)]
//     async fn send_without_start() {
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let client = SignalingClientInner::new(shutdown_rx);
//
//         let msg = SignalingMessage::Login {
//             token: "test".to_string(),
//             protocol_version: VACS_PROTOCOL_VERSION.to_string(),
//         };
//
//         let result = client.send(msg.clone()).await;
//         assert_matches!(result, Err(SignalingError::Disconnected));
//     }
//
//     #[test(tokio::test)]
//     async fn send_without_login() {
//         let ((sender, receiver), _handle) = mock::create();
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//
//         let msg = SignalingMessage::CallOffer {
//             peer_id: "client1".to_string(),
//             sdp: "".to_string(),
//         };
//
//         let result = client.send(msg.clone()).await;
//         assert_matches!(result, Err(SignalingError::ProtocolError { .. }));
//     }
//
//     #[test(tokio::test)]
//     async fn send_disconnected() {
//         let ((sender, receiver), _handle) = mock::create();
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let mut client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//         assert_matches!(client.status(), (true, false));
//
//         client.disconnect();
//         tokio::time::sleep(Duration::from_millis(100)).await;
//         assert_matches!(client.status(), (false, false));
//
//         let msg = SignalingMessage::Login {
//             token: "test".to_string(),
//             protocol_version: VACS_PROTOCOL_VERSION.to_string(),
//         };
//
//         let result = client.send(msg.clone()).await;
//         assert_matches!(result, Err(SignalingError::Disconnected));
//     }
//
//     #[test(tokio::test)]
//     async fn send_shutdown() {
//         let ((sender, receiver), _handle) = mock::create();
//         let (shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//         assert_matches!(client.status(), (true, false));
//
//         shutdown_tx.send(()).unwrap();
//         tokio::time::sleep(Duration::from_millis(100)).await;
//         assert_matches!(client.status(), (false, false));
//
//         let msg = SignalingMessage::Login {
//             token: "test".to_string(),
//             protocol_version: VACS_PROTOCOL_VERSION.to_string(),
//         };
//
//         let result = client.send(msg.clone()).await;
//         assert_matches!(result, Err(SignalingError::Disconnected));
//     }
//
//     #[test(tokio::test)]
//     async fn recv() {
//         let ((sender, receiver), handle) = mock::create();
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let mut client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//
//         let msg = SignalingMessage::ClientList {
//             clients: test_client_list(),
//         };
//
//         let task = tokio::spawn(async move {
//             return client.recv().await;
//         });
//
//         let result = handle
//             .incoming_tx
//             .send(tungstenite::Message::from(
//                 SignalingMessage::serialize(&msg).unwrap(),
//             ))
//             .await;
//         assert!(result.is_ok());
//
//         assert_eq!(task.await.unwrap().unwrap(), msg);
//     }
//
//     #[test(tokio::test)]
//     async fn recv_shutdown() {
//         let ((sender, receiver), handle) = mock::create();
//         let (shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//
//         let msg = SignalingMessage::ClientList {
//             clients: test_client_list(),
//         };
//
//         let mut client_clone = client.clone();
//         let task = tokio::spawn(async move {
//             return client_clone.recv().await;
//         });
//
//         let result = handle
//             .incoming_tx
//             .send(tungstenite::Message::from(
//                 SignalingMessage::serialize(&msg).unwrap(),
//             ))
//             .await;
//         assert!(result.is_ok());
//
//         shutdown_tx.send(()).unwrap();
//         tokio::time::sleep(Duration::from_millis(100)).await;
//         assert_matches!(client.status(), (false, false));
//
//         let recv_result = task.await.unwrap();
//         assert!(recv_result.is_err());
//         assert_eq!(
//             recv_result.unwrap_err().to_string(),
//             "timeout: Shutdown signal received".to_string()
//         );
//     }
//
//     #[test(tokio::test)]
//     async fn recv_with_timeout() {
//         let ((sender, receiver), handle) = mock::create();
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let mut client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//
//         let msg = SignalingMessage::ClientList {
//             clients: test_client_list(),
//         };
//
//         let task = tokio::spawn(async move {
//             return client.recv_with_timeout(Duration::from_millis(100)).await;
//         });
//
//         let result = handle
//             .incoming_tx
//             .send(tungstenite::Message::from(
//                 SignalingMessage::serialize(&msg).unwrap(),
//             ))
//             .await;
//         assert!(result.is_ok());
//
//         assert_eq!(task.await.unwrap().unwrap(), msg);
//     }
//
//     #[test(tokio::test)]
//     async fn recv_with_timeout_expired() {
//         let ((sender, receiver), handle) = mock::create();
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let mut client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//
//         let msg = SignalingMessage::ClientList {
//             clients: test_client_list(),
//         };
//
//         let task = tokio::spawn(async move {
//             return client.recv_with_timeout(Duration::from_millis(10)).await;
//         });
//         tokio::time::sleep(Duration::from_millis(50)).await;
//
//         let result = handle
//             .incoming_tx
//             .send(tungstenite::Message::from(
//                 SignalingMessage::serialize(&msg).unwrap(),
//             ))
//             .await;
//         assert!(result.is_ok());
//
//         let recv_result = task.await.unwrap();
//         assert!(recv_result.is_err());
//         assert_eq!(
//             recv_result.unwrap_err().to_string(),
//             "timeout: Timeout waiting for message".to_string()
//         );
//     }
//
//     #[test(tokio::test)]
//     async fn recv_disconnected() {
//         let ((sender, receiver), handle) = mock::create();
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let mut client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//
//         drop(handle.incoming_tx);
//         tokio::time::sleep(Duration::from_millis(100)).await;
//         assert_matches!(client.status(), (false, false));
//
//         let recv_result = client.recv().await;
//         assert!(recv_result.is_err());
//         assert_matches!(recv_result, Err(SignalingError::Disconnected));
//     }
//
//     #[test(tokio::test)]
//     async fn login() {
//         let ((sender, receiver), mut handle) = mock::create();
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let mut client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//
//         let test_clients = test_client_list();
//         let msg = SignalingMessage::ClientList {
//             clients: test_clients.clone(),
//         };
//
//         let result = handle
//             .incoming_tx
//             .send(tungstenite::Message::from(
//                 SignalingMessage::serialize(&msg).unwrap(),
//             ))
//             .await;
//         assert!(result.is_ok());
//
//         let login_result = client.login("token1", Duration::from_millis(100)).await;
//         assert!(login_result.is_ok());
//         assert_eq!(login_result.unwrap(), test_clients);
//
//         let login_msg = tungstenite::Message::from(
//             SignalingMessage::serialize(&SignalingMessage::Login {
//                 token: "token1".to_string(),
//                 protocol_version: VACS_PROTOCOL_VERSION.to_string(),
//             })
//             .unwrap(),
//         );
//
//         let sent_message = handle.outgoing_rx.recv().await;
//         assert_eq!(sent_message, Some(login_msg));
//     }
//
//     #[test(tokio::test)]
//     async fn login_timeout() {
//         let ((sender, receiver), mut handle) = mock::create();
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let mut client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//
//         let login_result = client.login("token1", Duration::from_millis(100)).await;
//         assert!(login_result.is_err());
//         assert_matches!(login_result, Err(SignalingError::Timeout(_)));
//
//         let login_msg = tungstenite::Message::from(
//             SignalingMessage::serialize(&SignalingMessage::Login {
//                 token: "token1".to_string(),
//                 protocol_version: VACS_PROTOCOL_VERSION.to_string(),
//             })
//             .unwrap(),
//         );
//
//         let sent_message = handle.outgoing_rx.recv().await;
//         assert_eq!(sent_message, Some(login_msg));
//     }
//
//     #[test(tokio::test)]
//     async fn login_unauthorized() {
//         let ((sender, receiver), mut handle) = mock::create();
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let mut client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//
//         let msg = SignalingMessage::LoginFailure {
//             reason: LoginFailureReason::Unauthorized,
//         };
//
//         let result = handle
//             .incoming_tx
//             .send(tungstenite::Message::from(
//                 SignalingMessage::serialize(&msg).unwrap(),
//             ))
//             .await;
//         assert!(result.is_ok());
//
//         let login_result = client.login("token1", Duration::from_millis(100)).await;
//         assert!(login_result.is_err());
//         assert_matches!(
//             login_result,
//             Err(SignalingError::LoginError(LoginFailureReason::Unauthorized))
//         );
//
//         let login_msg = tungstenite::Message::from(
//             SignalingMessage::serialize(&SignalingMessage::Login {
//                 token: "token1".to_string(),
//                 protocol_version: VACS_PROTOCOL_VERSION.to_string(),
//             })
//             .unwrap(),
//         );
//
//         let sent_message = handle.outgoing_rx.recv().await;
//         assert_eq!(sent_message, Some(login_msg));
//     }
//
//     #[test(tokio::test)]
//     async fn login_invalid_credentials() {
//         let ((sender, receiver), mut handle) = mock::create();
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let mut client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//
//         let msg = SignalingMessage::LoginFailure {
//             reason: LoginFailureReason::InvalidCredentials,
//         };
//
//         let result = handle
//             .incoming_tx
//             .send(tungstenite::Message::from(
//                 SignalingMessage::serialize(&msg).unwrap(),
//             ))
//             .await;
//         assert!(result.is_ok());
//
//         let login_result = client.login("token1", Duration::from_millis(100)).await;
//         assert!(login_result.is_err());
//         assert_matches!(
//             login_result,
//             Err(SignalingError::LoginError(
//                 LoginFailureReason::InvalidCredentials
//             ))
//         );
//
//         let login_msg = tungstenite::Message::from(
//             SignalingMessage::serialize(&SignalingMessage::Login {
//                 token: "token1".to_string(),
//                 protocol_version: VACS_PROTOCOL_VERSION.to_string(),
//             })
//             .unwrap(),
//         );
//
//         let sent_message = handle.outgoing_rx.recv().await;
//         assert_eq!(sent_message, Some(login_msg));
//     }
//
//     #[test(tokio::test)]
//     async fn login_duplicate_id() {
//         let ((sender, receiver), mut handle) = mock::create();
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let mut client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//
//         let msg = SignalingMessage::LoginFailure {
//             reason: LoginFailureReason::DuplicateId,
//         };
//
//         let result = handle
//             .incoming_tx
//             .send(tungstenite::Message::from(
//                 SignalingMessage::serialize(&msg).unwrap(),
//             ))
//             .await;
//         assert!(result.is_ok());
//
//         let login_result = client.login("token1", Duration::from_millis(100)).await;
//         assert!(login_result.is_err());
//         assert_matches!(
//             login_result,
//             Err(SignalingError::LoginError(LoginFailureReason::DuplicateId))
//         );
//
//         let login_msg = tungstenite::Message::from(
//             SignalingMessage::serialize(&SignalingMessage::Login {
//                 token: "token1".to_string(),
//                 protocol_version: VACS_PROTOCOL_VERSION.to_string(),
//             })
//             .unwrap(),
//         );
//
//         let sent_message = handle.outgoing_rx.recv().await;
//         assert_eq!(sent_message, Some(login_msg));
//     }
//
//     #[test(tokio::test)]
//     async fn login_unexpected_message() {
//         let ((sender, receiver), mut handle) = mock::create();
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let mut client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//
//         let msg = SignalingMessage::CallAnswer {
//             peer_id: "client1".to_string(),
//             sdp: "sdp".to_string(),
//         };
//
//         let result = handle
//             .incoming_tx
//             .send(tungstenite::Message::from(
//                 SignalingMessage::serialize(&msg).unwrap(),
//             ))
//             .await;
//         assert!(result.is_ok());
//
//         let login_result = client.login("token1", Duration::from_millis(100)).await;
//         assert!(login_result.is_err());
//         assert_matches!(login_result, Err(SignalingError::ProtocolError(_)));
//
//         let login_msg = tungstenite::Message::from(
//             SignalingMessage::serialize(&SignalingMessage::Login {
//                 token: "token1".to_string(),
//                 protocol_version: VACS_PROTOCOL_VERSION.to_string(),
//             })
//             .unwrap(),
//         );
//
//         let sent_message = handle.outgoing_rx.recv().await;
//         assert_eq!(sent_message, Some(login_msg));
//     }
//
//     #[test(tokio::test)]
//     async fn login_server_error() {
//         let ((sender, receiver), mut handle) = mock::create();
//         let (_shutdown_tx, shutdown_rx) = watch::channel(());
//         let (ready_tx, ready_rx) = oneshot::channel();
//         let mut client = SignalingClientInner::new(shutdown_rx);
//         let mut client_clone = client.clone();
//
//         tokio::spawn(async move {
//             client_clone.start(sender, receiver, ready_tx).await;
//         });
//         assert!(ready_rx.await.is_ok());
//
//         let msg = SignalingMessage::Error {
//             reason: ErrorReason::Internal("something failed".to_string()),
//             peer_id: None,
//         };
//
//         let result = handle
//             .incoming_tx
//             .send(tungstenite::Message::from(
//                 SignalingMessage::serialize(&msg).unwrap(),
//             ))
//             .await;
//         assert!(result.is_ok());
//
//         let login_result = client.login("token1", Duration::from_millis(100)).await;
//         assert!(login_result.is_err());
//         assert_matches!(login_result, Err(SignalingError::ServerError(_)));
//
//         let login_msg = tungstenite::Message::from(
//             SignalingMessage::serialize(&SignalingMessage::Login {
//                 token: "token1".to_string(),
//                 protocol_version: VACS_PROTOCOL_VERSION.to_string(),
//             })
//             .unwrap(),
//         );
//
//         let sent_message = handle.outgoing_rx.recv().await;
//         assert_eq!(sent_message, Some(login_msg));
//     }
// }
