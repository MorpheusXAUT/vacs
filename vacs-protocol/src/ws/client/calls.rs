use crate::vatsim::ClientId;
use crate::ws::client::ClientMessage;
use crate::ws::shared::CallId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum CallRejectReason {
    Busy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallReject {
    pub call_id: CallId,
    pub rejecting_client_id: ClientId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<CallRejectReason>,
}

impl From<CallReject> for ClientMessage {
    fn from(value: CallReject) -> Self {
        Self::CallReject(value)
    }
}
