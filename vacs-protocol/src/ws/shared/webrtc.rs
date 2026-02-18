use crate::vatsim::ClientId;
use crate::ws::client::ClientMessage;
use crate::ws::server::ServerMessage;
use crate::ws::shared::CallId;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebrtcOffer {
    pub call_id: CallId,
    pub from_client_id: ClientId,
    pub to_client_id: ClientId,
    pub sdp: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebrtcAnswer {
    pub call_id: CallId,
    pub from_client_id: ClientId,
    pub to_client_id: ClientId,
    pub sdp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebrtcIceCandidate {
    pub call_id: CallId,
    pub from_client_id: ClientId,
    pub to_client_id: ClientId,
    pub candidate: String,
}

impl std::fmt::Debug for WebrtcOffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebrtcOffer")
            .field("call_id", &self.call_id)
            .field("from_client_id", &self.from_client_id)
            .field("to_client_id", &self.to_client_id)
            .field("sdp_len", &self.sdp.len())
            .finish()
    }
}

impl std::fmt::Debug for WebrtcAnswer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebrtcAnswer")
            .field("call_id", &self.call_id)
            .field("from_client_id", &self.from_client_id)
            .field("to_client_id", &self.to_client_id)
            .field("sdp_len", &self.sdp.len())
            .finish()
    }
}

impl From<WebrtcOffer> for ClientMessage {
    fn from(value: WebrtcOffer) -> Self {
        Self::WebrtcOffer(value)
    }
}

impl From<WebrtcOffer> for ServerMessage {
    fn from(value: WebrtcOffer) -> Self {
        Self::WebrtcOffer(value)
    }
}

impl From<WebrtcAnswer> for ClientMessage {
    fn from(value: WebrtcAnswer) -> Self {
        Self::WebrtcAnswer(value)
    }
}

impl From<WebrtcAnswer> for ServerMessage {
    fn from(value: WebrtcAnswer) -> Self {
        Self::WebrtcAnswer(value)
    }
}

impl From<WebrtcIceCandidate> for ClientMessage {
    fn from(value: WebrtcIceCandidate) -> Self {
        Self::WebrtcIceCandidate(value)
    }
}

impl From<WebrtcIceCandidate> for ServerMessage {
    fn from(value: WebrtcIceCandidate) -> Self {
        Self::WebrtcIceCandidate(value)
    }
}
