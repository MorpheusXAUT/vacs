pub mod client;
pub mod server;
pub mod shared;

use crate::ws::client::ClientMessage;
use crate::ws::server::ServerMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Message {
    Client(ClientMessage),
    Server(ServerMessage),
}

impl Message {
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

impl From<ClientMessage> for Message {
    fn from(value: ClientMessage) -> Self {
        Self::Client(value)
    }
}

impl From<ServerMessage> for Message {
    fn from(value: ServerMessage) -> Self {
        Self::Server(value)
    }
}
