use pretty_assertions::{assert_eq, assert_matches};
use std::time::Duration;
use test_log::test;
use tokio::sync::watch;
use vacs_protocol::ws::{ClientInfo, LoginFailureReason, SignalingMessage};
use vacs_server::test_utils::{TestApp, TestClient};
use vacs_signaling::client;
use vacs_signaling::error::SignalingError;
use vacs_signaling::test_utils::TestRig;
use vacs_signaling::transport;

#[test(tokio::test)]
async fn login_without_self() {
    let test_app = TestApp::new().await;

    let transport = transport::tokio::TokioTransport::new(test_app.addr())
        .await
        .expect("Failed to create transport");
    let (shutdown_tx, shutdown_rx) = watch::channel(());
    let mut client = client::SignalingClient::builder(transport, shutdown_rx)
        .with_login_timeout(Duration::from_millis(100))
        .build();

    let res = client.login("token1").await;
    assert!(res.is_ok());
    assert_eq!(
        res.unwrap(),
        vec![]
    );

    shutdown_tx.send(()).unwrap();
}

#[test(tokio::test)]
async fn login() {
    let test_app = TestApp::new().await;

    let transport1 = transport::tokio::TokioTransport::new(test_app.addr())
        .await
        .expect("Failed to create transport");
    let (shutdown_tx1, shutdown_rx1) = watch::channel(());
    let mut client1 = client::SignalingClient::builder(transport1, shutdown_rx1)
        .with_login_timeout(Duration::from_millis(100))
        .build();

    let res1 = client1.login("token1").await;
    assert!(res1.is_ok());
    assert_eq!(
        res1.unwrap(),
        vec![]
    );

    let transport2 = transport::tokio::TokioTransport::new(test_app.addr())
        .await
        .expect("Failed to create transport");
    let (shutdown_tx2, shutdown_rx2) = watch::channel(());
    let mut client2 = client::SignalingClient::builder(transport2, shutdown_rx2)
        .with_login_timeout(Duration::from_millis(100))
        .build();

    let res2 = client2.login("token2").await;
    assert!(res2.is_ok());
    assert_eq!(
        res2.unwrap(),
        vec![ClientInfo {
            id: "client1".to_string(),
            display_name: "client1".to_string()
        }]
    );

    shutdown_tx1.send(()).unwrap();
    shutdown_tx2.send(()).unwrap();
}

#[test(tokio::test)]
#[cfg_attr(target_os = "windows", ignore)]
async fn login_timeout() {
    let test_app = TestApp::new().await;

    let transport = transport::tokio::TokioTransport::new(test_app.addr())
        .await
        .expect("Failed to create transport");
    let (shutdown_tx, shutdown_rx) = watch::channel(());
    let mut client = client::SignalingClient::builder(transport, shutdown_rx)
        .with_login_timeout(Duration::from_millis(100))
        .build();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let res = client.login("token1").await;
    assert!(res.is_err());
    assert_matches!(
        res.unwrap_err(),
        SignalingError::LoginError(LoginFailureReason::Timeout)
    );

    shutdown_tx.send(()).unwrap();
}

#[test(tokio::test)]
async fn login_invalid_credentials() {
    let test_app = TestApp::new().await;

    let transport = transport::tokio::TokioTransport::new(test_app.addr())
        .await
        .expect("Failed to create transport");
    let (shutdown_tx, shutdown_rx) = watch::channel(());
    let mut client = client::SignalingClient::builder(transport, shutdown_rx)
        .with_login_timeout(Duration::from_millis(100))
        .build();

    let res = client.login("").await;
    assert!(res.is_err());
    assert_matches!(
        res.unwrap_err(),
        SignalingError::LoginError(LoginFailureReason::InvalidCredentials)
    );

    shutdown_tx.send(()).unwrap();
}

