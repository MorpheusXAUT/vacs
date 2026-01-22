use crate::vatsim::ClientId;
use crate::ws::client::ClientMessage;
use crate::ws::server::ServerMessage;
use crate::ws::shared::CallId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebrtcOffer {
    pub call_id: CallId,
    pub from_client_id: ClientId,
    pub to_client_id: ClientId,
    pub sdp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdp_mid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdp_m_line_index: Option<u16>,
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
