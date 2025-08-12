use serde::Serialize;
use vacs_audio::{Device, DeviceType};
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
pub async fn audio_get_devices(device_type: DeviceType) -> Result<AudioDevices, Error> {
    log::info!("Getting audio devices (type: {:?})", device_type);

    let default_device = Device::find_default(device_type)?.device_name();
    let devices: Vec<String> = Device::find_all(device_type)?.into_iter().map(|device| device.device_name()).collect();

    Ok(AudioDevices {
        selected: "".to_string(), // TODO get selected device
        default: default_device,
        all: devices,
    })
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_set_device(device_name: String, device_type: DeviceType) -> Result<(), Error> {
    log::info!("Setting audio device (name: {:?}, type: {:?})", device_name, device_type);

    // TODO handle audio device selection

    Ok(())
}