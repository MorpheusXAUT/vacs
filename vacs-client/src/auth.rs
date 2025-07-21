use crate::config::BackendEndpoint;
use crate::state::AppState;
use anyhow::Context;
use tauri::{AppHandle, Emitter, Manager};
use url::Url;
use vacs_protocol::http::auth::{
    AuthExchangeToken, AuthResponse, InitVatsimLogin,
};

pub async fn open_auth_url(app_state: &AppState) -> anyhow::Result<()> {
    let auth_url = app_state
        .http_get::<InitVatsimLogin>(BackendEndpoint::InitAuth, None)
        .await
        .context("Failed to get auth URL")?
        .url;

    tauri_plugin_opener::open_url(auth_url, None::<&str>)
        .context("Failed to open auth URL with the default browser")?;

    Ok(())
}

pub async fn handle_auth_callback(app: &AppHandle, url: &str) -> anyhow::Result<()> {
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
        .http_post::<AuthResponse, AuthExchangeToken>(
            BackendEndpoint::ExchangeCode,
            None,
            Some(AuthExchangeToken {
                code: code.to_string(),
                state: state.to_string(),
            }),
        )
        .await
        .context("Failed to exchange auth code")?
        .cid;

    log::info!("Successfully authenticated as CID {cid}");

    app.emit("vatsim-cid", cid).ok();

    Ok(())
}
