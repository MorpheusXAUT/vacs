use crate::client::{InterruptionReason, SignalingClientInner};
use crate::transport;
use std::time::Duration;
use tokio::sync::{broadcast, oneshot, watch};
use tokio::task::JoinHandle;
use vacs_protocol::ws::SignalingMessage;
use vacs_server::test_utils::TestApp;

pub struct TestRigClient {
    pub client: SignalingClientInner,
    pub task: JoinHandle<()>,
    pub interrupt_rx: oneshot::Receiver<InterruptionReason>,
    pub broadcast_rx: broadcast::Receiver<SignalingMessage>,
}

impl TestRigClient {
    pub async fn recv_with_timeout(&mut self, timeout: Duration) -> Option<SignalingMessage> {
        match tokio::time::timeout(timeout, self.broadcast_rx.recv()).await {
            Ok(Ok(msg)) => Some(msg),
            _ => None,
        }
    }
}

pub struct TestRig {
    server: TestApp,
    clients: Vec<TestRigClient>,
    shutdown_tx: watch::Sender<()>,
}

impl TestRig {
    pub async fn new(num_clients: usize) -> anyhow::Result<Self> {
        let server = TestApp::new().await;
        let (shutdown_tx, _) = watch::channel(());

        let mut clients = Vec::with_capacity(num_clients);
        for i in 0..num_clients {
            let mut client = SignalingClientInner::new(shutdown_tx.subscribe());
            let mut client_clone = client.clone();
            let (sender, receiver) = transport::tokio::create(server.addr()).await?;
            let (ready_tx, ready_rx) = oneshot::channel();
            let (interrupt_tx, interrupt_rx) = oneshot::channel();
            let task = tokio::spawn(async move {
                let reason = client_clone.connect(sender, receiver, ready_tx).await;
                interrupt_tx.send(reason).unwrap();
            });
            ready_rx.await.expect("Client failed to connect");
            client
                .login(format!("token{i}").as_str(), Duration::from_millis(100))
                .await?;
            let broadcast_rx = client.subscribe();
            clients.push(TestRigClient {
                client,
                task,
                interrupt_rx,
                broadcast_rx,
            });
        }

        Ok(Self {
            server,
            clients,
            shutdown_tx,
        })
    }

    pub fn server(&self) -> &TestApp {
        &self.server
    }

    pub fn client(&self, index: usize) -> &TestRigClient {
        assert!(
            index < self.clients.len(),
            "Client index {index} out of bounds",
        );
        &self.clients[index]
    }

    pub fn client_mut(&mut self, index: usize) -> &mut TestRigClient {
        assert!(
            index < self.clients.len(),
            "Client index {index} out of bounds",
        );
        &mut self.clients[index]
    }

    pub fn clients(&self) -> &[TestRigClient] {
        &self.clients
    }

    pub fn clients_mut(&mut self) -> &mut [TestRigClient] {
        &mut self.clients
    }

    pub fn shutdown(&self) {
        self.shutdown_tx.send(()).unwrap();
        for client in &self.clients {
            client.task.abort();
        }
    }
}

impl Drop for TestRig {
    fn drop(&mut self) {
        self.shutdown();
    }
}
