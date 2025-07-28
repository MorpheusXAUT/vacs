use crate::client::SignalingClient;
use crate::transport::tokio::TokioTransport;
use tokio::sync::watch;
use vacs_server::test_utils::TestApp;

pub struct TestRig {
    server: TestApp,
    clients: Vec<SignalingClient<TokioTransport>>,
    shutdown_tx: watch::Sender<()>,
}

impl TestRig {
    pub async fn new(num_clients: usize) -> anyhow::Result<Self> {
        let server = TestApp::new().await;
        let (shutdown_tx, _) = watch::channel(());

        let mut clients = Vec::with_capacity(num_clients);
        for i in 0..num_clients {
            let mut client = SignalingClient::new(
                TokioTransport::new(server.addr()).await?,
                shutdown_tx.subscribe(),
            );
            client
                .login(
                    format!("token{}", i).as_str(),
                )
                .await?;
            clients.push(client);
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

    pub fn client(&self, index: usize) -> &SignalingClient<TokioTransport> {
        assert!(
            index < self.clients.len(),
            "Client index {} out of bounds",
            index
        );
        &self.clients[index]
    }

    pub fn client_mut(&mut self, index: usize) -> &mut SignalingClient<TokioTransport> {
        assert!(
            index < self.clients.len(),
            "Client index {} out of bounds",
            index
        );
        &mut self.clients[index]
    }

    pub fn clients(&self) -> &[SignalingClient<TokioTransport>] {
        &self.clients
    }

    pub fn clients_mut(&mut self) -> &mut [SignalingClient<TokioTransport>] {
        &mut self.clients
    }

    pub fn shutdown(&self) {
        self.shutdown_tx.send(()).unwrap();
    }
}

impl Drop for TestRig {
    fn drop(&mut self) {
        self.shutdown();
    }
}
