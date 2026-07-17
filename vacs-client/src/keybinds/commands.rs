use crate::app::PersistedClientConfig;
use crate::app::state::AppState;
use crate::config::{CLIENT_SETTINGS_FILE_NAME, Persistable};
use crate::error::Error;
use crate::keybinds::engine::KeybindEngineHandle;
use crate::keybinds::{
    FrontendKeybindsConfig, FrontendTransmitConfig, Keybind, KeybindsConfig, TransmitConfig,
    parse_key_code,
};
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

        state.config.client.radio.validate(&transmit_config).await?;

        keybind_engine
            .write()
            .await
            .set_config(
                &transmit_config,
                &state.config.client.keybinds,
                state.config.client.radio.integration.is_some(),
            )
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
pub async fn keybinds_get_keybinds_config(
    app_state: State<'_, AppState>,
) -> Result<FrontendKeybindsConfig, Error> {
    Ok(app_state.lock().await.config.client.keybinds.clone().into())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn keybinds_set_binding(
    app: AppHandle,
    app_state: State<'_, AppState>,
    keybind_engine: State<'_, KeybindEngineHandle>,
    code: Option<String>,
    keybind: Keybind,
) -> Result<(), Error> {
    let capabilities = Capabilities::default();
    if !capabilities.keybind_listener {
        return Err(Error::CapabilityNotAvailable("Keybinds".to_string()));
    }

    let code = parse_key_code(code)?;

    let persisted_client_config: PersistedClientConfig = {
        let mut state = app_state.lock().await;

        let mut keybinds_config: KeybindsConfig = state.config.client.keybinds.clone();

        match keybind {
            Keybind::AcceptCall => keybinds_config.accept_call = code,
            Keybind::EndCall => keybinds_config.end_call = code,
            Keybind::ToggleRadioPrio => keybinds_config.toggle_radio_prio = code,
            _ => {}
        }

        keybind_engine
            .write()
            .await
            .set_config(
                &state.config.client.transmit_config,
                &keybinds_config,
                state.config.client.radio.integration.is_some(),
            )
            .await?;

        state.config.client.keybinds = keybinds_config;
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
pub async fn keybinds_get_external_binding(
    keybind_engine: State<'_, KeybindEngineHandle>,
    keybind: Keybind,
) -> Result<Option<String>, Error> {
    let capabilities = Capabilities::default();
    if !capabilities.keybind_listener {
        return Err(Error::CapabilityNotAvailable("Keybinds".to_string()));
    }
    Ok(keybind_engine.read().await.get_external_binding(keybind))
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
pub async fn keybinds_is_portal_shortcut_bound(
    #[cfg_attr(not(target_os = "linux"), allow(unused_variables))] keybind: Keybind,
) -> Result<bool, Error> {
    #[cfg(target_os = "linux")]
    {
        use crate::keybinds::runtime;
        return Ok(runtime::is_portal_shortcut_bound(keybind.into()).await);
    }

    #[cfg(not(target_os = "linux"))]
    {
        return Err(Error::Other(Box::new(anyhow::anyhow!(
            "Checking portal shortcut bindings is only supported on Linux"
        ))));
    }
}
