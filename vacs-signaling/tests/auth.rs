use pretty_assertions::assert_matches;
use std::time::Duration;
use test_log::test;
use tokio_util::sync::CancellationToken;
use vacs_protocol::ws::{LoginFailureReason, SignalingMessage};
use vacs_server::test_utils::{TestApp, TestClient};
use vacs_signaling::auth::mock::MockTokenProvider;
use vacs_signaling::client::{SignalingClient, SignalingEvent, State};
use vacs_signaling::error::SignalingError;
use vacs_signaling::test_utils::{RecvWithTimeoutExt, TestRig};
use vacs_signaling::transport::tokio::TokioTransport;

#[test(tokio::test)]
async fn login_without_self() {
    let test_app = TestApp::new().await;

    let transport = TokioTransport::new(test_app.addr());
    let token_provider = MockTokenProvider::new(1, None);
    let shutdown_token = CancellationToken::new();

    let client = SignalingClient::new(
        transport,
        token_provider,
        |_| async {},
        shutdown_token.clone(),
        Duration::from_millis(100),
        8,
        &tokio::runtime::Handle::current(),
    );

    let mut broadcast_rx = client.subscribe();
    let res = client.connect().await;
    let connected_event = broadcast_rx.recv_with_timeout(Duration::from_millis(100), |event|
        matches!(event, SignalingEvent::Connected{ client_info } if client_info.id == "client1" && client_info.display_name == "client1" && client_info.frequency == ""),
    ).await;
    let client_list_event = broadcast_rx.recv_with_timeout(Duration::from_millis(100), |event| matches!(event, SignalingEvent::Message(SignalingMessage::ClientList { clients }) if clients.is_empty())).await;

    assert!(res.is_ok());
    assert!(connected_event.is_ok());
    assert!(client_list_event.is_ok());

    shutdown_token.cancel();
    client.disconnect().await;
}

#[test(tokio::test)]
async fn login() {
    let test_app = TestApp::new().await;

    let transport1 = TokioTransport::new(test_app.addr());
    let token_provider1 = MockTokenProvider::new(1, None);
    let shutdown_token1 = CancellationToken::new();

    let client1 = SignalingClient::new(
        transport1,
        token_provider1,
        |_| async {},
        shutdown_token1.child_token(),
        Duration::from_millis(100),
        8,
        &tokio::runtime::Handle::current(),
    );

    let mut broadcast_rx1 = client1.subscribe();
    let res1 = client1.connect().await;
    let connected_event1 = broadcast_rx1.recv_with_timeout(Duration::from_millis(100), |event|
        matches!(event, SignalingEvent::Connected{ client_info } if client_info.id == "client1" && client_info.display_name == "client1" && client_info.frequency == ""),
    ).await;
    let client_list_event1 = broadcast_rx1.recv_with_timeout(Duration::from_millis(100), |event| matches!(event, SignalingEvent::Message(SignalingMessage::ClientList { clients }) if clients.is_empty())).await;

    assert!(res1.is_ok());
    assert!(connected_event1.is_ok());
    assert!(client_list_event1.is_ok());

    let transport2 = TokioTransport::new(test_app.addr());
    let token_provider2 = MockTokenProvider::new(2, None);
    let shutdown_token2 = CancellationToken::new();

    let client2 = SignalingClient::new(
        transport2,
        token_provider2,
        |_| async {},
        shutdown_token2.child_token(),
        Duration::from_millis(100),
        8,
        &tokio::runtime::Handle::current(),
    );

    let mut broadcast_rx2 = client2.subscribe();
    let res2 = client2.connect().await;
    let connected_event2 = broadcast_rx2.recv_with_timeout(Duration::from_millis(100), |event|
        matches!(event, SignalingEvent::Connected{ client_info } if client_info.id == "client2" && client_info.display_name == "client2" && client_info.frequency == ""),
    ).await;
    let client_list_event2 = broadcast_rx2.recv_with_timeout(Duration::from_millis(100), |event| matches!(event, SignalingEvent::Message(SignalingMessage::ClientList { clients }) if clients.len() == 1 && clients[0].id == "client1")).await;

    assert!(res2.is_ok());
    assert!(connected_event2.is_ok());
    assert!(client_list_event2.is_ok());

    shutdown_token1.cancel();
    client1.disconnect().await;
    shutdown_token2.cancel();
    client2.disconnect().await;
}

#[test(tokio::test)]
#[cfg_attr(target_os = "windows", ignore)]
async fn login_timeout() {
    let test_app = TestApp::new().await;

    let transport = TokioTransport::new(test_app.addr());
    let token_provider = MockTokenProvider::new(1, Some(Duration::from_millis(150)));
    let shutdown_token = CancellationToken::new();

    let client = SignalingClient::new(
        transport,
        token_provider,
        |_| async {},
        shutdown_token.clone(),
        Duration::from_millis(100),
        8,
        &tokio::runtime::Handle::current(),
    );

    let res = client.connect().await;

    assert!(res.is_err());
    assert_matches!(res.unwrap_err(), SignalingError::Timeout(reason) if reason == "Timeout waiting for message");

    shutdown_token.cancel();
    client.disconnect().await;
}

