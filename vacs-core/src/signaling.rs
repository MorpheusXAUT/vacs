use serde::{Deserialize, Serialize};

/// Possible reasons for a login failure.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum LoginFailureReason {
    /// The client's ID is already in use.
    IdTaken,
    /// The provided credentials are invalid.
    InvalidCredentials,
    /// The login flow/initial handshake was performed incorrectly.
    InvalidLoginFlow,
}

/// Represents the current or updated status of a client as observed by the signaling server.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum ClientStatus {
    /// The client is connected or just established connection to the signaling server.
    Connected,
    /// The client just disconnected from the signaling server.
    Disconnected,
}

/// Represents a client as observed by the signaling server.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ClientInfo {
    /// ID of the client.
    pub id: String,
    /// Display (station) name of the client.
    pub display_name: String,
}

/// Represents a message exchanged between the signaling server and clients.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Message {
    /// A login message sent by the client upon initial connection, providing its ID and auth token.
    ///
    /// Upon successful login, a [`Message::ClientList`] response will be returned, containing a list of all currently connected clients.
    ///
    /// Upon login failure (either due to the client's ID already being in use or due to an invalid auth token), a [`Message::Error`] response will be returned.
    Login {
        /// ID of the client, displayed to the user for call selection.
        id: String,
        /// Opaque token used to authenticate the client.
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
    /// A call offer message sent by the client to initiate a call with another client.
    ///
    /// The SDP provided should contain the WebRTC offer created by the caller.
    ///
    /// The signaling server will forward the offer to the target client, exchanging the [`Message::CallOffer::peer_id`] with the caller's ID.
    /// The target client will in turn prompt the user to accept or reject the call.
    ///
    /// Upon acceptance, the target client will create a WebRTC answer and reply with a [`Message::CallAnswer`] message containing the corresponding SDP,
    /// which is returned to the source client by the signaling server.
    ///
    /// Upon rejection, the target client will reply with [`Message::CallReject`].
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
    /// The signaling server will forward the answer to the source client, exchanging the [`Message::CallAnswer::peer_id`] with the callee's ID.
    ///
    /// After the [`Message::CallAnswer`] message has been processed, both clients can start ICE candidate gathering
    /// and trickle them to their peer using [`Message::CallIceCandidate`].
    CallAnswer {
        /// SDP containing the WebRTC answer based on the previously received offer.
        sdp: String,
        /// When sent to the signaling server by the callee, this is the ID of the source client initiating the call.
        /// When received from the signaling server (by the caller), this is the ID of the target client accepting the call.
        peer_id: String,
    },
    /// A call reject message sent by the target client to reject an incoming call.
    ///
    /// The signaling server will forward the offer to the source client, exchanging the [`Message::CallReject::peer_id`] with the callee's ID.
    CallReject {
        /// When sent to the signaling server by the callee, this is the ID of the source client initiating the call.
        /// When received from the signaling server (by the caller), this is the ID of the target client rejecting the call.
        peer_id: String,
    },
    /// A call end message sent by either client to indicate the (gracious) end of a call.
    ///
    /// The signaling server will forward the message to the given peer, exchanging the [`Message::CallEnd::peer_id`] with the other peer's ID.
    CallEnd { peer_id: String },
    /// A call ICE candidate message sent by either client to trickle ICE candidates to the other peer during call setup.
    ///
    /// The signaling server will forward the candidate to the given peer, exchanging the [`Message::CallIceCandidate::peer_id`] with the other peer's ID.
    CallIceCandidate {
        /// ICE candidate to be trickled to the other peer.
        candidate: String,
        /// Contains the ID of the respective other peer during call setup.
        peer_id: String,
    },
    /// A message sent by the signaling server if no peer with the given ID was found.
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
    /// This message is automatically sent by the signaling server upon successful login (after receiving a [`Message::Login`] message)
    /// and as a response to [`Message::ListClients`] requests.
    ClientList {
        /// List of all currently connected clients.
        clients: Vec<ClientInfo>,
    },
    /// Generic error message sent by either a client or the signaling server.
    /// This could indicate an error processing the last received message or signals a failure with the last request.
    Error {
        /// Message describing the error.
        message: String,
        /// Optional ID of the peer that caused the error.
        peer_id: Option<String>,
    },
}

