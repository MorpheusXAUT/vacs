use serde::Deserialize;
use crate::DeviceType;

#[derive(Debug, Default, Deserialize)]
pub struct AudioConfig {
    pub input: AudioDeviceConfig,
    pub output: AudioDeviceConfig,
}

#[derive(Debug, Deserialize)]
pub struct AudioDeviceConfig {
    pub host_name: Option<String>,
    pub device_name: Option<String>,
    pub channels: u16,
}

impl Default for AudioDeviceConfig {
    fn default() -> Self {
        AudioDeviceConfig {
            host_name: None,
            device_name: None,
            channels: 2,
        }
    }
}

impl From<DeviceType> for AudioDeviceConfig {
    fn from(device_type: DeviceType) -> Self {
        match device_type {
            DeviceType::Input => AudioDeviceConfig {
                channels: 1,
                ..Default::default()
            },
            DeviceType::Output => AudioDeviceConfig::default(),
        }
    }
}