use crate::SAMPLE_RATE;
use crate::config::AudioDeviceConfig;
use anyhow::Context;
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{SupportedStreamConfig, SupportedStreamConfigRange};
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum DeviceType {
    Input,
    Output,
}

impl Display for DeviceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceType::Input => write!(f, "input"),
            DeviceType::Output => write!(f, "output"),
        }
    }
}

pub struct Device {
    pub device_type: DeviceType,
    pub device: cpal::Device,
    pub stream_config: SupportedStreamConfig,
}

impl Display for Device {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = self
            .device
            .name()
            .unwrap_or_else(|_| "<unknown>".to_string());
        write!(
            f,
            "{} device: {}, stream config: {:?}",
            self.device_type, name, self.stream_config
        )
    }
}

impl Device {
    pub fn new(config: &AudioDeviceConfig, device_type: DeviceType) -> anyhow::Result<Self> {
        tracing::trace!("Initialising device");

        let host = find_host(&config.host_name)?;
        let device = find_device(&host, &config.device_name, &device_type)?;
        let stream_config = find_supported_stream_config(&device, config, &device_type)?;
        let device = Device {
            device_type,
            device,
            stream_config,
        };

        tracing::debug!(%device, "Device initialised");
        Ok(device)
    }
}

fn find_host(host_name: &Option<String>) -> anyhow::Result<cpal::Host> {
    tracing::trace!(?host_name, "Trying to find audio host");

    let host_id = match host_name {
        Some(host_name) => {
            let available_hosts = cpal::available_hosts();
            match available_hosts
                .iter()
                .find(|id| id.name().eq_ignore_ascii_case(host_name))
            {
                Some(id) => *id,
                None => {
                    anyhow::bail!(
                        "Unknown audio host '{}’. Available: {:?}",
                        host_name,
                        available_hosts
                            .iter()
                            .map(|id| id.name())
                            .collect::<Vec<_>>()
                    );
                }
            }
        }
        None => cpal::default_host().id(),
    };

    cpal::host_from_id(host_id).context("Failed to get audio host")
}

fn find_device(
    host: &cpal::Host,
    device_name: &Option<String>,
    device_type: &DeviceType,
) -> anyhow::Result<cpal::Device> {
    tracing::trace!(?device_type, ?device_name, "Trying to find device");

    match device_name {
        Some(device_name) => {
            let devices = match device_type {
                DeviceType::Input => host
                    .input_devices()
                    .context("Failed to get input devices")?,
                DeviceType::Output => host
                    .output_devices()
                    .context("Failed to get output devices")?,
            };

            let matching_devices = devices
                .filter(|device| {
                    device
                        .name()
                        .unwrap_or("".into())
                        .eq_ignore_ascii_case(device_name)
                })
                .collect::<Vec<_>>();

            if matching_devices.is_empty() {
                anyhow::bail!(
                    "Unknown {} device '{}'. Available: {:?}",
                    device_type,
                    device_name,
                    match device_type {
                        DeviceType::Input => host
                            .input_devices()
                            .context("Failed to get input devices")?,
                        DeviceType::Output => host
                            .output_devices()
                            .context("Failed to get output devices")?,
                    }
                    .map(|d| d.name().unwrap())
                    .collect::<Vec<_>>()
                );
            } else if matching_devices.len() > 1 {
                anyhow::bail!(
                    "Multiple matching {} devices '{}' found: {:?}",
                    device_type,
                    device_name,
                    matching_devices
                        .iter()
                        .map(|d| d.name().unwrap())
                        .collect::<Vec<_>>()
                );
            }

            Ok(matching_devices[0].clone())
        }
        None => match device_type {
            DeviceType::Input => host
                .default_input_device()
                .context("Failed to get default input device"),
            DeviceType::Output => host
                .default_output_device()
                .context("Failed to get default output device"),
        },
    }
}

fn find_supported_stream_config(
    device: &cpal::Device,
    config: &AudioDeviceConfig,
    device_type: &DeviceType,
) -> anyhow::Result<SupportedStreamConfig> {
    tracing::trace!(
        ?device_type,
        ?config,
        "Trying to find supported stream config"
    );

    let mut configs: Box<dyn Iterator<Item = SupportedStreamConfigRange>> = match device_type {
        DeviceType::Input => Box::new(
            device
                .supported_input_configs()
                .context("Failed to get supported input stream configs")?,
        ),
        DeviceType::Output => Box::new(
            device
                .supported_output_configs()
                .context("Failed to get supported output stream configs")?,
        ),
    };

    let config_range = configs
        .find(|c| {
            c.sample_format() == cpal::SampleFormat::F32
                && c.channels() == config.channels
                && c.min_sample_rate().0 <= SAMPLE_RATE
                && c.max_sample_rate().0 >= SAMPLE_RATE
        })
        .ok_or_else(|| anyhow::anyhow!("No supported {} stream config found", device_type))?;

    Ok(config_range.with_sample_rate(cpal::SampleRate(SAMPLE_RATE)))
}
