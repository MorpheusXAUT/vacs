use crate::vatsim::PositionId;
use crate::ws::client::ClientMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Login {
    pub token: String,
    pub protocol_version: String,
    pub custom_profile: bool,
    pub position_id: Option<PositionId>,
}

impl From<Login> for ClientMessage {
    fn from(value: Login) -> Self {
        Self::Login(value)
    }
}