#[test(tokio::test)]
async fn login_invalid_credentials() {
    let test_app = TestApp::new().await;

    let transport = TokioTransport::new(test_app.addr());
    let token_provider = MockTokenProvider::new(usize::MAX, None);
    let shutdown_token = CancellationToken::new();

    let client = SignalingClient::new(
        transport,
        token_provider,
        |_| async {},
        shutdown_token.clone(),
        Duration::from_millis(100),
        8,
        &tokio::runtime::Handle::current(),
    );

    let res = client.connect().await;

    assert!(res.is_err());
    assert_matches!(
        res.unwrap_err(),
        SignalingError::LoginError(LoginFailureReason::InvalidCredentials)
    );

    shutdown_token.cancel();
    client.disconnect().await;
}

#[test(tokio::test)]
async fn login_duplicate_id() {
    let test_rig = TestRig::new(1).await;

    let transport = TokioTransport::new(test_rig.server().addr());
    let token_provider = MockTokenProvider::new(0, None);
    let shutdown_token = CancellationToken::new();

    let client = SignalingClient::new(
        transport,
        token_provider,
        |_| async {},
        shutdown_token.clone(),
        Duration::from_millis(100),
        8,
        &tokio::runtime::Handle::current(),
    );

    let res = client.connect().await;

    assert!(res.is_err());
    assert_matches!(
        res.unwrap_err(),
        SignalingError::LoginError(LoginFailureReason::DuplicateId)
    );

    shutdown_token.cancel();
    client.disconnect().await;
}

#[test(tokio::test)]
async fn logout() {
    let mut test_rig = TestRig::new(1).await;
    let client = test_rig.client_mut(0);

    let res = client.client.send(SignalingMessage::Logout).await;
    assert!(res.is_ok());
}

#[test(tokio::test)]
async fn login_multiple_clients() {
    let test_rig = TestRig::new(5).await;

    for i in 0..5 {
        let client = test_rig.client(i);
        let state = client.client.state();
        assert_matches!(state, State::LoggedIn);
    }
}

#[test(tokio::test)]
async fn client_disconnects() {
    let mut test_rig = TestRig::new(2).await;

    test_rig.client_mut(0).client.disconnect().await;

    let state = test_rig.client(0).client.state();
    assert_matches!(state, State::Disconnected);

    let event = test_rig
        .client_mut(1)
        .recv_with_timeout_and_filter(
            Duration::from_millis(300),
            |e| matches!(e, SignalingEvent::Message(SignalingMessage::ClientDisconnected { id }) if id == "client0"),
        )
        .await;
    assert!(event.is_some());
}

#[test(tokio::test)]
async fn client_list_synchronization() {
    let mut test_rig = TestRig::new(3).await;

    test_rig.client_mut(0).client.disconnect().await;

    let state = test_rig.client(0).client.state();
    assert_matches!(state, State::Disconnected);

    let event = test_rig
        .client_mut(2)
        .recv_with_timeout_and_filter(
            Duration::from_millis(300),
            |e| matches!(e, SignalingEvent::Message(SignalingMessage::ClientDisconnected { id }) if id == "client0"),
        )
        .await;
    assert!(event.is_some());

    test_rig
        .client_mut(2)
        .client
        .send(SignalingMessage::ListClients)
        .await
        .unwrap();

    let event = test_rig
        .client_mut(2)
        .recv_with_timeout_and_filter(
            Duration::from_millis(300),
            |e| matches!(e, SignalingEvent::Message(SignalingMessage::ClientList { clients }) if clients.len() == 1 && clients[0].id == "client1"),
        )
        .await;
    assert!(event.is_some());
}

#[test(tokio::test)]
async fn client_connected_broadcast() {
    let mut test_rig = TestRig::new(3).await;

    let mut client3 = TestClient::new(test_rig.server().addr(), "client3", "token3")
        .await
        .unwrap();
    client3.login(|_, _| Ok(()), |_| Ok(())).await.unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    let clients = test_rig.clients_mut();
    for (i, client) in clients.iter_mut().enumerate() {
        let mut received_client_ids = vec![];
        while let Some(msg) = client
            .recv_with_timeout_and_filter(Duration::from_millis(100), |e| {
                matches!(
                    e,
                    SignalingEvent::Message(SignalingMessage::ClientConnected { .. })
                )
            })
            .await
        {
            match msg {
                SignalingEvent::Message(SignalingMessage::ClientConnected { client }) => {
                    received_client_ids.push(client.id);
                }
                _ => panic!("Unexpected message: {msg:?}"),
            }
        }

        let expected_ids: Vec<_> = (i + 1..=3).map(|i| format!("client{i}")).collect();
        assert_eq!(
            received_client_ids,
            expected_ids,
            "Client{} did not receive expected broadcasts: {:?}",
            i + 1,
            received_client_ids
        );
    }
}
