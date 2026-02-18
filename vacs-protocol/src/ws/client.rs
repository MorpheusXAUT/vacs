pub mod auth;
pub mod calls;

pub use auth::*;
pub use calls::*;

use crate::ws::shared::{
    CallAccept, CallEnd, CallError, CallInvite, Error, WebrtcAnswer, WebrtcIceCandidate,
    WebrtcOffer,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ClientMessage {
    Login(Login),
    Logout,
    CallInvite(CallInvite),
    CallAccept(CallAccept),
    CallEnd(CallEnd),
    CallReject(CallReject),
    CallError(CallError),
    WebrtcOffer(WebrtcOffer),
    WebrtcAnswer(WebrtcAnswer),
    WebrtcIceCandidate(WebrtcIceCandidate),
    ListClients,
    ListStations,
    Disconnect,
    Error(Error),
}

impl ClientMessage {
    pub fn serialize(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    pub fn into_json(self) -> serde_json::Result<String> {
        self.serialize()
    }

    pub fn deserialize(s: &str) -> serde_json::Result<Self> {
        serde_json::from_str(s)
    }
}
