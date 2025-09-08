use crate::app::state::AppState;
use crate::config::{CLIENT_SETTINGS_FILE_NAME, Persistable, PersistedClientConfig};
use crate::error::Error;
use anyhow::Context;
use tauri::{AppHandle, Manager, State, Window};

#[tauri::command]
pub fn app_frontend_ready() {
    log::info!("Frontend ready");
}

#[tauri::command]
pub async fn app_set_always_on_top(
    window: Window,
    app: AppHandle,
    app_state: State<'_, AppState>,
    always_on_top: bool,
) -> Result<bool, Error> {
    let persisted_client_config: PersistedClientConfig = {
        window
            .set_always_on_top(always_on_top)
            .context("Failed to change window always on top behaviour")?;

        let mut state = app_state.lock().await;
        state.config.client.always_on_top = always_on_top;
        state.config.client.clone().into()
    };

    let config_dir = app
        .path()
        .app_config_dir()
        .expect("Cannot get config directory");
    persisted_client_config.persist(&config_dir, CLIENT_SETTINGS_FILE_NAME)?;

    Ok(persisted_client_config.client.always_on_top)
}
