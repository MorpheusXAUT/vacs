use serde::Serialize;
use vacs_audio::{Device, DeviceType};
use crate::error::Error;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    name: String,
    is_default: bool,
}

impl From<Device> for AudioDevice {
    fn from(device: Device) -> Self {
        Self {
            name: device.device_name(),
            is_default: device.is_default,
        }
    }
}

#[tauri::command]
#[vacs_macros::log_err]
pub async fn audio_get_devices(device_type: DeviceType) -> Result<Vec<AudioDevice>, Error> {
    log::info!("Getting audio devices (type: {:?})", device_type);

    let devices: Vec<AudioDevice> = Device::find_all(device_type)?.into_iter().map(AudioDevice::from).collect();

    Ok(devices)
}