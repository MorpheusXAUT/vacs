pub(crate) mod commands;

use crate::config::BackendEndpoint;
use crate::error::Error;
use anyhow::Context;
use tauri::{AppHandle, Emitter, Manager};
use url::Url;
use vacs_signaling::protocol::http::auth::{AuthExchangeToken, UserInfo};
use crate::app::state::http::HttpState;

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
        .state::<HttpState>()
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
