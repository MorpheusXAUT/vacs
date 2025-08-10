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
pub async fn signaling_start_call(
    app_state: State<'_, AppState>,
    peer_id: String,
) -> Result<(), Error> {
    log::debug!("Starting call with {peer_id}");

    app_state
        .lock()
        .await
        .send_signaling_message(
            SignalingMessage::CallOffer {
                peer_id,
                sdp: "".to_string(), // TODO webrtc
            },
        )
        .await
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_accept_call(
    app_state: State<'_, AppState>,
    peer_id: String,
    _sdp: String,
) -> Result<(), Error> {
    log::debug!("Accepting call from {peer_id}");

    app_state
        .lock()
        .await
        .send_signaling_message(
            SignalingMessage::CallAnswer {
                peer_id,
                sdp: "".to_string(), // TODO webrtc
            },
        )
        .await
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_end_call(
    app_state: State<'_, AppState>,
    peer_id: String,
) -> Result<(), Error> {
    log::debug!("Ending call with {peer_id}");

    app_state
        .lock()
        .await
        .send_signaling_message(
            SignalingMessage::CallEnd {
                peer_id,
            },
        )
        .await
}