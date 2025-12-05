use crate::app::state::AppState;
use crate::config::{
    CLIENT_SETTINGS_FILE_NAME, FrontendRadioConfig, FrontendTransmitConfig, Persistable,
    PersistedClientConfig, RadioConfig, TransmitConfig, TransmitMode,
};
use crate::error::Error;
use crate::keybinds::engine::KeybindEngineHandle;
use crate::platform::Capabilities;
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
    keybind_engine: State<'_, KeybindEngineHandle>,
    transmit_config: FrontendTransmitConfig,
) -> Result<(), Error> {
    let capabilities = Capabilities::default();
    if !capabilities.keybind_listener {
        return Err(Error::CapabilityNotAvailable("Keybinds".to_string()));
    }

    let persisted_client_config: PersistedClientConfig = {
        let mut state = app_state.lock().await;

        let transmit_config: TransmitConfig = transmit_config.try_into()?;

        keybind_engine
            .write()
            .await
            .set_config(&transmit_config)
            .await?;

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

#[tauri::command]
#[vacs_macros::log_err]
pub async fn keybinds_get_radio_config(
    app_state: State<'_, AppState>,
) -> Result<FrontendRadioConfig, Error> {
    Ok(app_state.lock().await.config.client.radio.clone().into())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn keybinds_set_radio_config(
    app: AppHandle,
    app_state: State<'_, AppState>,
    keybind_engine: State<'_, KeybindEngineHandle>,
    radio_config: FrontendRadioConfig,
) -> Result<(), Error> {
    let capabilities = Capabilities::default();
    if !capabilities.keybind_listener {
        return Err(Error::CapabilityNotAvailable("Keybinds".to_string()));
    }

    let persisted_client_config: PersistedClientConfig = {
        let mut state = app_state.lock().await;

        let radio_config: RadioConfig = radio_config.try_into()?;

        keybind_engine
            .write()
            .await
            .set_radio_config(&radio_config)
            .await?;

        state.config.client.radio = radio_config;
        state.config.client.clone().into()
    };

    let config_dir = app
        .path()
        .app_config_dir()
        .expect("Cannot get config directory");
    persisted_client_config.persist(&config_dir, CLIENT_SETTINGS_FILE_NAME)?;

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn keybinds_has_radio(
    keybind_engine: State<'_, KeybindEngineHandle>,
) -> Result<bool, Error> {
    let capabilities = Capabilities::default();
    if !capabilities.keybind_listener {
        return Ok(false);
    }

    Ok(keybind_engine.read().await.has_radio())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn keybinds_get_external_binding(
    keybind_engine: State<'_, KeybindEngineHandle>,
    mode: TransmitMode,
) -> Result<Option<String>, Error> {
    let capabilities = Capabilities::default();
    if !capabilities.keybind_listener {
        return Err(Error::CapabilityNotAvailable("Keybinds".to_string()));
    }
    Ok(keybind_engine.read().await.get_external_binding(mode))
}

#[tauri::command]
#[vacs_macros::log_err]
pub fn keybinds_open_system_shortcuts_settings() -> Result<(), Error> {
    #[cfg(target_os = "linux")]
    {
        use crate::platform::DesktopEnvironment;
        return DesktopEnvironment::get()
            .open_keyboard_shortcuts_settings()
            .map_err(|err| Error::Other(Box::new(anyhow::anyhow!(err))));
    }

    #[cfg(not(target_os = "linux"))]
    {
        return Err(Error::Other(Box::new(anyhow::anyhow!(
            "Opening keyboard shortcuts settings is only supported on Linux"
        ))));
    }
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn keybinds_reconnect_radio(
    keybind_engine: State<'_, KeybindEngineHandle>,
) -> Result<(), Error> {
    keybind_engine.read().await.reconnect_radio().await
}