impl Message {
    /// Serializes a [`Message`] into a JSON string.
    #[allow(unused)]
    pub fn serialize(message: &Self) -> serde_json::error::Result<String> {
        serde_json::to_string(message)
    }

    /// Deserializes a JSON string into a [`Message`].
    #[allow(unused)]
    pub fn deserialize(message: &str) -> serde_json::error::Result<Self> {
        serde_json::from_str(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize_login() {
        let message = Message::Login {
            id: "client1".to_string(),
            token: "token1".to_string(),
        };

        let serialized = Message::serialize(&message).unwrap();
        assert_eq!(
            serialized,
            "{\"Login\":{\"id\":\"client1\",\"token\":\"token1\"}}"
        );

        let deserialized = Message::deserialize(&serialized).unwrap();
        match deserialized {
            Message::Login { id, token } => {
                assert_eq!(id, "client1");
                assert_eq!(token, "token1");
            }
            _ => panic!("Expected Login message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_login_failure() {
        let message = Message::LoginFailure {
            reason: LoginFailureReason::IdTaken,
        };

        let serialized = Message::serialize(&message).unwrap();
        assert_eq!(serialized, "{\"LoginFailure\":{\"reason\":\"IdTaken\"}}");

        let deserialized = Message::deserialize(&serialized).unwrap();
        match deserialized {
            Message::LoginFailure { reason } => {
                assert_eq!(reason, LoginFailureReason::IdTaken);
            }
            _ => panic!("Expected LoginFailure message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_logout() {
        let message = Message::Logout {};

        let serialized = Message::serialize(&message).unwrap();
        assert_eq!(serialized, "\"Logout\"");

        let deserialized = Message::deserialize(&serialized).unwrap();
        assert!(matches!(deserialized, Message::Logout));
    }

    #[test]
    fn test_serialize_deserialize_call_offer() {
        let message = Message::CallOffer {
            sdp: "sdp1".to_string(),
            peer_id: "client1".to_string(),
        };

        let serialized = Message::serialize(&message).unwrap();
        assert_eq!(
            serialized,
            "{\"CallOffer\":{\"sdp\":\"sdp1\",\"peer_id\":\"client1\"}}"
        );

        let deserialized = Message::deserialize(&serialized).unwrap();
        match deserialized {
            Message::CallOffer { sdp, peer_id } => {
                assert_eq!(sdp, "sdp1");
                assert_eq!(peer_id, "client1");
            }
            _ => panic!("Expected CallOffer message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_call_answer() {
        let message = Message::CallAnswer {
            sdp: "sdp1".to_string(),
            peer_id: "client1".to_string(),
        };

        let serialized = Message::serialize(&message).unwrap();
        assert_eq!(
            serialized,
            "{\"CallAnswer\":{\"sdp\":\"sdp1\",\"peer_id\":\"client1\"}}"
        );

        let deserialized = Message::deserialize(&serialized).unwrap();
        match deserialized {
            Message::CallAnswer { sdp, peer_id } => {
                assert_eq!(sdp, "sdp1");
                assert_eq!(peer_id, "client1");
            }
            _ => panic!("Expected CallAnswer message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_call_reject() {
        let message = Message::CallReject {
            peer_id: "client1".to_string(),
        };

        let serialized = Message::serialize(&message).unwrap();
        assert_eq!(serialized, "{\"CallReject\":{\"peer_id\":\"client1\"}}");

        let deserialized = Message::deserialize(&serialized).unwrap();
        match deserialized {
            Message::CallReject { peer_id } => {
                assert_eq!(peer_id, "client1");
            }
            _ => panic!("Expected CallReject message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_call_end() {
        let message = Message::CallEnd {
            peer_id: "client1".to_string(),
        };

        let serialized = Message::serialize(&message).unwrap();
        assert_eq!(serialized, "{\"CallEnd\":{\"peer_id\":\"client1\"}}");

        let deserialized = Message::deserialize(&serialized).unwrap();
        match deserialized {
            Message::CallEnd { peer_id } => {
                assert_eq!(peer_id, "client1");
            }
            _ => panic!("Expected CallEnd message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_call_ice_candidate() {
        let message = Message::CallIceCandidate {
            candidate: "candidate1".to_string(),
            peer_id: "client1".to_string(),
        };

        let serialized = Message::serialize(&message).unwrap();
        assert_eq!(
            serialized,
            "{\"CallIceCandidate\":{\"candidate\":\"candidate1\",\"peer_id\":\"client1\"}}"
        );

        let deserialized = Message::deserialize(&serialized).unwrap();
        match deserialized {
            Message::CallIceCandidate { candidate, peer_id } => {
                assert_eq!(candidate, "candidate1");
                assert_eq!(peer_id, "client1");
            }
            _ => panic!("Expected CallIceCandidate message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_client_connected() {
        let message = Message::ClientConnected {
            client: ClientInfo {
                id: "client1".to_string(),
                display_name: "station1".to_string(),
            },
        };

        let serialized = Message::serialize(&message).unwrap();
        assert_eq!(
            serialized,
            "{\"ClientConnected\":{\"client\":{\"id\":\"client1\",\"display_name\":\"station1\"}}}"
        );

        let deserialized = Message::deserialize(&serialized).unwrap();
        match deserialized {
            Message::ClientConnected { client } => {
                assert_eq!(client.id, "client1");
                assert_eq!(client.display_name, "station1");
            }
            _ => panic!("Expected ClientConnected message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_client_disconnected() {
        let message = Message::ClientDisconnected {
            id: "client1".to_string(),
        };

        let serialized = Message::serialize(&message).unwrap();
        assert_eq!(serialized, "{\"ClientDisconnected\":{\"id\":\"client1\"}}");

        let deserialized = Message::deserialize(&serialized).unwrap();
        match deserialized {
            Message::ClientDisconnected { id } => {
                assert_eq!(id, "client1");
            }
            _ => panic!("Expected ClientDisconnected message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_list_clients() {
        let message = Message::ListClients {};

        let serialized = Message::serialize(&message).unwrap();
        assert_eq!(serialized, "\"ListClients\"");

        let deserialized = Message::deserialize(&serialized).unwrap();
        assert!(matches!(deserialized, Message::ListClients));
    }

    #[test]
    fn test_serialize_deserialize_client_list() {
        let message = Message::ClientList {
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

        let serialized = Message::serialize(&message).unwrap();
        assert_eq!(
            serialized,
            "{\"ClientList\":{\"clients\":[{\"id\":\"client1\",\"display_name\":\"station1\"},{\"id\":\"client2\",\"display_name\":\"station2\"}]}}"
        );

        let deserialized = Message::deserialize(&serialized).unwrap();
        match deserialized {
            Message::ClientList { clients } => {
                assert_eq!(clients.len(), 2);
                assert_eq!(clients[0].id, "client1");
                assert_eq!(clients[1].id, "client2");
            }
            _ => panic!("Expected CallIceCandidate message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_error() {
        let message = Message::Error {
            message: "error1".to_string(),
            peer_id: None,
        };

        let serialized = Message::serialize(&message).unwrap();
        assert_eq!(
            serialized,
            "{\"Error\":{\"message\":\"error1\",\"peer_id\":null}}"
        );

        let deserialized = Message::deserialize(&serialized).unwrap();
        match deserialized {
            Message::Error { message, peer_id } => {
                assert_eq!(message, "error1");
                assert!(peer_id.is_none());
            }
            _ => panic!("Expected Error message"),
        }
    }

    #[test]
    fn test_serialize_deserialize_error_with_peer_id() {
        let message = Message::Error {
            message: "error1".to_string(),
            peer_id: Some("client1".to_string()),
        };

        let serialized = Message::serialize(&message).unwrap();
        assert_eq!(
            serialized,
            "{\"Error\":{\"message\":\"error1\",\"peer_id\":\"client1\"}}"
        );

        let deserialized = Message::deserialize(&serialized).unwrap();
        match deserialized {
            Message::Error { message, peer_id } => {
                assert_eq!(message, "error1");
                assert_eq!(peer_id, Some("client1".to_string()));
            }
            _ => panic!("Expected Error message"),
        }
    }
}
