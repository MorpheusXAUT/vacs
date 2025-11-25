use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct IceServer {
    pub urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
}

impl Debug for IceServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IceServer")
            .field("urls", &self.urls)
            .field("username", &self.username)
            .finish_non_exhaustive()
    }
}

impl IceServer {
    pub fn new(urls: Vec<String>) -> Self {
        Self {
            urls,
            username: None,
            credential: None,
        }
    }

    pub fn with_auth(mut self, username: String, credential: String) -> Self {
        self.username = Some(username);
        self.credential = Some(credential);
        self
    }
}

impl From<Vec<String>> for IceServer {
    fn from(value: Vec<String>) -> Self {
        Self::new(value)
    }
}

impl From<String> for IceServer {
    fn from(value: String) -> Self {
        Self::new(vec![value])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IceConfig {
    pub ice_servers: Vec<IceServer>,
}

impl Default for IceConfig {
    fn default() -> Self {
        Self::from(vec![
            "stun:stun.cloudflare.com:3478".to_string(),
            "stun:stun.cloudflare.com:53".to_string(),
        ])
    }
}

impl IceConfig {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

impl From<Vec<IceServer>> for IceConfig {
    fn from(value: Vec<IceServer>) -> Self {
        Self { ice_servers: value }
    }
}

impl From<Vec<String>> for IceConfig {
    fn from(value: Vec<String>) -> Self {
        Self {
            ice_servers: vec![IceServer::new(value)],
        }
    }
}

impl From<String> for IceConfig {
    fn from(value: String) -> Self {
        Self {
            ice_servers: vec![IceServer::new(vec![value])],
        }
    }
}
