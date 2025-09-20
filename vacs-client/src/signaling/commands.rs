use crate::app::state::AppState;
use crate::app::state::audio::AppStateAudioExt;
use crate::app::state::http::HttpState;
use crate::app::state::signaling::AppStateSignalingExt;
use crate::app::state::webrtc::AppStateWebrtcExt;
use crate::audio::manager::SourceType;
use crate::config::BackendEndpoint;
use crate::error::{Error, HandleUnauthorizedExt};
use tauri::{AppHandle, Manager, State};
use vacs_signaling::protocol::ws::SignalingMessage;

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
    http_state: State<'_, HttpState>,
) -> Result<(), Error> {
    log::debug!("Terminating signaling server session");

    http_state
        .http_delete::<()>(BackendEndpoint::TerminateWsSession, None)
        .await
        .handle_unauthorized(&app)?;

    log::info!("Successfully terminated signaling server session");

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_start_call(
    app: AppHandle,
    app_state: State<'_, AppState>,
    peer_id: String,
) -> Result<(), Error> {
    log::debug!("Starting call with {peer_id}");

    let mut state = app_state.lock().await;

    state
        .send_signaling_message(SignalingMessage::CallInvite {
            peer_id: peer_id.clone(),
        })
        .await?;

    state.add_call_to_call_list(&app, &peer_id, false);

    state.set_outgoing_call_peer_id(Some(peer_id));
    state.audio_manager().restart(SourceType::Ringback);

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_accept_call(
    app_state: State<'_, AppState>,
    peer_id: String,
) -> Result<(), Error> {
    log::debug!("Accepting call from {peer_id}");

    let mut state = app_state.lock().await;

    state
        .send_signaling_message(SignalingMessage::CallAccept {
            peer_id: peer_id.clone(),
        })
        .await?;
    state.remove_incoming_call_peer_id(&peer_id);

    state.audio_manager().stop(SourceType::Ring);

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_end_call(
    app_state: State<'_, AppState>,
    peer_id: String,
) -> Result<(), Error> {
    log::debug!("Ending call with {peer_id}");

    let mut state = app_state.lock().await;

    state.end_call(&peer_id).await;

    state
        .send_signaling_message(SignalingMessage::CallEnd { peer_id })
        .await?;

    state.set_outgoing_call_peer_id(None);
    state.audio_manager().stop(SourceType::Ringback);

    Ok(())
}
