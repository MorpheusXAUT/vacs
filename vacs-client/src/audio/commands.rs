use tauri::State;
use vacs_audio::{Device, DeviceType};
use crate::app::state::AppState;
use crate::audio::{AudioDevices, AudioVolumes, VolumeType};
use crate::config::{Persistable, PersistedAudioConfig, AUDIO_SETTINGS_FILE_NAME};
use crate::error::Error;

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_get_devices(app_state: State<'_, AppState>, device_type: DeviceType) -> Result<AudioDevices, Error> {
    log::info!("Getting audio devices (type: {:?})", device_type);

    let selected = match device_type {
        DeviceType::Input => {
            app_state.lock().await.config.audio.input_device_name.to_string()
        },
        DeviceType::Output => {
            app_state.lock().await.config.audio.output_device_name.to_string()
        },
    };

    let default_device = Device::find_default(device_type)?.device_name();
    let devices: Vec<String> = Device::find_all(device_type)?.into_iter().map(|device| device.device_name()).collect();

    Ok(AudioDevices {
        selected,
        default: default_device,
        all: devices,
    })
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_set_device(app_state: State<'_, AppState>, device_type: DeviceType, device_name: String) -> Result<(), Error> {
    log::info!("Setting audio device (name: {:?}, type: {:?})", device_name, device_type);

    let persisted_audio_config: PersistedAudioConfig = {
        let mut state = app_state.lock().await;

        match device_type {
            DeviceType::Input => state.config.audio.input_device_name = device_name,
            DeviceType::Output => state.config.audio.output_device_name = device_name,
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
pub async fn audio_set_volume(app_state: State<'_, AppState>, volume_type: VolumeType, volume: f32) -> Result<(), Error> {
    log::info!("Setting audio volume (type: {:?}, volume: {:?})", volume_type, volume);

    let persisted_audio_config: PersistedAudioConfig = {
        let mut state = app_state.lock().await;

        match volume_type {
            VolumeType::Input => state.config.audio.input_device_volume = volume,
            VolumeType::Output => state.config.audio.output_device_volume = volume,
            VolumeType::Click => state.config.audio.click_volume = volume,
            VolumeType::Chime => state.config.audio.chime_volume = volume,
        }

        state.config.audio.clone().into()
    };

    persisted_audio_config.persist(AUDIO_SETTINGS_FILE_NAME)?;

    Ok(())
}