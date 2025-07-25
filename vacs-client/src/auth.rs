use crate::config::BackendEndpoint;
use crate::state::AppState;
use anyhow::Context;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use url::Url;
use vacs_protocol::http::auth::{AuthExchangeToken, InitVatsimLogin, UserInfo};

pub async fn open_auth_url(app_state: &AppState) -> anyhow::Result<()> {
    let auth_url = app_state
        .http_get::<InitVatsimLogin>(BackendEndpoint::InitAuth, None)
        .await
        .context("Failed to get auth URL")?
        .url;

    log::info!("Opening auth URL: {}", auth_url);

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
        .http_post::<UserInfo, AuthExchangeToken>(
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
    app.emit("auth:authenticated", cid).ok();

    Ok(())
}

pub async fn check_auth_session(app: &AppHandle) -> anyhow::Result<bool> {
    log::debug!("Fetching user info");
    let user_info = app
        .state::<AppState>()
        .http_get::<UserInfo>(BackendEndpoint::UserInfo, None)
        .await
        .context("Failed to fetch user info");

    if let Ok(user_info) = user_info {
        log::info!("Authenticated as CID {}", user_info.cid);
        app.emit("auth:authenticated", user_info.cid).ok();
        Ok(true)
    } else {
        log::info!("Not authenticated");
        app.emit("auth:unauthenticated", Value::Null).ok();
        Ok(false)
    }
}

pub async fn logout(app: &AppHandle) -> anyhow::Result<()> {
    log::debug!("Logging out");

    let app_state = app.state::<AppState>();
    app_state
        .http_post::<(), ()>(BackendEndpoint::Logout, None, None)
        .await
        .context("Failed to logout")?;

    app_state
        .clear_cookie_store()
        .context("Failed to clear cookie store")?;

    log::info!("Successfully logged out");
    app.emit("auth:unauthenticated", Value::Null).ok();

    Ok(())
}
