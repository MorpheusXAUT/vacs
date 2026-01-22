use crate::vatsim::PositionId;
use crate::ws::server::ServerMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum LoginFailureReason {
    Unauthorized,
    DuplicateId,
    InvalidCredentials,
    NoActiveVatsimConnection,
    AmbiguousVatsimPosition(Vec<PositionId>),
    InvalidVatsimPosition,
    Timeout,
    IncompatibleProtocolVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TerminationReason {
    NoActiveVatsimConnection,
    AmbiguousVatsimPosition(Vec<PositionId>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginFailure {
    reason: LoginFailureReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Terminated {
    reason: TerminationReason,
}

impl From<LoginFailureReason> for LoginFailure {
    fn from(reason: LoginFailureReason) -> Self {
        Self { reason }
    }
}

impl From<LoginFailure> for ServerMessage {
    fn from(value: LoginFailure) -> Self {
        Self::LoginFailure(value)
    }
}

impl From<LoginFailureReason> for ServerMessage {
    fn from(value: LoginFailureReason) -> Self {
        Self::LoginFailure(value.into())
    }
}

impl From<TerminationReason> for Terminated {
    fn from(reason: TerminationReason) -> Self {
        Self { reason }
    }
}

impl From<Terminated> for ServerMessage {
    fn from(value: Terminated) -> Self {
        Self::Terminated(value)
    }
}

impl From<TerminationReason> for ServerMessage {
    fn from(value: TerminationReason) -> Self {
        Self::Terminated(value.into())
    }
}
