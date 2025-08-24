use serde::{Deserialize, Serialize};

/// Possible reasons for a login failure.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum LoginFailureReason {
    /// The client has not authenticated properly yet, the login flow must be performed before sending any other messages.
    Unauthorized,
    /// The provided credentials are already in use.
    DuplicateId,
    /// The provided credentials are invalid.
    InvalidCredentials,
    /// The login flow was not completed in time, the client should reconnect and authenticate immediately.
    Timeout,
}

/// Possible reasons for a client or server error.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum ErrorReason {
    /// The message was malformed and could not be parsed.
    MalformedMessage,
    /// The message was processed successfully, but an internal error occurred.
    Internal(String),
    /// The message was processed successfully, but an error communicating with the selected peer occurred.
    PeerConnection,
    /// The client or server encountered an unexpected message.
    UnexpectedMessage(String),
}

/// Represents a client as observed by the signaling server.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    /// ID of the client.
    pub id: String,
    /// Display (station) name of the client.
    pub display_name: String,
}

/// Represents a message exchanged between the signaling server and clients.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum SignalingMessage {
    /// A login message sent by the client upon initial connection, providing an VATSIM access token.
    ///
    /// Upon successful login, a [`SignalingMessage::ClientList`] response will be returned, containing a list of all currently connected clients.
    ///
    /// Upon login failure (either due to the client's ID already being in use or due to an invalid auth token), a [`SignalingMessage::Error`] response will be returned.
    Login {
        /// VATSIM access token received from OAuth2 flow, used to authenticate client and retrieve the user's CID server-side.
        token: String,
    },
    /// A login failure message sent by the signaling server after a failed login attempt.
    LoginFailure {
        /// Reason for the login failure.
        reason: LoginFailureReason,
    },
    /// A logout message sent by the client upon disconnection.
    ///
    /// This performs a graceful logout, cleanly indicating a disconnect to the signaling server.
    /// However, the server will also perform a periodic `Ping` to ensure the connected clients are still alive, disconnecting them forcefully if necessary.
    Logout,
    /// A call invite message sent by the client to initiate a call with another client.
    ///
    /// The signaling server will forward the offer to the target client, exchanging the [`SignalingMessage::CallOffer::peer_id`] with the caller's ID.
    /// The target client will in turn prompt the user to accept or reject the call.
    ///
    /// Upon acceptance, the target client will reply with a [`SignalingMessage::CallAccept`] message,
    /// which is returned to the source client by the signaling server. After receiving the [`SignalingMessage::CallAccept`] message,
    /// the source client will create a WebRTC offer and transmit it via a [`SignalingMessage::CallOffer`] message containing the corresponding SDP.
    ///
    /// Upon rejection, the target client will reply with [`SignalingMessage::CallReject`].
    #[serde(rename_all = "camelCase")]
    CallInvite {
        /// When sent to the signaling server by the caller, this is the ID of the target client to call.
        /// When received from the signaling server (by the callee), this is the ID of the source client initiating the call.
        peer_id: String,
    },
    /// A call accept message sent by the target client to accept an incoming call.
    ///
    /// The signaling server will forward the offer to the source client, exchanging the [`SignalingMessage::CallAccept::peer_id`] with the callee's ID.
    #[serde(rename_all = "camelCase")]
    CallAccept {
        /// When sent to the signaling server by the callee, this is the ID of the source client initiating the call.
        /// When received from the signaling server (by the caller), this is the ID of the target client rejecting the call.
        peer_id: String,
    },
    /// A call reject message sent by the target client to reject an incoming call.
    ///
    /// The signaling server will forward the offer to the source client, exchanging the [`SignalingMessage::CallReject::peer_id`] with the callee's ID.
    #[serde(rename_all = "camelCase")]
    CallReject {
        /// When sent to the signaling server by the callee, this is the ID of the source client initiating the call.
        /// When received from the signaling server (by the caller), this is the ID of the target client rejecting the call.
        peer_id: String,
    },
    /// A call offer message sent by the client to initiate a call with another client.
    ///
    /// The SDP provided should contain the WebRTC offer created by the caller.
    ///
    /// The signaling server will forward the offer to the target client, exchanging the [`SignalingMessage::CallOffer::peer_id`] with the caller's ID.
    /// The target client will in turn prompt the user to accept or reject the call.
    ///
    /// The target client will create a WebRTC answer and reply with a [`SignalingMessage::CallAnswer`] message containing the corresponding SDP,
    /// which is returned to the source client by the signaling server.
    #[serde(rename_all = "camelCase")]
    CallOffer {
        /// SDP containing the WebRTC offer.
        sdp: String,
        /// When sent to the signaling server by the caller, this is the ID of the target client to call.
        /// When received from the signaling server (by the callee), this is the ID of the source client initiating the call.
        peer_id: String,
    },
    /// A call answer message sent by the target client to accept an incoming call.
    ///
    /// The SDP provided should contain the WebRTC answer created by the callee.
    ///
    /// The signaling server will forward the answer to the source client, exchanging the [`SignalingMessage::CallAnswer::peer_id`] with the callee's ID.
    ///
    /// After the [`SignalingMessage::CallAnswer`] message has been processed, both clients can start ICE candidate gathering
    /// and trickle them to their peer using [`SignalingMessage::CallIceCandidate`].
    #[serde(rename_all = "camelCase")]
    CallAnswer {
        /// SDP containing the WebRTC answer based on the previously received offer.
        sdp: String,
        /// When sent to the signaling server by the callee, this is the ID of the source client initiating the call.
        /// When received from the signaling server (by the caller), this is the ID of the target client accepting the call.
        peer_id: String,
    },
    /// A call end message sent by either client to indicate the (gracious) end of a call.
    ///
    /// The signaling server will forward the message to the given peer, exchanging the [`SignalingMessage::CallEnd::peer_id`] with the other peer's ID.
    #[serde(rename_all = "camelCase")]
    CallEnd { peer_id: String },
    /// A call ICE candidate message sent by either client to trickle ICE candidates to the other peer during call setup.
    ///
    /// The signaling server will forward the candidate to the given peer, exchanging the [`SignalingMessage::CallIceCandidate::peer_id`] with the other peer's ID.
    #[serde(rename_all = "camelCase")]
    CallIceCandidate {
        /// ICE candidate to be trickled to the other peer.
        candidate: String,
        /// Contains the ID of the respective other peer during call setup.
        peer_id: String,
    },
    /// A message sent by the signaling server if no peer with the given ID was found.
    #[serde(rename_all = "camelCase")]
    PeerNotFound {
        /// ID of the peer that was not found.
        peer_id: String,
    },
    /// A message broadcasted by the signaling server when a new client connects.
    ClientConnected {
        /// Information about the newly connected client.
        client: ClientInfo,
    },
    /// A message broadcasted by the signaling server when a client disconnects.
    ClientDisconnected {
        /// ID of the disconnected client.
        id: String,
    },
    /// A message sent by a client to request a list of all currently connected clients.
    ListClients,
    /// A message sent by the signaling server, containing a full list of all currently connected clients.
    ///
    /// This message is automatically sent by the signaling server upon successful login (after receiving a [`SignalingMessage::Login`] message)
    /// and as a response to [`SignalingMessage::ListClients`] requests.
    ClientList {
        /// List of all currently connected clients.
        clients: Vec<ClientInfo>,
    },
    /// Generic error message sent by either a client or the signaling server.
    /// This could indicate an error processing the last received message or signals a failure with the last request.
    #[serde(rename_all = "camelCase")]
    Error {
        /// Reason for the error.
        reason: ErrorReason,
        /// Optional ID of the peer that caused the error.
        peer_id: Option<String>,
    },
}

