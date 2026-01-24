use crate::vatsim::{ClientId, PositionId, StationId};
use crate::ws::client::ClientMessage;
use crate::ws::server::ServerMessage;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[repr(transparent)]
#[serde(transparent)]
pub struct CallId(Uuid);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallSource {
    pub client_id: ClientId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_id: Option<PositionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub station_id: Option<StationId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CallTarget {
    Client(ClientId),
    Position(PositionId),
    Station(StationId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CallErrorReason {
    TargetNotFound,
    CallActive,
    WebrtcFailure,
    AudioFailure,
    CallFailure,
    SignalingFailure,
    AutoHangup,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallInvite {
    pub call_id: CallId,
    pub source: CallSource,
    pub target: CallTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallAccept {
    pub call_id: CallId,
    pub accepting_client_id: ClientId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallEnd {
    pub call_id: CallId,
    pub ending_client_id: ClientId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallError {
    pub call_id: CallId,
    pub reason: CallErrorReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl CallId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub const fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }

    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl std::fmt::Display for CallId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Uuid> for CallId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl std::str::FromStr for CallId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::try_parse(s)?))
    }
}

impl TryFrom<String> for CallId {
    type Error = uuid::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<&str> for CallId {
    type Error = uuid::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl AsRef<Uuid> for CallId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl std::borrow::Borrow<Uuid> for CallId {
    fn borrow(&self) -> &Uuid {
        &self.0
    }
}

impl From<ClientId> for CallSource {
    fn from(value: ClientId) -> Self {
        Self {
            client_id: value,
            position_id: None,
            station_id: None,
        }
    }
}

impl CallSource {
    pub fn new(client_id: ClientId) -> Self {
        Self {
            client_id,
            position_id: None,
            station_id: None,
        }
    }

    pub fn with_position(mut self, position_id: PositionId) -> Self {
        self.position_id = Some(position_id);
        self
    }

    pub fn with_station(mut self, station_id: StationId) -> Self {
        self.station_id = Some(station_id);
        self
    }
}

impl CallEnd {
    pub fn new(call_id: CallId, ending_client_id: ClientId) -> Self {
        Self {
            call_id,
            ending_client_id,
        }
    }
}

impl From<ClientId> for CallTarget {
    fn from(value: ClientId) -> Self {
        Self::Client(value)
    }
}

impl From<PositionId> for CallTarget {
    fn from(value: PositionId) -> Self {
        Self::Position(value)
    }
}

impl From<StationId> for CallTarget {
    fn from(value: StationId) -> Self {
        Self::Station(value)
    }
}

impl From<CallInvite> for ClientMessage {
    fn from(value: CallInvite) -> Self {
        Self::CallInvite(value)
    }
}

impl From<CallInvite> for ServerMessage {
    fn from(value: CallInvite) -> Self {
        Self::CallInvite(value)
    }
}

impl From<CallAccept> for ClientMessage {
    fn from(value: CallAccept) -> Self {
        Self::CallAccept(value)
    }
}

impl From<CallAccept> for ServerMessage {
    fn from(value: CallAccept) -> Self {
        Self::CallAccept(value)
    }
}

impl From<CallEnd> for ClientMessage {
    fn from(value: CallEnd) -> Self {
        Self::CallEnd(value)
    }
}

impl From<CallEnd> for ServerMessage {
    fn from(value: CallEnd) -> Self {
        Self::CallEnd(value)
    }
}

impl From<CallError> for ClientMessage {
    fn from(value: CallError) -> Self {
        Self::CallError(value)
    }
}

impl From<CallError> for ServerMessage {
    fn from(value: CallError) -> Self {
        Self::CallError(value)
    }
}
