use serde::Serialize;
use tauri::State;
use vacs_audio::{Device, DeviceType};
use crate::app::state::AppState;
use crate::config::{Persistable, PersistedAudioConfig};
use crate::error::Error;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevices {
    selected: String,
    default: String,
    all: Vec<String>,
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_get_devices(app_state: State<'_, AppState>, device_type: DeviceType) -> Result<AudioDevices, Error> {
    log::info!("Getting audio devices (type: {:?})", device_type);

    let selected = match device_type {
        DeviceType::Input => {
            app_state.lock().await.config.audio.input_device.to_string()
        },
        DeviceType::Output => {
            app_state.lock().await.config.audio.output_device.to_string()
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
            DeviceType::Input => state.config.audio.input_device = device_name,
            DeviceType::Output => state.config.audio.output_device = device_name,
        }

        state.config.audio.clone().into()
    };

    persisted_audio_config.persist("audio.toml")?;

    Ok(())
}