use crate::app::state::AppState;
use crate::app::{UpdateInfo, get_update, open_fatal_error_dialog, open_logs_folder};
use crate::build::VersionInfo;
use crate::config::{CLIENT_SETTINGS_FILE_NAME, Persistable, PersistedClientConfig};
use crate::error::Error;
use crate::platform::Capabilities;
use anyhow::Context;
use tauri::{AppHandle, Emitter, Manager, State, Window};

#[tauri::command]
pub fn app_frontend_ready(app: AppHandle, window: Window) {
    log::info!("Frontend ready");
    if let Err(err) = window.show() {
        log::error!("Failed to show window: {err}");

        open_fatal_error_dialog(
            &app,
            "Failed to show main window. Check your logs for further details.",
        );

        app.exit(1);
    };
}

#[tauri::command]
#[vacs_macros::log_err]
pub fn app_open_logs_folder(app: AppHandle) -> Result<(), Error> {
    open_logs_folder(&app).context("Failed to open logs folder")?;
    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn app_check_for_update(app: AppHandle) -> Result<UpdateInfo, Error> {
    let current_version = VersionInfo::gather().version.to_string();

    if cfg!(debug_assertions) {
        log::info!("Debug build, skipping update check");
        return Ok(UpdateInfo {
            current_version,
            new_version: None,
            required: false,
        });
    }

    let update_info = if let Some(update) = get_update(&app).await? {
        let required = update
            .raw_json
            .get("required")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        log::info!("Update available. Required: {required}");

        UpdateInfo {
            current_version,
            new_version: Some(update.version),
            required,
        }
    } else {
        log::info!("No update available");
        UpdateInfo {
            current_version,
            new_version: None,
            required: false,
        }
    };

    Ok(update_info)
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn app_update(app: AppHandle) -> Result<(), Error> {
    if cfg!(debug_assertions) {
        log::info!("Debug build, skipping update");
        return Ok(());
    }

    if let Some(update) = get_update(&app).await? {
        log::info!(
            "Downloading and installing update. Version: {}",
            &update.version
        );
        let mut downloaded = 0;
        update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    log::debug!("Downloaded {downloaded} of {content_length:?}");
                    if let Some(content_length) = content_length {
                        let progress = (downloaded / (content_length as usize)) * 100;
                        app.emit("update:progress", progress.clamp(0, 100)).ok();
                    }
                },
                || {
                    log::debug!("Download finished");
                },
            )
            .await
            .context("Failed to download and install the update")?;

        log::info!("Update installed. Restarting...");
        app.restart();
    } else {
        log::warn!("Tried to update without an update being available");
    }

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn app_platform_capabilities() -> Result<Capabilities, Error> {
    Ok(Capabilities::default())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn app_set_always_on_top(
    window: Window,
    app: AppHandle,
    app_state: State<'_, AppState>,
    always_on_top: bool,
) -> Result<bool, Error> {
    let capabilities = Capabilities::default();
    if !capabilities.always_on_top {
        return Err(Error::CapabilityNotAvailable("Always on top".to_string()));
    }

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
