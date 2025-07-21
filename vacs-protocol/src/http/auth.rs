use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct InitVatsimLogin {
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthExchangeToken {
    pub code: String,
    pub state: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthResponse {
    pub cid: String,
}
