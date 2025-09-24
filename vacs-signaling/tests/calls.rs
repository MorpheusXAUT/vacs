use std::time::Duration;
use test_log::test;
use vacs_protocol::ws::SignalingMessage;
use vacs_signaling::client::SignalingEvent;
use vacs_signaling::test_utils::TestRig;

#[test(tokio::test)]
async fn call_offer_answer() {
    let mut test_rig = TestRig::new(2).await;

    let clients = test_rig.clients_mut();

    clients[0]
        .client
        .send(SignalingMessage::CallOffer {
            peer_id: "client1".to_string(),
            sdp: "sdp0".to_string(),
        })
        .await
        .unwrap();

    let event = clients[1]
        .recv_with_timeout_and_filter(Duration::from_millis(100), |e| {
            matches!(e, SignalingEvent::Message(SignalingMessage::CallOffer {
                peer_id,
                sdp
            }) if peer_id == "client0" && sdp == "sdp0")
        })
        .await;
    assert!(event.is_some());

    clients[1]
        .client
        .send(SignalingMessage::CallAnswer {
            peer_id: "client0".to_string(),
            sdp: "sdp1".to_string(),
        })
        .await
        .unwrap();

    let event = clients[0]
        .recv_with_timeout_and_filter(Duration::from_millis(100), |e| {
            matches!(e, SignalingEvent::Message(SignalingMessage::CallAnswer {
                peer_id,
                sdp
            }) if peer_id == "client1" && sdp == "sdp1")
        })
        .await;
    assert!(event.is_some());
}
