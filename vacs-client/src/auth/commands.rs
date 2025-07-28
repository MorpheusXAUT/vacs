use crate::config::BackendEndpoint;
use crate::error::{Error, HandleUnauthorizedExt};
use crate::app::state::AppState;
use anyhow::Context;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use vacs_protocol::http::auth::{InitVatsimLogin, UserInfo};

#[tauri::command]
#[vacs_macros::log_err]
pub async fn auth_open_oauth_url(app: AppHandle) -> Result<(), Error> {
    let auth_url = app
        .state::<AppState>()
        .http_get::<InitVatsimLogin>(BackendEndpoint::InitAuth, None)
        .await?
        .url;

    log::info!("Opening auth URL: {auth_url}");

    tauri_plugin_opener::open_url(auth_url, None::<&str>)
        .context("Failed to open auth URL with the default browser")?;

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn auth_check_session(app: AppHandle) -> Result<(), Error> {
    log::debug!("Fetching user info");
    let response = app
        .state::<AppState>()
        .http_get::<UserInfo>(BackendEndpoint::UserInfo, None)
        .await;

    match response {
        Ok(user_info) => {
            log::info!("Authenticated as CID {}", user_info.cid);
            app.emit("auth:authenticated", user_info.cid).ok();
            Ok(())
        }
        Err(Error::Unauthorized) => {
            log::info!("Not authenticated");
            app.emit("auth:unauthenticated", Value::Null).ok();
            Ok(())
        }
        Err(err) => {
            log::info!("Not authenticated");
            app.emit("auth:unauthenticated", Value::Null).ok();
            Err(err)
        }
    }
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn auth_logout(app: AppHandle) -> Result<(), Error> {
    log::debug!("Logging out");

    let app_state = &app.state::<AppState>();
    app_state
        .http_post::<(), ()>(BackendEndpoint::Logout, None, None)
        .await
        .handle_unauthorized(&app)?;

    app_state
        .clear_cookie_store()
        .context("Failed to clear cookie store")?;

    log::info!("Successfully logged out");
    app.emit("auth:unauthenticated", Value::Null).ok();

    Ok(())
}
