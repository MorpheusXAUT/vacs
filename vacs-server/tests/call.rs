mod common;

use crate::common::{TestApp, setup_n_test_clients};
use std::time::Duration;
use test_log::test;
use vacs_core::signaling::Message;

#[test(tokio::test)]
async fn call_offer() -> anyhow::Result<()> {
    let test_app = TestApp::new().await;
    let mut clients = setup_n_test_clients(test_app.addr(), 5).await;

    let mut client1 = clients.remove(0);
    let mut client2 = clients.remove(0);

    client1
        .send(Message::CallOffer {
            peer_id: client2.id.clone(),
            sdp: "sdp1".to_string(),
        })
        .await?;

    let call_offer_messages = client2
        .receive_until_timeout_with_filter(Duration::from_millis(100), |m| {
            matches!(m, Message::CallOffer { .. })
        })
        .await;

    assert_eq!(
        call_offer_messages.len(),
        1,
        "client2 should have received exactly one CallOffer message"
    );

    match &call_offer_messages[0] {
        Message::CallOffer { peer_id, sdp } => {
            assert_eq!(peer_id, &client1.id, "CallOffer targeted the wrong client");
            assert_eq!(sdp, "sdp1", "CallOffer contains the wrong SDP");
        }
        message => panic!(
            "Unexpected message: {:?}, expected CallOffer from client1",
            message
        ),
    };

    for (i, client) in clients.iter_mut().enumerate() {
        let call_offer_messages = client
            .receive_until_timeout_with_filter(Duration::from_millis(100), |m| {
                matches!(m, Message::CallOffer { .. })
            })
            .await;

        assert!(
            call_offer_messages.is_empty(),
            "client{} should have received no messages, but received: {:?}",
            i + 3,
            call_offer_messages
        );
    }

    let call_offer_messages = client1
        .receive_until_timeout_with_filter(Duration::from_millis(100), |m| {
            matches!(m, Message::CallOffer { .. })
        })
        .await;
    assert!(
        call_offer_messages.is_empty(),
        "client1 should have received no messages, but received: {:?}",
        call_offer_messages
    );

    Ok(())
}

#[test(tokio::test)]
async fn call_offer_answer() -> anyhow::Result<()> {
    let test_app = TestApp::new().await;
    let mut clients = setup_n_test_clients(test_app.addr(), 5).await;

    let mut client1 = clients.remove(0);
    let mut client2 = clients.remove(0);

    client1
        .send(Message::CallOffer {
            peer_id: client2.id.clone(),
            sdp: "sdp1".to_string(),
        })
        .await?;

    let call_offer_messages = client2
        .receive_until_timeout_with_filter(Duration::from_millis(100), |m| {
            matches!(m, Message::CallOffer { .. })
        })
        .await;

    assert_eq!(
        call_offer_messages.len(),
        1,
        "client2 should have received exactly one CallOffer message"
    );

    match &call_offer_messages[0] {
        Message::CallOffer { peer_id, sdp } => {
            assert_eq!(peer_id, &client1.id, "CallOffer targeted the wrong client");
            assert_eq!(sdp, "sdp1", "CallOffer contains the wrong SDP");
        }
        message => panic!(
            "Unexpected message: {:?}, expected CallOffer from client1",
            message
        ),
    };

    client2
        .send(Message::CallAnswer {
            peer_id: client1.id.clone(),
            sdp: "sdp2".to_string(),
        })
        .await?;

    let call_answer_messages = client1
        .receive_until_timeout_with_filter(Duration::from_millis(100), |m| {
            matches!(m, Message::CallAnswer { .. })
        })
        .await;

    assert_eq!(
        call_answer_messages.len(),
        1,
        "client1 should have received exactly one CallAnswer message"
    );

    match &call_answer_messages[0] {
        Message::CallAnswer { peer_id, sdp } => {
            assert_eq!(peer_id, &client2.id, "CallAnswer targeted the wrong client");
            assert_eq!(sdp, "sdp2", "CallAnswer contains the wrong SDP");
        }
        message => panic!(
            "Unexpected message: {:?}, expected CallAnswer from client2",
            message
        ),
    };

    for (i, client) in clients.iter_mut().enumerate() {
        let messages = client
            .receive_until_timeout_with_filter(Duration::from_millis(100), |m| {
                matches!(m, Message::CallOffer { .. } | Message::CallAnswer { .. })
            })
            .await;

        assert!(
            messages.is_empty(),
            "client{} should have received no messages, but received: {:?}",
            i + 3,
            messages
        );
    }

    let call_offer_messages = client1
        .receive_until_timeout_with_filter(Duration::from_millis(100), |m| {
            matches!(m, Message::CallOffer { .. })
        })
        .await;
    assert!(
        call_offer_messages.is_empty(),
        "client1 should have received no messages, but received: {:?}",
        call_offer_messages
    );

    let call_answer_messages = client2
        .receive_until_timeout_with_filter(Duration::from_millis(100), |m| {
            matches!(m, Message::CallAnswer { .. })
        })
        .await;
    assert!(
        call_answer_messages.is_empty(),
        "client2 should have received no messages, but received: {:?}",
        call_answer_messages
    );

    Ok(())
}

#[test(tokio::test)]
async fn peer_not_found() -> anyhow::Result<()> {
    let test_app = TestApp::new().await;
    let mut clients = setup_n_test_clients(test_app.addr(), 5).await;

    let mut client1 = clients.remove(0);
    let mut client2 = clients.remove(0);

    client1
        .send(Message::CallOffer {
            peer_id: "client69".to_string(),
            sdp: "sdp1".to_string(),
        })
        .await?;

    let call_offer_messages = client2
        .receive_until_timeout_with_filter(Duration::from_millis(100), |m| {
            matches!(m, Message::CallOffer { .. })
        })
        .await;

    assert!(
        call_offer_messages.is_empty(),
        "client2 should have received no messages, but received: {:?}",
        call_offer_messages
    );

    let peer_not_found_messages = client1
        .receive_until_timeout_with_filter(Duration::from_millis(100), |m| {
            matches!(m, Message::PeerNotFound { .. })
        })
        .await;

    assert_eq!(
        peer_not_found_messages.len(),
        1,
        "client1 should have received exactly one PeerNotFound message"
    );

    match &peer_not_found_messages[0] {
        Message::PeerNotFound { peer_id } => {
            assert_eq!(
                peer_id, "client69",
                "PeerNotFound targeted the wrong client"
            );
        }
        message => panic!(
            "Unexpected message: {:?}, expected PeerNotFound for client2",
            message
        ),
    };

    for (i, client) in clients.iter_mut().enumerate() {
        let call_offer_messages = client
            .receive_until_timeout_with_filter(Duration::from_millis(100), |m| {
                matches!(m, Message::CallOffer { .. } | Message::PeerNotFound { .. })
            })
            .await;

        assert!(
            call_offer_messages.is_empty(),
            "client{} should have received no messages, but received: {:?}",
            i + 3,
            call_offer_messages
        );
    }

    Ok(())
}
