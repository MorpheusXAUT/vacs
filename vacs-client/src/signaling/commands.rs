use serde_json::Value;
use crate::app::state::AppState;
use crate::error::{Error, HandleUnauthorizedExt};
use tauri::{AppHandle, Emitter, Manager, State};
use crate::config::BackendEndpoint;

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_connect(app: AppHandle) -> Result<(), Error> {
    app.state::<AppState>().lock().await.connect(&app).await
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_disconnect(app: AppHandle) -> Result<(), Error> {
    app.state::<AppState>().lock().await.disconnect(&app).await;
    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_terminate(app: AppHandle, app_state: State<'_, AppState>) -> Result<(), Error> {
    log::debug!("Terminating signaling server session");

    let state = app_state.lock().await;

    state
        .http_delete::<()>(BackendEndpoint::TerminateWsSession, None)
        .await
        .handle_unauthorized(&app)?;

    log::info!("Successfully terminated signaling server session");
    app.emit("signaling:terminated", Value::Null).ok();

    Ok(())
}
