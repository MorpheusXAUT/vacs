use crate::app::state::AppState;
use crate::config::{
    CLIENT_SETTINGS_FILE_NAME, FrontendTransmitConfig, Persistable, PersistedClientConfig,
    TransmitConfig,
};
use crate::error::Error;
use crate::keybinds::KeybindsTrait;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
#[vacs_macros::log_err]
pub async fn keybinds_get_transmit_config(
    app_state: State<'_, AppState>,
) -> Result<FrontendTransmitConfig, Error> {
    Ok(app_state
        .lock()
        .await
        .config
        .client
        .transmit_config
        .clone()
        .into())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn keybinds_set_transmit_config(
    app: AppHandle,
    app_state: State<'_, AppState>,
    transmit_config: FrontendTransmitConfig,
) -> Result<(), Error> {
    let persisted_client_config: PersistedClientConfig = {
        let mut state = app_state.lock().await;

        state
            .config
            .client
            .transmit_config
            .unregister_keybinds(app.clone());

        let transmit_config: TransmitConfig = transmit_config.try_into()?;

        if transmit_config.code_for_mode().is_some() {
            transmit_config.register_keybinds(app.clone())?;
        }

        state.config.client.transmit_config = transmit_config;
        state.config.client.clone().into()
    };

    let config_dir = app
        .path()
        .app_config_dir()
        .expect("Cannot get config directory");
    persisted_client_config.persist(&config_dir, CLIENT_SETTINGS_FILE_NAME)?;

    Ok(())
}