impl SignalingMessage {
    /// Serializes a [`SignalingMessage`] into a JSON string.
    #[allow(unused)]
    pub fn serialize(message: &Self) -> serde_json::error::Result<String> {
        serde_json::to_string(message)
    }

    /// Deserializes a JSON string into a [`SignalingMessage`].
    #[allow(unused)]
    pub fn deserialize(message: &str) -> serde_json::error::Result<Self> {
        serde_json::from_str(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_serialize_deserialize_login() {
        let message = SignalingMessage::Login {
            token: "token1".to_string(),
        };

        let serialized = SignalingMessage::serialize(&message).unwrap();
        assert_eq!(serialized, "{\"type\":\"Login\",\"token\":\"token1\"}");

        let deserialized = SignalingMessage::deserialize(&serialized).unwrap();
        match deserialized {
            SignalingMessage::Login { token } => {
                assert_eq!(token, "token1");
            }
            _ => panic!("Expected Login message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_login_failure() {
        let message = SignalingMessage::LoginFailure {
            reason: LoginFailureReason::DuplicateId,
        };

        let serialized = SignalingMessage::serialize(&message).unwrap();
        assert_eq!(
            serialized,
            "{\"type\":\"LoginFailure\",\"reason\":\"DuplicateId\"}"
        );

        let deserialized = SignalingMessage::deserialize(&serialized).unwrap();
        match deserialized {
            SignalingMessage::LoginFailure { reason } => {
                assert_eq!(reason, LoginFailureReason::DuplicateId);
            }
            _ => panic!("Expected LoginFailure message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_logout() {
        let message = SignalingMessage::Logout {};

        let serialized = SignalingMessage::serialize(&message).unwrap();
        assert_eq!(serialized, "{\"type\":\"Logout\"}");

        let deserialized = SignalingMessage::deserialize(&serialized).unwrap();
        assert!(matches!(deserialized, SignalingMessage::Logout));
    }

    #[test]
    fn test_serialize_deserialize_call_offer() {
        let message = SignalingMessage::CallOffer {
            sdp: "sdp1".to_string(),
            peer_id: "client1".to_string(),
        };

        let serialized = SignalingMessage::serialize(&message).unwrap();
        assert_eq!(
            serialized,
            "{\"type\":\"CallOffer\",\"sdp\":\"sdp1\",\"peerId\":\"client1\"}"
        );

        let deserialized = SignalingMessage::deserialize(&serialized).unwrap();
        match deserialized {
            SignalingMessage::CallOffer { sdp, peer_id } => {
                assert_eq!(sdp, "sdp1");
                assert_eq!(peer_id, "client1");
            }
            _ => panic!("Expected CallOffer message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_call_answer() {
        let message = SignalingMessage::CallAnswer {
            sdp: "sdp1".to_string(),
            peer_id: "client1".to_string(),
        };

        let serialized = SignalingMessage::serialize(&message).unwrap();
        assert_eq!(
            serialized,
            "{\"type\":\"CallAnswer\",\"sdp\":\"sdp1\",\"peerId\":\"client1\"}"
        );

        let deserialized = SignalingMessage::deserialize(&serialized).unwrap();
        match deserialized {
            SignalingMessage::CallAnswer { sdp, peer_id } => {
                assert_eq!(sdp, "sdp1");
                assert_eq!(peer_id, "client1");
            }
            _ => panic!("Expected CallAnswer message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_call_reject() {
        let message = SignalingMessage::CallReject {
            peer_id: "client1".to_string(),
        };

        let serialized = SignalingMessage::serialize(&message).unwrap();
        assert_eq!(serialized, "{\"type\":\"CallReject\",\"peerId\":\"client1\"}");

        let deserialized = SignalingMessage::deserialize(&serialized).unwrap();
        match deserialized {
            SignalingMessage::CallReject { peer_id } => {
                assert_eq!(peer_id, "client1");
            }
            _ => panic!("Expected CallReject message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_call_end() {
        let message = SignalingMessage::CallEnd {
            peer_id: "client1".to_string(),
        };

        let serialized = SignalingMessage::serialize(&message).unwrap();
        assert_eq!(serialized, "{\"type\":\"CallEnd\",\"peerId\":\"client1\"}");

        let deserialized = SignalingMessage::deserialize(&serialized).unwrap();
        match deserialized {
            SignalingMessage::CallEnd { peer_id } => {
                assert_eq!(peer_id, "client1");
            }
            _ => panic!("Expected CallEnd message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_call_ice_candidate() {
        let message = SignalingMessage::CallIceCandidate {
            candidate: "candidate1".to_string(),
            peer_id: "client1".to_string(),
        };

        let serialized = SignalingMessage::serialize(&message).unwrap();
        assert_eq!(
            serialized,
            "{\"type\":\"CallIceCandidate\",\"candidate\":\"candidate1\",\"peerId\":\"client1\"}"
        );

        let deserialized = SignalingMessage::deserialize(&serialized).unwrap();
        match deserialized {
            SignalingMessage::CallIceCandidate { candidate, peer_id } => {
                assert_eq!(candidate, "candidate1");
                assert_eq!(peer_id, "client1");
            }
            _ => panic!("Expected CallIceCandidate message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_client_connected() {
        let message = SignalingMessage::ClientConnected {
            client: ClientInfo {
                id: "client1".to_string(),
                display_name: "station1".to_string(),
            },
        };

        let serialized = SignalingMessage::serialize(&message).unwrap();
        assert_eq!(
            serialized,
            "{\"type\":\"ClientConnected\",\"client\":{\"id\":\"client1\",\"displayName\":\"station1\"}}"
        );

        let deserialized = SignalingMessage::deserialize(&serialized).unwrap();
        match deserialized {
            SignalingMessage::ClientConnected { client } => {
                assert_eq!(client.id, "client1");
                assert_eq!(client.display_name, "station1");
            }
            _ => panic!("Expected ClientConnected message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_client_disconnected() {
        let message = SignalingMessage::ClientDisconnected {
            id: "client1".to_string(),
        };

        let serialized = SignalingMessage::serialize(&message).unwrap();
        assert_eq!(serialized, "{\"type\":\"ClientDisconnected\",\"id\":\"client1\"}");

        let deserialized = SignalingMessage::deserialize(&serialized).unwrap();
        match deserialized {
            SignalingMessage::ClientDisconnected { id } => {
                assert_eq!(id, "client1");
            }
            _ => panic!("Expected ClientDisconnected message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_list_clients() {
        let message = SignalingMessage::ListClients {};

        let serialized = SignalingMessage::serialize(&message).unwrap();
        assert_eq!(serialized, "{\"type\":\"ListClients\"}");

        let deserialized = SignalingMessage::deserialize(&serialized).unwrap();
        assert!(matches!(deserialized, SignalingMessage::ListClients));
    }

    #[test]
    fn test_serialize_deserialize_client_list() {
        let message = SignalingMessage::ClientList {
            clients: vec![
                ClientInfo {
                    id: "client1".to_string(),
                    display_name: "station1".to_string(),
                },
                ClientInfo {
                    id: "client2".to_string(),
                    display_name: "station2".to_string(),
                },
            ],
        };

        let serialized = SignalingMessage::serialize(&message).unwrap();
        assert_eq!(
            serialized,
            "{\"type\":\"ClientList\",\"clients\":[{\"id\":\"client1\",\"displayName\":\"station1\"},{\"id\":\"client2\",\"displayName\":\"station2\"}]}"
        );

        let deserialized = SignalingMessage::deserialize(&serialized).unwrap();
        match deserialized {
            SignalingMessage::ClientList { clients } => {
                assert_eq!(clients.len(), 2);
                assert_eq!(clients[0].id, "client1");
                assert_eq!(clients[1].id, "client2");
            }
            _ => panic!("Expected CallIceCandidate message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_error() {
        let message = SignalingMessage::Error {
            reason: ErrorReason::MalformedMessage,
            peer_id: None,
        };

        let serialized = SignalingMessage::serialize(&message).unwrap();
        assert_eq!(
            serialized,
            "{\"type\":\"Error\",\"reason\":\"MalformedMessage\",\"peerId\":null}"
        );

        let deserialized = SignalingMessage::deserialize(&serialized).unwrap();
        match deserialized {
            SignalingMessage::Error { reason, peer_id } => {
                assert_eq!(reason, ErrorReason::MalformedMessage);
                assert!(peer_id.is_none());
            }
            _ => panic!("Expected Error message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_error_with_peer_id() {
        let message = SignalingMessage::Error {
            reason: ErrorReason::UnexpectedMessage("error1".to_string()),
            peer_id: Some("client1".to_string()),
        };

        let serialized = SignalingMessage::serialize(&message).unwrap();
        assert_eq!(
            serialized,
            "{\"type\":\"Error\",\"reason\":{\"UnexpectedMessage\":\"error1\"},\"peerId\":\"client1\"}"
        );

        let deserialized = SignalingMessage::deserialize(&serialized).unwrap();
        match deserialized {
            SignalingMessage::Error { reason, peer_id } => {
                assert_eq!(reason, ErrorReason::UnexpectedMessage("error1".to_string()));
                assert_eq!(peer_id, Some("client1".to_string()));
            }
            _ => panic!("Expected Error message"),
        }
    }
}
