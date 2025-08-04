use pretty_assertions::assert_eq;
use std::time::Duration;
use test_log::test;
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::tungstenite::Bytes;
use vacs_protocol::ws::SignalingMessage;
use vacs_server::test_utils::{TestApp, TestClient, setup_n_test_clients};

#[test(tokio::test)]
async fn client_connected() -> anyhow::Result<()> {
    let test_app = TestApp::new().await;
    let mut clients = setup_n_test_clients(test_app.addr(), 5).await;
    let client_count = clients.len();

    for (i, client) in clients.iter_mut().enumerate() {
        let messages = client.recv_until_timeout(Duration::from_millis(100)).await;

        let expected_message_count = client_count - i - 1;
        assert_eq!(
            messages.len(),
            expected_message_count,
            "Client{} did not receive expected number of messages",
            i + 1
        );

        let expected_ids: Vec<_> = (i + 2..=client_count)
            .map(|i| format!("client{i}"))
            .collect();

        for message in messages {
            if let SignalingMessage::ClientConnected { client } = message {
                assert!(
                    expected_ids.contains(&client.id),
                    "Unexpected client ID: {:?}, expected one of: {:?}",
                    client.id,
                    expected_ids
                );
            } else {
                panic!("Unexpected message: {message:?}");
            }
        }
    }

    Ok(())
}

#[test(tokio::test)]
async fn client_disconnected() -> anyhow::Result<()> {
    let test_app = TestApp::new().await;
    let mut clients = setup_n_test_clients(test_app.addr(), 5).await;
    let initial_client_count = clients.len();

    clients
        .last_mut()
        .unwrap()
        .send(SignalingMessage::Logout)
        .await
        .expect("Failed to send logout message");

    for (i, client) in clients.iter_mut().enumerate() {
        let messages = client.recv_until_timeout(Duration::from_millis(100)).await;

        let expected_message_count = if i == initial_client_count - 1 {
            0 // last client receives no login or logout messages
        } else {
            initial_client_count - i
        };

        assert_eq!(
            messages.len(),
            expected_message_count,
            "Client{} did not receive expected number of messages",
            i + 1
        );

        let expected_ids: Vec<_> = (i + 2..=initial_client_count)
            .map(|i| format!("client{i}"))
            .collect();

        for message in messages {
            match message {
                SignalingMessage::ClientConnected { client } => {
                    assert!(
                        expected_ids.contains(&client.id),
                        "Unexpected client ID: {:?}, expected one of: {:?}",
                        client.id,
                        expected_ids
                    );
                }
                SignalingMessage::ClientDisconnected { id } => {
                    assert_eq!(
                        id,
                        format!("client{initial_client_count}"),
                        "Unexpected client ID: {:?}",
                        id
                    );
                }
                message => {
                    panic!("Unexpected message: {message:?}");
                }
            }
        }
    }

    Ok(())
}

#[test(tokio::test)]
async fn client_dropped() -> anyhow::Result<()> {
    let test_app = TestApp::new().await;
    let mut clients = setup_n_test_clients(test_app.addr(), 5).await;
    let initial_client_count = clients.len();
    clients.pop();

    for (i, client) in clients.iter_mut().enumerate() {
        let messages = client.recv_until_timeout(Duration::from_millis(100)).await;

        let expected_message_count = initial_client_count - i;
        assert_eq!(
            messages.len(),
            expected_message_count,
            "Client{} did not receive expected number of messages",
            i + 1
        );

        let expected_ids: Vec<_> = (i + 2..=initial_client_count)
            .map(|i| format!("client{i}"))
            .collect();

        for message in messages {
            match message {
                SignalingMessage::ClientConnected { client } => {
                    assert!(
                        expected_ids.contains(&client.id),
                        "Unexpected client ID: {:?}, expected one of: {:?}",
                        client.id,
                        expected_ids
                    );
                }
                SignalingMessage::ClientDisconnected { id } => {
                    assert_eq!(
                        id,
                        format!("client{initial_client_count}"),
                        "Unexpected client ID: {:?}",
                        id
                    );
                }
                message => {
                    panic!("Unexpected message: {message:?}");
                }
            }
        }
    }

    Ok(())
}

#[test(tokio::test)]
async fn control_messages() -> anyhow::Result<()> {
    let test_app = TestApp::new().await;
    let mut client = TestClient::new_with_login(test_app.addr(), "client1", "token1", |_| Ok(()))
        .await
        .expect("Failed to create client");

    for n in 0..10 {
        client
            .send_raw_and_expect(
                tungstenite::Message::Ping(Bytes::from(format!("ping{n}"))),
                |message| {
                    assert_eq!(
                        message,
                        tungstenite::Message::Pong(Bytes::from(format!("ping{n}")))
                    );
                },
            )
            .await
            .expect("Expected server to respond to pings");
    }

    Ok(())
}