#[test(tokio::test)]
async fn login_duplicate_id() {
    let test_rig = TestRig::new(1).await.unwrap();

    let transport = transport::tokio::TokioTransport::new(test_rig.server().addr())
        .await
        .expect("Failed to create transport");
    let (_shutdown_tx, shutdown_rx) = watch::channel(());
    let mut client = client::SignalingClient::builder(transport, shutdown_rx)
        .with_login_timeout(Duration::from_millis(100))
        .build();

    let res = client.login("token0").await;
    assert!(res.is_err());
    assert_matches!(
        res.unwrap_err(),
        SignalingError::LoginError(LoginFailureReason::DuplicateId)
    );
}

#[test(tokio::test)]
async fn logout() {
    let mut test_rig = TestRig::new(1).await.unwrap();
    let client = test_rig.client_mut(0);

    let res = client.send(SignalingMessage::Logout).await;
    assert!(res.is_ok());
}

#[test(tokio::test)]
async fn login_multiple_clients() {
    let test_rig = TestRig::new(5).await.unwrap();

    for i in 0..5 {
        let client = test_rig.client(i);
        let (is_connected, is_logged_in) = client.status();
        assert!(is_connected);
        assert!(is_logged_in);
    }
}

#[test(tokio::test)]
async fn client_disconnects() {
    let mut test_rig = TestRig::new(2).await.unwrap();

    let res = test_rig.client_mut(0).logout().await;
    assert!(res.is_ok());

    tokio::time::sleep(Duration::from_millis(50)).await;

    let (is_connected, is_logged_in) = test_rig.client(0).status();
    assert!(!is_connected);
    assert!(!is_logged_in);

    let msg = test_rig
        .client_mut(1)
        .recv_with_timeout(Duration::from_millis(100))
        .await
        .unwrap();
    assert_matches!(
        msg,
        SignalingMessage::ClientDisconnected { id } if id == "client0"
    );
}

#[test(tokio::test)]
async fn client_list_synchronization() {
    let mut test_rig = TestRig::new(3).await.unwrap();

    let res = test_rig.client_mut(0).logout().await;
    assert!(res.is_ok());

    tokio::time::sleep(Duration::from_millis(50)).await;

    let (is_connected, is_logged_in) = test_rig.client(0).status();
    assert!(!is_connected);
    assert!(!is_logged_in);

    let msg = test_rig
        .client_mut(2)
        .recv_with_timeout(Duration::from_millis(100))
        .await;
    assert_matches!(
        msg.unwrap(),
        SignalingMessage::ClientDisconnected { id } if id == "client0"
    );

    test_rig
        .client_mut(2)
        .send(SignalingMessage::ListClients)
        .await
        .unwrap();

    let msg = test_rig
        .client_mut(2)
        .recv_with_timeout(Duration::from_millis(100))
        .await;
    assert_matches!(
        msg.unwrap(),
        SignalingMessage::ClientList { clients } if clients.len() == 1 && clients[0].id == "client1"
    );
}

#[test(tokio::test)]
async fn client_connected_broadcast() {
    let mut test_rig = TestRig::new(3).await.unwrap();

    let mut client3 = TestClient::new(&test_rig.server().addr(), "client3", "token3")
        .await
        .unwrap();
    client3.login(|_| Ok(())).await.unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    let clients = test_rig.clients_mut();
    for (i, client) in clients.iter_mut().enumerate() {
        let mut received_client_ids = vec![];
        while let Ok(msg) = client.recv_with_timeout(Duration::from_millis(100)).await {
            match msg {
                SignalingMessage::ClientConnected { client } => {
                    received_client_ids.push(client.id.clone());
                }
                _ => panic!("Unexpected message: {:?}", msg),
            }
        }

        let expected_ids: Vec<_> = (i + 1..=3).map(|i| format!("client{}", i)).collect();
        assert_eq!(
            received_client_ids,
            expected_ids,
            "Client{} did not receive expected broadcasts: {:?}",
            i + 1,
            received_client_ids
        );
    }
}
