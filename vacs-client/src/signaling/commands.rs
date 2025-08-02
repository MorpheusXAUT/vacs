use crate::app::state::AppState;
use crate::config::BackendEndpoint;
use crate::error::{Error, HandleUnauthorizedExt};
use tauri::{AppHandle, Manager, State};
use vacs_protocol::ws::SignalingMessage;

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_connect(app: AppHandle) -> Result<(), Error> {
    app.state::<AppState>()
        .lock()
        .await
        .connect_signaling(&app)
        .await
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_disconnect(app: AppHandle) -> Result<(), Error> {
    log::debug!("Disconnecting signaling server");
    app.state::<AppState>()
        .lock()
        .await
        .disconnect_signaling(&app)
        .await;
    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_terminate(
    app: AppHandle,
    app_state: State<'_, AppState>,
) -> Result<(), Error> {
    log::debug!("Terminating signaling server session");

    let state = app_state.lock().await;

    state
        .http_delete::<()>(BackendEndpoint::TerminateWsSession, None)
        .await
        .handle_unauthorized(&app)?;

    log::info!("Successfully terminated signaling server session");
    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_da_key_click(
    app_state: State<'_, AppState>,
    client_id: &str,
) -> Result<(), Error> {
    log::debug!("Handling DA key click");

    app_state
        .lock()
        .await
        .send_signaling_message(
            SignalingMessage::CallOffer {
                peer_id: client_id.to_string(),
                sdp: "".to_string(),
            },
        )
        .await
}
