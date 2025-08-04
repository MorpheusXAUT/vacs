use pretty_assertions::assert_eq;
use std::time::Duration;
use test_log::test;
use vacs_protocol::ws::SignalingMessage;
use vacs_signaling::test_utils::TestRig;

#[test(tokio::test)]
async fn call_offer_answer() {
    let mut test_rig = TestRig::new(2).await.unwrap();

    let clients = test_rig.clients_mut();

    clients[0]
        .client
        .send(SignalingMessage::CallOffer {
            peer_id: "client1".to_string(),
            sdp: "sdp0".to_string(),
        })
        .await
        .unwrap();

    let msg = clients[1]
        .recv_with_timeout(Duration::from_millis(100))
        .await
        .unwrap();
    assert_eq!(
        msg,
        SignalingMessage::CallOffer {
            peer_id: "client0".to_string(),
            sdp: "sdp0".to_string()
        }
    );

    clients[1]
        .client
        .send(SignalingMessage::CallAnswer {
            peer_id: "client0".to_string(),
            sdp: "sdp1".to_string(),
        })
        .await
        .unwrap();

    // Skip the first message, will be the ClientConnected from client1.
    clients[0]
        .recv_with_timeout(Duration::from_millis(100))
        .await
        .unwrap();
    let msg = clients[0]
        .recv_with_timeout(Duration::from_millis(100))
        .await
        .unwrap();
    assert_eq!(
        msg,
        SignalingMessage::CallAnswer {
            peer_id: "client1".to_string(),
            sdp: "sdp1".to_string()
        }
    );
}
