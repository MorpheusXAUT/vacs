use crate::app::state::AppState;
use crate::audio::manager::SourceType;
use crate::audio::{AudioDevices, AudioHosts, AudioVolumes, VolumeType};
use crate::config::{Persistable, PersistedAudioConfig, AUDIO_SETTINGS_FILE_NAME};
use crate::error::Error;
use tauri::State;
use vacs_audio::{Device, DeviceType};

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_get_hosts(app_state: State<'_, AppState>) -> Result<AudioHosts, Error> {
    log::info!("Getting audio hosts");

    let mut selected = app_state.lock().await.config.audio.host_name.to_string();
    if selected.is_empty() {
        selected = Device::find_default_host();
    }

    let hosts = Device::find_all_hosts();

    Ok(AudioHosts {
        selected,
        all: hosts,
    })
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_set_host(
    app_state: State<'_, AppState>,
    host_name: String,
) -> Result<(), Error> {
    let mut state = app_state.lock().await;

    // TODO: Replace with webrtc peer
    if state.active_call_peer_id().is_some() {
        return Err(Error::AudioDevice("Cannot set audio device while call is active".to_string()));
    }

    log::info!("Setting audio host (name: {host_name})");

    let persisted_audio_config: PersistedAudioConfig = {
        state.config.audio.host_name = host_name;
        state.config.audio.clone().into()
    };

    persisted_audio_config.persist(AUDIO_SETTINGS_FILE_NAME)?;

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_get_devices(
    app_state: State<'_, AppState>,
    device_type: DeviceType,
) -> Result<AudioDevices, Error> {
    log::info!("Getting audio devices (type: {:?})", device_type);

    let selected = match device_type {
        DeviceType::Input => app_state
            .lock()
            .await
            .config
            .audio
            .input_device_name
            .to_string(),
        DeviceType::Output => app_state
            .lock()
            .await
            .config
            .audio
            .output_device_name
            .to_string(),
    };

    let default_device = Device::find_default(device_type)?.device_name();
    let devices: Vec<String> = Device::find_all(device_type)?
        .into_iter()
        .map(|device| device.device_name())
        .collect();

    Ok(AudioDevices {
        selected,
        default: default_device,
        all: devices,
    })
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_set_device(
    app_state: State<'_, AppState>,
    device_type: DeviceType,
    device_name: String,
) -> Result<(), Error> {
    let mut state = app_state.lock().await;

    // TODO: Replace with webrtc peer
    if state.active_call_peer_id().is_some() {
        return Err(Error::AudioDevice("Cannot set audio device while call is active".to_string()));
    }

    log::info!(
        "Setting audio device (name: {:?}, type: {:?})",
        device_name,
        device_type
    );

    let persisted_audio_config: PersistedAudioConfig = {

        match device_type {
            DeviceType::Input => state.config.audio.input_device_name = device_name,
            DeviceType::Output => {
                state.config.audio.output_device_name = device_name;
                let audio_config = state.config.audio.clone();
                state.audio_manager.switch_output_device(&audio_config)?;
            }
        }

        state.config.audio.clone().into()
    };

    persisted_audio_config.persist(AUDIO_SETTINGS_FILE_NAME)?;

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
    app_state: State<'_, AppState>,
    volume_type: VolumeType,
    volume: f32,
) -> Result<(), Error> {
    log::info!(
        "Setting audio volume (type: {:?}, volume: {:?})",
        volume_type,
        volume
    );
    let mut state = app_state.lock().await;

    match volume_type {
        VolumeType::Input => state.config.audio.input_device_volume = volume,
        VolumeType::Output => {
            state.audio_manager.set_volume(SourceType::Opus, volume);
            state.audio_manager.set_volume(SourceType::Ringback, volume);
            state
                .audio_manager
                .set_volume(SourceType::RingbackOneshot, volume);
            state.config.audio.output_device_volume = volume;
        }
        VolumeType::Click => {
            state.audio_manager.set_volume(SourceType::Click, volume);
            state.config.audio.click_volume = volume;
        }
        VolumeType::Chime => {
            state.audio_manager.set_volume(SourceType::Ring, volume);
            state.config.audio.chime_volume = volume;
        }
    }

    let persisted_audio_config: PersistedAudioConfig = state.config.audio.clone().into();
    persisted_audio_config.persist(AUDIO_SETTINGS_FILE_NAME)?;

    Ok(())
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_play_ui_click(app_state: State<'_, AppState>) -> Result<(), Error> {
    log::trace!("Playing UI click");

    app_state
        .lock()
        .await
        .audio_manager
        .start(SourceType::Click);

    Ok(())
}
