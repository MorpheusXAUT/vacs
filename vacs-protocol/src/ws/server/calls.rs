use crate::vatsim::ClientId;
use crate::ws::server::ServerMessage;
use crate::ws::shared::CallId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CallCancelReason {
    AnsweredElsewhere(ClientId),
    CallerCancelled,
    AllFailed, // TODO: Separate in error/reject and choose whatever the last remaining client (which received the call invite) resulted to
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallCancelled {
    pub call_id: CallId,
    pub reason: CallCancelReason,
}

impl CallCancelled {
    pub fn new(call_id: CallId, reason: CallCancelReason) -> Self {
        Self { call_id, reason }
    }
}

impl From<CallCancelled> for ServerMessage {
    fn from(value: CallCancelled) -> Self {
        Self::CallCancelled(value)
    }
}
