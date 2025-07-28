pub(crate) mod commands;

use std::sync::Mutex;
use crate::config::BackendEndpoint;
use crate::error::Error;
use crate::app::state::AppState;
use anyhow::Context;
use tauri::{AppHandle, Emitter, Manager};
use url::Url;
use vacs_protocol::http::auth::{AuthExchangeToken, UserInfo};

#[vacs_macros::log_err]
pub async fn handle_auth_callback(app: &AppHandle, url: &str) -> Result<(), Error> {
    let url = Url::parse(url).context("Failed to parse auth callback URL")?;

    let mut code = None;
    let mut state = None;

    for (key, value) in url.query_pairs() {
        match &*key {
            "code" => code = Some(value),
            "state" => state = Some(value),
            _ => {}
        }
    }

    let code = code.context("Auth callback URL does not contain code")?;
    let state = state.context("Auth callback URL does not contain code")?;

    let cid = app
        .state::<AppState>()
        .lock()
        .await
        .http_post::<UserInfo, AuthExchangeToken>(
            BackendEndpoint::ExchangeCode,
            None,
            Some(AuthExchangeToken {
                code: code.to_string(),
                state: state.to_string(),
            }),
        )
        .await?
        .cid;

    log::info!("Successfully authenticated as CID {cid}");
    app.emit("auth:authenticated", cid).ok();

    Ok(())
}
