use crate::app::state::http::HttpState;
use crate::app::state::signaling::AppStateSignalingExt;
use crate::app::state::webrtc::AppStateWebrtcExt;
use crate::app::state::{AppState, AppStateInner};
use crate::audio::manager::{AudioManagerHandle, SourceType};
use crate::config::{
    BackendEndpoint, CLIENT_SETTINGS_FILE_NAME, FrontendStationsConfig, Persistable,
    PersistedClientConfig, PersistedStationsConfig, STATIONS_SETTINGS_FILE_NAME,
};
use crate::error::{Error, HandleUnauthorizedExt};
use std::collections::HashSet;
use tauri::{AppHandle, Manager, State};
use vacs_signaling::protocol::http::webrtc::IceConfig;
use vacs_signaling::protocol::ws::SignalingMessage;

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_connect(
    app_state: State<'_, AppState>,
    http_state: State<'_, HttpState>,
) -> Result<(), Error> {
    let mut app_state = app_state.lock().await;
    app_state.connect_signaling().await?;

    if !app_state.config.ice.is_default() {
        log::info!("Modified ICE config detected, not fetching from server");
        return Ok(());
    }

    refresh_ice_config(&http_state, &mut app_state).await;

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_disconnect(app: AppHandle) -> Result<(), Error> {
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
    http_state: State<'_, HttpState>,
    audio_manager: State<'_, AudioManagerHandle>,
    peer_id: String,
) -> Result<(), Error> {
    log::debug!("Starting call with {peer_id}");

    let mut state = app_state.lock().await;

    state
        .send_signaling_message(SignalingMessage::CallInvite {
            peer_id: peer_id.clone(),
        })
        .await?;

    if state.is_ice_config_expired() {
        refresh_ice_config(&http_state, &mut state).await;
    }

    state.add_call_to_call_list(&app, &peer_id, false);
    state.start_unanswered_call_timer(&app, &peer_id);
    state.set_outgoing_call_peer_id(Some(peer_id));

    audio_manager.read().restart(SourceType::Ringback);

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_accept_call(
    app_state: State<'_, AppState>,
    http_state: State<'_, HttpState>,
    audio_manager: State<'_, AudioManagerHandle>,
    peer_id: String,
) -> Result<(), Error> {
    log::debug!("Accepting call from {peer_id}");

    let mut state = app_state.lock().await;

    if state.is_ice_config_expired() {
        refresh_ice_config(&http_state, &mut state).await;
    }

    state
        .send_signaling_message(SignalingMessage::CallAccept {
            peer_id: peer_id.clone(),
        })
        .await?;
    state.remove_incoming_call_peer_id(&peer_id);

    audio_manager.read().stop(SourceType::Ring);

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_end_call(
    app_state: State<'_, AppState>,
    audio_manager: State<'_, AudioManagerHandle>,
    peer_id: String,
) -> Result<(), Error> {
    log::debug!("Ending call with {peer_id}");

    let mut state = app_state.lock().await;

    state.end_call(&peer_id).await;

    state
        .send_signaling_message(SignalingMessage::CallEnd {
            peer_id: peer_id.clone(),
        })
        .await?;

    state.cancel_unanswered_call_timer(&peer_id);
    state.set_outgoing_call_peer_id(None);

    audio_manager.read().stop(SourceType::Ringback);

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_get_stations_config(
    app_state: State<'_, AppState>,
) -> Result<FrontendStationsConfig, Error> {
    Ok(app_state.lock().await.config.stations.clone().into())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_set_selected_stations_config_profile(
    app: AppHandle,
    app_state: State<'_, AppState>,
    profile: String,
) -> Result<(), Error> {
    let persisted_stations_config: PersistedStationsConfig = {
        let mut state = app_state.lock().await;
        state.config.stations.selected_profile = profile;

        state.config.stations.clone().into()
    };

    let config_dir = app
        .path()
        .app_config_dir()
        .expect("Cannot get config directory");
    persisted_stations_config.persist(&config_dir, STATIONS_SETTINGS_FILE_NAME)?;

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_get_ignored_clients(
    app_state: State<'_, AppState>,
) -> Result<HashSet<String>, Error> {
    let state = app_state.lock().await;

    Ok(state.config.client.ignored.clone())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_add_ignored_client(
    app: AppHandle,
    app_state: State<'_, AppState>,
    client_id: String,
) -> Result<bool, Error> {
    let (persisted_stations_config, added): (PersistedClientConfig, bool) = {
        let mut state = app_state.lock().await;
        let added = state.config.client.ignored.insert(client_id);
        (state.config.client.clone().into(), added)
    };

    let config_dir = app
        .path()
        .app_config_dir()
        .expect("Cannot get config directory");
    persisted_stations_config.persist(&config_dir, CLIENT_SETTINGS_FILE_NAME)?;

    Ok(added)
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn signaling_remove_ignored_client(
    app: AppHandle,
    app_state: State<'_, AppState>,
    client_id: String,
) -> Result<bool, Error> {
    let (persisted_stations_config, removed): (PersistedClientConfig, bool) = {
        let mut state = app_state.lock().await;
        let removed = state.config.client.ignored.remove(&client_id);
        (state.config.client.clone().into(), removed)
    };

    let config_dir = app
        .path()
        .app_config_dir()
        .expect("Cannot get config directory");
    persisted_stations_config.persist(&config_dir, CLIENT_SETTINGS_FILE_NAME)?;

    Ok(removed)
}

async fn refresh_ice_config(http_state: &HttpState, app_state: &mut AppStateInner) {
    let config = match http_state
        .http_get::<IceConfig>(BackendEndpoint::IceConfig, None)
        .await
    {
        Ok(config) => config,
        Err(err) => {
            log::warn!("Failed to fetch ICE config, falling back to default: {err:?}");
            return;
        }
    };

    log::info!(
        "Received ICE config from server, expires at {}",
        config.expires_at.unwrap_or_default()
    );
    app_state.set_ice_config(config);
}
