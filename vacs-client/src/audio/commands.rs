use crate::app::state::AppState;
use crate::app::state::webrtc::AppStateWebrtcExt;
use crate::audio::manager::{AudioManagerHandle, SourceType};
use crate::audio::{AudioDevices, AudioHosts, AudioVolumes, VolumeType};
use crate::config::{AUDIO_SETTINGS_FILE_NAME, Persistable, PersistedAudioConfig};
use crate::error::Error;
use crate::keybinds::engine::KeybindEngineHandle;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use vacs_audio::device::{DeviceSelector, DeviceType};
use vacs_audio::error::AudioError;

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_get_hosts(app_state: State<'_, AppState>) -> Result<AudioHosts, Error> {
    log::info!("Getting audio hosts");

    let mut selected = app_state
        .lock()
        .await
        .config
        .audio
        .host_name
        .clone()
        .unwrap_or_default();
    if selected.is_empty() {
        selected = DeviceSelector::default_host_name();
    }

    let hosts = DeviceSelector::all_host_names();

    Ok(AudioHosts {
        selected,
        all: hosts,
    })
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_set_host(
    app: AppHandle,
    app_state: State<'_, AppState>,
    audio_manager: State<'_, AudioManagerHandle>,
    host_name: String,
) -> Result<(), Error> {
    let mut state = app_state.lock().await;

    if state.active_call_peer_id().is_some() {
        return Err(AudioError::Other(anyhow::anyhow!(
            "Cannot set audio host while call is active"
        ))
        .into());
    }

    log::info!("Setting audio host (name: {host_name})");

    let persisted_audio_config: PersistedAudioConfig = {
        let mut audio_config = state.config.audio.clone();
        audio_config.host_name = Some(host_name).filter(|x| !x.is_empty());

        audio_manager
            .write()
            .switch_output_device(app.clone(), &audio_config, false)?;

        state.config.audio = audio_config;
        state.config.audio.clone().into()
    };

    let config_dir = app
        .path()
        .app_config_dir()
        .expect("Cannot get config directory");
    persisted_audio_config.persist(&config_dir, AUDIO_SETTINGS_FILE_NAME)?;

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_get_devices(
    app_state: State<'_, AppState>,
    audio_manager: State<'_, AudioManagerHandle>,
    device_type: DeviceType,
) -> Result<AudioDevices, Error> {
    log::info!("Getting audio devices (type: {:?})", device_type);

    let state = app_state.lock().await;

    let host = state.config.audio.host_name.clone();
    let host = host.as_deref();
    let (preferred, picked) = match device_type {
        DeviceType::Input => {
            let preferred = state
                .config
                .audio
                .input_device_name
                .clone()
                .unwrap_or_default();
            let picked =
                DeviceSelector::picked_device_name(DeviceType::Input, host, Some(&preferred))?;
            (preferred, picked)
        }
        DeviceType::Output => {
            let preferred = state
                .config
                .audio
                .output_device_name
                .clone()
                .unwrap_or_default();
            let picked = audio_manager.read().output_device_name();
            (preferred, picked)
        }
    };
    drop(state);

    let default = DeviceSelector::default_device_name(device_type, host)?;
    let devices: Vec<String> = DeviceSelector::all_device_names(device_type, host)?;

    Ok(AudioDevices {
        preferred,
        picked,
        default,
        all: devices,
    })
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_set_device(
    app: AppHandle,
    app_state: State<'_, AppState>,
    audio_manager: State<'_, AudioManagerHandle>,
    device_type: DeviceType,
    device_name: String,
) -> Result<(), Error> {
    let mut state = app_state.lock().await;
    let mut audio_manager = audio_manager.write();

    if audio_manager.is_input_device_attached() {
        return Err(AudioError::Other(anyhow::anyhow!(
            "Cannot set audio device while call is active"
        ))
        .into());
    }

    log::info!(
        "Setting audio device (name: {:?}, type: {:?})",
        device_name,
        device_type
    );

    let device_name = Some(device_name).filter(|x| !x.is_empty());
    let persisted_audio_config: PersistedAudioConfig = {
        match device_type {
            DeviceType::Input => state.config.audio.input_device_name = device_name,
            DeviceType::Output => {
                let mut audio_config = state.config.audio.clone();
                audio_config.output_device_name = device_name;

                audio_manager.switch_output_device(app.clone(), &audio_config, false)?;

                state.config.audio = audio_config;
            }
        }

        state.config.audio.clone().into()
    };

    let config_dir = app
        .path()
        .app_config_dir()
        .expect("Cannot get config directory");
    persisted_audio_config.persist(&config_dir, AUDIO_SETTINGS_FILE_NAME)?;

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_get_volumes(app_state: State<'_, AppState>) -> Result<AudioVolumes, Error> {
    log::info!("Getting audio volumes");

    let state = app_state.lock().await;
    let audio_config = &state.config.audio;

    Ok(AudioVolumes {
        input: audio_config.input_device_volume,
        output: audio_config.output_device_volume,
        click: audio_config.click_volume,
        chime: audio_config.chime_volume,
    })
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_set_volume(
    app: AppHandle,
    app_state: State<'_, AppState>,
    audio_manager: State<'_, AudioManagerHandle>,
    volume_type: VolumeType,
    volume: f32,
) -> Result<(), Error> {
    log::info!(
        "Setting audio volume (type: {:?}, volume: {:?})",
        volume_type,
        volume
    );
    let mut state = app_state.lock().await;
    let audio_manager = audio_manager.read();

    match volume_type {
        VolumeType::Input => {
            audio_manager.set_input_volume(volume);
            state.config.audio.input_device_volume = volume;
        }
        VolumeType::Output => {
            audio_manager.set_output_volume(SourceType::Opus, volume);
            audio_manager.set_output_volume(SourceType::Ringback, volume);
            audio_manager.set_output_volume(SourceType::RingbackOneshot, volume);
            state.config.audio.output_device_volume = volume;
        }
        VolumeType::Click => {
            audio_manager.set_output_volume(SourceType::Click, volume);
            state.config.audio.click_volume = volume;
        }
        VolumeType::Chime => {
            audio_manager.set_output_volume(SourceType::Ring, volume);
            state.config.audio.chime_volume = volume;
        }
    }

    let persisted_audio_config: PersistedAudioConfig = state.config.audio.clone().into();

    let config_dir = app
        .path()
        .app_config_dir()
        .expect("Cannot get config directory");
    persisted_audio_config.persist(&config_dir, AUDIO_SETTINGS_FILE_NAME)?;

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_play_ui_click(
    audio_manager: State<'_, AudioManagerHandle>,
) -> Result<(), Error> {
    if let Some(audio_manager) = audio_manager.try_read_for(Duration::from_millis(500)) {
        log::trace!("Playing UI click");
        audio_manager.start(SourceType::Click);
    } else {
        log::warn!("Play UI click state lock acquire timed out");
    }

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_start_input_level_meter(
    app_state: State<'_, AppState>,
    audio_manager: State<'_, AudioManagerHandle>,
    app: AppHandle,
) -> Result<(), Error> {
    log::trace!("Starting input level meter");

    let state = app_state.lock().await;
    let audio_config = &state.config.audio.clone();
    let mut audio_manager = audio_manager.write();

    if audio_manager.is_input_device_attached() {
        return Err(AudioError::Other(anyhow::anyhow!(
            "Cannot start input level meter while call is active"
        ))
        .into());
    }

    audio_manager.attach_input_level_meter(
        app.clone(),
        audio_config,
        Box::new(move |level| {
            app.emit("audio:input-level", level).ok();
        }),
    )?;

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_stop_input_level_meter(
    audio_manager: State<'_, AudioManagerHandle>,
) -> Result<(), Error> {
    log::trace!("Stopping input level meter");

    audio_manager.write().detach_input_device();

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_set_radio_prio(
    keybind_engine: State<'_, KeybindEngineHandle>,
    prio: bool,
) -> Result<(), Error> {
    keybind_engine.read().await.set_radio_prio(prio);
    Ok(())
}
