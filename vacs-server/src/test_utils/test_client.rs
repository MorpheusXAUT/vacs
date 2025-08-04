use crate::test_utils::connect_to_websocket;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use vacs_protocol::ws::{ClientInfo, SignalingMessage};

pub struct TestClient {
    id: String,
    token: String,
    ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl TestClient {
    pub async fn new(ws_addr: &str, id: &str, token: &str) -> anyhow::Result<Self> {
        let ws_stream = connect_to_websocket(ws_addr).await;
        Ok(Self {
            id: id.to_string(),
            token: token.to_string(),
            ws_stream,
        })
    }

    pub async fn new_with_login<F>(
        ws_addr: &str,
        id: &str,
        token: &str,
        client_list_predicate: F,
    ) -> anyhow::Result<Self>
    where
        F: FnOnce(&[ClientInfo]) -> anyhow::Result<()>,
    {
        let mut client = Self::new(ws_addr, id, token).await?;
        client.login(client_list_predicate).await?;
        Ok(client)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub async fn login<F>(&mut self, client_list_predicate: F) -> anyhow::Result<()>
    where
        F: FnOnce(&[ClientInfo]) -> anyhow::Result<()>,
    {
        let login_msg = SignalingMessage::Login {
            token: self.token.to_string(),
        };
        self.send_and_expect_with_timeout(login_msg, Duration::from_millis(100), |msg| match msg {
            SignalingMessage::ClientList { clients } => client_list_predicate(&clients),
            SignalingMessage::LoginFailure { reason } => {
                Err(anyhow::anyhow!("Login failed: {:?}", reason))
            }
            _ => Err(anyhow::anyhow!("Unexpected response: {:?}", msg)),
        })
        .await
    }

    pub async fn send_raw(&mut self, msg: Message) -> anyhow::Result<()> {
        self.ws_stream.send(msg).await?;
        Ok(())
    }

    pub async fn send(&mut self, msg: SignalingMessage) -> anyhow::Result<()> {
        self.ws_stream
            .send(Message::from(SignalingMessage::serialize(&msg)?))
            .await?;
        Ok(())
    }

    pub async fn recv_raw_with_timeout(&mut self, timeout: Duration) -> Option<Message> {
        loop {
            match tokio::time::timeout(timeout, self.ws_stream.next()).await {
                Ok(Some(Ok(Message::Ping(_)))) => continue,
                Ok(Some(Ok(message))) => return Some(message),
                _ => return None,
            }
        }
    }

    pub async fn recv_raw_until_timeout(&mut self, timeout: Duration) -> Vec<Message> {
        let mut messages = Vec::new();
        while let Some(message) = self.recv_raw_with_timeout(timeout).await {
            messages.push(message);
        }
        messages
    }

    pub async fn recv_raw(&mut self) -> Option<Message> {
        self.recv_raw_with_timeout(Duration::MAX).await
    }

    pub async fn recv_with_timeout(&mut self, timeout: Duration) -> Option<SignalingMessage> {
        loop {
            match self.recv_raw_with_timeout(timeout).await {
                Some(Message::Text(text)) => return SignalingMessage::deserialize(&text).ok(),
                Some(Message::Ping(_)) => continue,
                _ => return None,
            }
        }
    }

    pub async fn recv_until_timeout(&mut self, timeout: Duration) -> Vec<SignalingMessage> {
        let mut messages = Vec::new();
        while let Some(message) = self.recv_with_timeout(timeout).await {
            messages.push(message);
        }
        messages
    }

    pub async fn recv_until_timeout_with_filter<F>(
        &mut self,
        timeout: Duration,
        predicate: F,
    ) -> Vec<SignalingMessage>
    where
        F: Fn(&SignalingMessage) -> bool,
    {
        let mut messages = Vec::new();
        while let Some(message) = self.recv_with_timeout(timeout).await {
            if predicate(&message) {
                messages.push(message);
            }
        }
        messages
    }

    pub async fn recv(&mut self) -> Option<SignalingMessage> {
        self.recv_with_timeout(Duration::MAX).await
    }

    pub async fn send_raw_and_expect_with_timeout<F>(
        &mut self,
        msg: Message,
        timeout: Duration,
        predicate: F,
    ) -> anyhow::Result<()>
    where
        F: FnOnce(Message),
    {
        self.send_raw(msg).await?;
        match self.recv_raw_with_timeout(timeout).await {
            Some(response) => predicate(response),
            None => anyhow::bail!("No response received"),
        }
        Ok(())
    }

    pub async fn send_raw_and_expect<F>(&mut self, msg: Message, predicate: F) -> anyhow::Result<()>
    where
        F: FnOnce(Message),
    {
        self.send_raw_and_expect_with_timeout(msg, Duration::MAX, predicate)
            .await
    }

    pub async fn send_and_expect_with_timeout<F>(
        &mut self,
        msg: SignalingMessage,
        timeout: Duration,
        predicate: F,
    ) -> anyhow::Result<()>
    where
        F: FnOnce(SignalingMessage) -> anyhow::Result<()>,
    {
        self.send(msg).await?;
        match self.recv_with_timeout(timeout).await {
            Some(response) => predicate(response),
            None => anyhow::bail!("No response received"),
        }
    }

    pub async fn send_and_expect<F>(
        &mut self,
        msg: SignalingMessage,
        predicate: F,
    ) -> anyhow::Result<()>
    where
        F: FnOnce(SignalingMessage) -> anyhow::Result<()>,
    {
        self.send_and_expect_with_timeout(msg, Duration::MAX, predicate)
            .await
    }

    pub async fn close(&mut self) {
        self.ws_stream
            .close(None)
            .await
            .expect("Failed to close websocket");
    }
}

#[allow(unused)]
pub async fn setup_test_clients(
    addr: &str,
    clients: &[(&str, &str)],
) -> HashMap<String, TestClient> {
    let mut test_clients = HashMap::new();
    for (id, token) in clients {
        let client = TestClient::new_with_login(addr, id, token, |clients| Ok(()))
            .await
            .expect("Failed to create test client");
        test_clients.insert(client.id.clone(), client);
    }
    test_clients
}

#[allow(unused)]
pub async fn setup_n_test_clients(addr: &str, num_clients: usize) -> Vec<TestClient> {
    let mut test_clients = Vec::new();
    for n in 1..=num_clients {
        let client = TestClient::new_with_login(
            addr,
            &format!("client{n}"),
            &format!("token{n}"),
            |clients| Ok(()),
        )
        .await
        .expect("Failed to create test client");
        test_clients.push(client);
    }
    test_clients
}
