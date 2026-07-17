use serde::{Deserialize, Serialize};
use vacs_audio::device::DeviceType;

pub(crate) mod commands;
pub(crate) mod manager;
pub(crate) mod source_type;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub host_name: Option<String>, // Name of audio backend host, None means default host
    pub input_device_name: Option<String>, // None means default device
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_device_id: Option<String>, // Stable device ID for reliable matching, None means default device
    pub output_device_name: Option<String>, // None means default device
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_device_id: Option<String>, // Stable device ID for reliable matching, None means default device
    pub speaker_enabled: bool,
    pub speaker_device_name: Option<String>, // None means default device
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker_device_id: Option<String>, // Stable device ID for reliable matching, None means default device
    pub input_device_volume: f32,
    pub input_device_volume_amp: f32,
    pub output_device_volume: f32,
    pub output_device_volume_amp: f32,
    pub click_volume: f32,
    pub chime_volume: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            host_name: None,
            input_device_name: None,
            input_device_id: None,
            output_device_name: None,
            output_device_id: None,
            speaker_enabled: false,
            speaker_device_name: None,
            speaker_device_id: None,
            input_device_volume: 0.5,
            input_device_volume_amp: 4.0,
            output_device_volume: 0.5,
            output_device_volume_amp: 2.0,
            click_volume: 0.5,
            chime_volume: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PersistedAudioConfig {
    pub audio: AudioConfig,
}

impl From<AudioConfig> for PersistedAudioConfig {
    fn from(audio: AudioConfig) -> Self {
        Self { audio }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioHosts {
    selected: String,
    all: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevices {
    preferred: Option<String>,
    picked: Option<String>,
    default: String,
    all: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VolumeType {
    Input,
    Output,
    Click,
    Chime,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioVolumes {
    input: f32,
    output: f32,
    click: f32,
    chime: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientAudioDeviceType {
    Input,
    Output,
    Speaker,
}

impl From<ClientAudioDeviceType> for DeviceType {
    fn from(value: ClientAudioDeviceType) -> Self {
        match value {
            ClientAudioDeviceType::Input => DeviceType::Input,
            ClientAudioDeviceType::Output => DeviceType::Output,
            ClientAudioDeviceType::Speaker => DeviceType::Output,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackDeviceType {
    Output,
    Speaker,
}

impl std::fmt::Display for PlaybackDeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlaybackDeviceType::Output => write!(f, "output"),
            PlaybackDeviceType::Speaker => write!(f, "speaker"),
        }
    }
}
