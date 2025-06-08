mod common;

use crate::common::{
    TestApp, TestClient, assert_message_matches, assert_raw_message_matches, connect_to_websocket,
    setup_test_clients,
};
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio_tungstenite::tungstenite;
use vacs_shared::signaling;

#[tokio::test]
async fn login() {
    let test_app = TestApp::new().await;

    let _client1 = TestClient::new(test_app.addr(), "client1", "token1", |clients| {
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].id, "client1");
        assert_eq!(clients[0].display_name, "client1");
    })
    .await
    .expect("Failed to log in first client");

    let _client2 = TestClient::new(test_app.addr(), "client2", "token2", |clients| {
        assert_eq!(clients.len(), 2);
        assert!(clients.iter().any(|client| client.id == "client1"));
        assert!(clients.iter().any(|client| client.id == "client2"));
    })
    .await
    .expect("Failed to log in second client");
}

#[tokio::test]
async fn duplicate_login() {
    let test_app = TestApp::new().await;

    let _client1 = TestClient::new(test_app.addr(), "client1", "token1", |clients| {
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].id, "client1");
        assert_eq!(clients[0].display_name, "client1");
    })
    .await
    .expect("Failed to log in first client");

    assert!(
        TestClient::new(test_app.addr(), "client1", "token1", |_| {})
            .await
            .is_err_and(|err| { err.to_string() == "Login failed: IdTaken" })
    );
}

#[tokio::test]
async fn invalid_login() {
    let test_app = TestApp::new().await;

    assert!(
        TestClient::new(test_app.addr(), "client1", "", |_| {})
            .await
            .is_err_and(|err| { err.to_string() == "Login failed: InvalidCredentials" })
    );
}

#[tokio::test]
async fn unauthorized_message_before_login() {
    let test_app = TestApp::new().await;

    let mut ws_stream = connect_to_websocket(test_app.addr()).await;

    ws_stream
        .send(tungstenite::Message::from(
            signaling::Message::serialize(&signaling::Message::ListClients).unwrap(),
        ))
        .await
        .expect("Failed to send ListClients message");

    let message_result = ws_stream.next().await;
    assert_raw_message_matches(message_result, |response| match response {
        signaling::Message::LoginFailure { reason } => {
            assert_eq!(
                reason,
                signaling::LoginFailureReason::InvalidLoginFlow,
                "Unexpected reason for LoginFailure"
            );
        }
        _ => panic!("Unexpected response: {:?}", response),
    });
}

#[tokio::test]
async fn simultaneous_login_attempts() {
    let test_app = TestApp::new().await;

    let attempt1 = TestClient::new(test_app.addr(), "client1", "token1", |_| {});
    let attempt2 = TestClient::new(test_app.addr(), "client1", "token1", |_| {});

    let (attempt1_result, attempt2_result) = tokio::join!(attempt1, attempt2);

    assert!(
        (attempt1_result.is_ok() && attempt2_result.is_err())
            || (attempt1_result.is_err() && attempt2_result.is_ok()),
        "Expected one attempt to succeed and one to fail with IdTaken"
    );
}

#[tokio::test]
async fn login_timeout() {
    let test_app = TestApp::new().await;

    let mut ws_stream = connect_to_websocket(test_app.addr()).await;

    tokio::time::sleep(Duration::from_millis(
        test_app.state.config.auth.login_flow_timeout_millis,
    ))
    .await;

    ws_stream
        .send(tungstenite::Message::from(
            signaling::Message::serialize(&signaling::Message::Login {
                id: "client1".to_string(),
                token: "token".to_string(),
            })
            .unwrap(),
        ))
        .await
        .expect("Failed to send login message");

    let message_result = ws_stream.next().await;
    assert!(message_result.is_none() || matches!(message_result, Some(Err(_))));
}

#[tokio::test]
async fn client_connected() {
    let test_app = TestApp::new().await;

    let mut clients = setup_test_clients(
        test_app.addr(),
        &vec![("client1", "token1"), ("client2", "token2")],
    )
    .await;

    let client1 = clients.get_mut("client1").unwrap();
    let client_connected = client1
        .receive_with_timeout(Duration::from_millis(100))
        .await;
    assert_message_matches(client_connected, |message| match message {
        signaling::Message::ClientConnected { client } => {
            assert_eq!(client.id, "client2");
            assert_eq!(client.display_name, "client2");
        }
        _ => panic!("Unexpected message: {:?}", message),
    });

    let client2 = clients.get_mut("client2").unwrap();
    assert!(
        client2
            .receive_with_timeout(Duration::from_millis(100))
            .await
            .is_none()
    );
}

#[tokio::test]
async fn client_disconnected() {
    let test_app = TestApp::new().await;

    let mut clients = setup_test_clients(
        test_app.addr(),
        &vec![("client1", "token1"), ("client2", "token2")],
    )
    .await;

    let client1 = clients.get_mut("client1").unwrap();
    let client_connected = client1
        .receive_with_timeout(Duration::from_millis(100))
        .await;
    assert_message_matches(client_connected, |message| match message {
        signaling::Message::ClientConnected { client } => {
            assert_eq!(client.id, "client2");
            assert_eq!(client.display_name, "client2");
        }
        _ => panic!("Unexpected message: {:?}", message),
    });

    client1.close().await;

    let client2 = clients.get_mut("client2").unwrap();
    let client_disconnected = client2
        .receive_with_timeout(Duration::from_millis(100))
        .await;
    assert_message_matches(client_disconnected, |message| match message {
        signaling::Message::ClientDisconnected { id } => assert_eq!(id, "client1"),
        _ => panic!("Unexpected message: {:?}", message),
    });
}

#[tokio::test]
async fn login_client_list() {
    let test_app = TestApp::new().await;

    let _clients = setup_test_clients(
        test_app.addr(),
        &vec![
            ("client1", "token1"),
            ("client2", "token2"),
            ("client3", "token3"),
        ],
    )
    .await;

    let _client4 = TestClient::new(test_app.addr(), "client4", "token4", |clients| {
        assert_eq!(clients.len(), 4);
        assert!(clients.iter().any(|client| client.id == "client1"));
        assert!(clients.iter().any(|client| client.id == "client2"));
        assert!(clients.iter().any(|client| client.id == "client3"));
        assert!(clients.iter().any(|client| client.id == "client4"));
    })
    .await
    .expect("Failed to log in fourth client");
}

#[tokio::test]
async fn logout() {
    let test_app = TestApp::new().await;

    let mut clients = setup_test_clients(
        test_app.addr(),
        &vec![("client1", "token1"), ("client2", "token2")],
    )
    .await;

    let client1 = clients.get_mut("client1").unwrap();
    let client_connected = client1
        .receive_with_timeout(Duration::from_millis(100))
        .await;
    assert_message_matches(client_connected, |message| match message {
        signaling::Message::ClientConnected { client } => {
            assert_eq!(client.id, "client2");
            assert_eq!(client.display_name, "client2");
        }
        _ => panic!("Unexpected message: {:?}", message),
    });

    client1.send(signaling::Message::Logout).await.unwrap();
    assert!(
        client1
            .receive_with_timeout(Duration::from_millis(100))
            .await
            .is_none()
    );

    let client2 = clients.get_mut("client2").unwrap();
    let client_disconnected = client2
        .receive_with_timeout(Duration::from_millis(100))
        .await;
    assert_message_matches(client_disconnected, |message| match message {
        signaling::Message::ClientDisconnected { id } => assert_eq!(id, "client1"),
        _ => panic!("Unexpected message: {:?}", message),
    });
}
