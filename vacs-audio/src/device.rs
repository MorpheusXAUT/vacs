use crate::config::AudioDeviceConfig;
use crate::TARGET_SAMPLE_RATE;
use anyhow::Context;
use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{Sample, SampleFormat, SupportedStreamConfig, SupportedStreamConfigRange};
use rubato::{SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter};
use tracing::instrument;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

pub struct StreamDevice {
    pub(crate) device_type: DeviceType,
    pub(crate) device: cpal::Device,
    pub(crate) config: cpal::StreamConfig,
    pub(crate) sample_format: SampleFormat,
}

impl StreamDevice {
    #[inline]
    pub fn device_type(&self) -> DeviceType {
        self.device_type
    }

    #[inline]
    pub fn name(&self) -> String {
        self.device.name().unwrap_or_default()
    }

    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    #[inline]
    pub fn channels(&self) -> u16 {
        self.config.channels
    }

    #[instrument(level = "trace", skip(data_callback, error_callback), err)]
    pub(crate) fn build_input_stream<D, E>(
        &self,
        data_callback: D,
        error_callback: E,
    ) -> Result<cpal::Stream>
    where
        D: FnMut(&[f32], &cpal::InputCallbackInfo) + Send + 'static,
        E: FnMut(cpal::StreamError) + Send + 'static,
    {
        debug_assert!(matches!(self.device_type, DeviceType::Input));

        match self.sample_format {
            SampleFormat::F32 => self
                .device
                .build_input_stream::<f32, _, _>(&self.config, data_callback, error_callback, None)
                .map_err(Into::into),
            SampleFormat::I16 => {
                self.build_f32_input_stream::<i16, _, _>(data_callback, error_callback)
            }
            SampleFormat::U16 => {
                self.build_f32_input_stream::<u16, _, _>(data_callback, error_callback)
            }
            other => Err(anyhow::anyhow!(
                "Unsupported input sample format: {:?}",
                other
            )),
        }
    }

    fn build_f32_input_stream<T, D, E>(
        &self,
        mut data_callback: D,
        error_callback: E,
    ) -> Result<cpal::Stream>
    where
        T: Sample<Float = f32> + cpal::SizedSample + 'static,
        D: FnMut(&[f32], &cpal::InputCallbackInfo) + Send + 'static,
        E: FnMut(cpal::StreamError) + Send + 'static,
    {
        let buf: RefCell<Vec<f32>> = RefCell::new(Vec::new());
        if let cpal::BufferSize::Fixed(n) = self.config.buffer_size {
            buf.borrow_mut().reserve(n as usize);
        }

        self.device
            .build_input_stream::<T, _, _>(
                &self.config,
                move |input: &[T], info| {
                    let mut b = buf.borrow_mut();
                    if b.len() != input.len() {
                        b.resize(input.len(), 0.0f32);
                    }
                    for (dst, &src) in b.iter_mut().zip(input.iter()) {
                        *dst = src.to_float_sample();
                    }
                    data_callback(&b, info);
                },
                error_callback,
                None,
            )
            .map_err(Into::into)
    }

    #[instrument(level = "trace", skip(data_callback, error_callback), err)]
    pub(crate) fn build_output_stream<D, E>(
        &self,
        data_callback: D,
        error_callback: E,
    ) -> Result<cpal::Stream>
    where
        D: FnMut(&mut [f32], &cpal::OutputCallbackInfo) + Send + 'static,
        E: FnMut(cpal::StreamError) + Send + 'static,
    {
        debug_assert!(matches!(self.device_type, DeviceType::Output));

        match self.sample_format {
            SampleFormat::F32 => self
                .device
                .build_output_stream::<f32, _, _>(&self.config, data_callback, error_callback, None)
                .map_err(Into::into),
            SampleFormat::I16 => {
                self.build_f32_output_stream::<i16, _, _>(data_callback, error_callback)
            }
            SampleFormat::U16 => {
                self.build_f32_output_stream::<u16, _, _>(data_callback, error_callback)
            }
            other => Err(anyhow::anyhow!(
                "Unsupported output sample format: {:?}",
                other
            )),
        }
    }

    fn build_f32_output_stream<T, D, E>(
        &self,
        mut data_callback: D,
        error_callback: E,
    ) -> Result<cpal::Stream>
    where
        T: cpal::SizedSample + cpal::FromSample<f32> + 'static,
        D: FnMut(&mut [f32], &cpal::OutputCallbackInfo) + Send + 'static,
        E: FnMut(cpal::StreamError) + Send + 'static,
    {
        let buf: RefCell<Vec<f32>> = RefCell::new(Vec::new());
        if let cpal::BufferSize::Fixed(n) = self.config.buffer_size {
            buf.borrow_mut().reserve(n as usize);
        }

        self.device
            .build_output_stream::<T, _, _>(
                &self.config,
                move |output: &mut [T], info| {
                    let mut b = buf.borrow_mut();
                    if b.len() != output.len() {
                        b.resize(output.len(), 0.0f32);
                    }
                    data_callback(&mut b, info);
                    for (dst, &src) in output.iter_mut().zip(b.iter()) {
                        *dst = src.to_sample::<T>();
                    }
                },
                error_callback,
                None,
            )
            .map_err(Into::into)
    }

    pub(crate) fn resampler(&self) -> Result<Option<SincFixedIn<f32>>> {
        if self.sample_rate() == TARGET_SAMPLE_RATE {
            Ok(None)
        } else {
            let resampler_params = SincInterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Cubic,
                oversampling_factor: 256,
                window: WindowFunction::BlackmanHarris2,
            };

            let resample_ratio = match self.device_type {
                DeviceType::Input => TARGET_SAMPLE_RATE as f64 / self.sample_rate() as f64,
                DeviceType::Output => self.sample_rate() as f64 / TARGET_SAMPLE_RATE as f64,
            };

            Ok(Some(
                SincFixedIn::<f32>::new(
                    resample_ratio,
                    2.0,
                    resampler_params,
                    if let cpal::BufferSize::Fixed(n) = self.config.buffer_size {
                        n as usize
                    } else {
                        1024usize
                    },
                    1,
                )
                .context("Failed to create resampler")?,
            ))
        }
    }
}

impl Debug for StreamDevice {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "StreamDevice {{ device_type: {}, device: {}, config: {:?}, sample_format: {:?} }}",
            self.device_type,
            self.device.name().unwrap_or_default(),
            self.config,
            self.sample_format
        )
    }
}

pub struct DeviceSelector {}

impl DeviceSelector {
    #[instrument(level = "debug", err)]
    pub fn open(
        device_type: DeviceType,
        preferred_host: Option<&str>,
        preferred_device_name: Option<&str>,
    ) -> Result<(StreamDevice, bool)> {
        tracing::debug!("Opening device");

        let host = Self::select_host(preferred_host);
        let (device, stream_config, is_fallback) =
            Self::pick_device_with_stream_config(device_type, &host, preferred_device_name)?;

        tracing::debug!(?stream_config, device = ?DeviceDebug(&device), ?is_fallback, "Opened device");
        Ok((
            StreamDevice {
                device_type,
                device,
                config: stream_config.config(),
                sample_format: stream_config.sample_format(),
            },
            is_fallback,
        ))
    }

    #[instrument(level = "debug")]
    pub fn all_host_names() -> Vec<String> {
        tracing::debug!("Retrieving all host names");

        let host_names = cpal::available_hosts()
            .iter()
            .map(|id| id.name().to_string())
            .collect::<Vec<_>>();
        tracing::debug!(host_count = ?host_names.len(), "Retrieved host names");
        host_names
    }

    #[instrument(level = "debug")]
    pub fn default_host_name() -> String {
        tracing::debug!("Retrieving default host name");

        cpal::default_host().id().name().to_string()
    }

    #[instrument(level = "debug", err)]
    pub fn all_device_names(
        preferred_host: Option<&str>,
        device_type: DeviceType,
    ) -> Result<Vec<String>> {
        tracing::debug!("Retrieving all devices names with at least one stream config");

        let host = Self::select_host(preferred_host);
        let devices = Self::host_devices(device_type, &host)?;

        let device_names = devices
            .into_iter()
            .filter_map(|device| {
                if let Ok(device_name) = device.name()
                    && Self::pick_best_stream_config(&device, device_type).is_ok()
                {
                    Some(device_name)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        tracing::debug!(device_count = ?device_names.len(), "Retrieved device names");
        Ok(device_names)
    }

    #[instrument(level = "debug", err)]
    pub fn default_device_name(
        preferred_host: Option<&str>,
        device_type: DeviceType,
    ) -> Result<String> {
        tracing::debug!("Retrieving device name for default device");

        let host = Self::select_host(preferred_host);
        let (device, _) = Self::select_device(device_type, &host, None)?;
        Self::pick_best_stream_config(&device, device_type)?;

        tracing::debug!(device = ?DeviceDebug(&device), "Retrieved device name for default device");
        Ok(device.name().unwrap_or_default())
    }

    #[instrument(level = "trace")]
    fn select_host(preferred_host: Option<&str>) -> cpal::Host {
        tracing::trace!("Selecting host");

        let hosts = cpal::available_hosts();

        if let Some(name) = preferred_host {
            if let Some(id) = hosts.iter().find(|id| id.name().eq_ignore_ascii_case(name)) {
                tracing::trace!(?id, "Selected preferred audio host");
                return cpal::host_from_id(*id).unwrap_or(cpal::default_host());
            }
            if let Some(id) = hosts
                .iter()
                .find(|id| id.name().to_lowercase().contains(&name.to_lowercase()))
            {
                tracing::trace!(
                    ?id,
                    "Selected preferred audio host (based on substring match)"
                );
                return cpal::host_from_id(*id).unwrap_or(cpal::default_host());
            }
        }

        tracing::trace!("Selected default audio host");
        cpal::default_host()
    }

    #[instrument(level = "trace", err, skip(host), fields(host = ?HostDebug(host)))]
    fn pick_device_with_stream_config(
        device_type: DeviceType,
        host: &cpal::Host,
        preferred_device_name: Option<&str>,
    ) -> Result<(cpal::Device, SupportedStreamConfig, bool)> {
        let (mut device, mut is_fallback) =
            Self::select_device(device_type, host, preferred_device_name)?;

        let (stream_config, _) = match Self::pick_best_stream_config(&device, device_type) {
            Ok(stream_config) => stream_config,
            Err(err) => {
                tracing::warn!(?err, device = ?DeviceDebug(&device), "Failed to pick stream config for preferred device, picking best fallback device");

                let devices = Self::host_devices(device_type, host)?;
                let mut best_fallback: Option<(
                    cpal::Device,
                    SupportedStreamConfig,
                    StreamConfigScore,
                )> = None;

                for dev in devices {
                    if let Ok((config, score)) = Self::pick_best_stream_config(&dev, device_type) {
                        match &mut best_fallback {
                            None => best_fallback = Some((dev, config, score)),
                            Some((_, _, best_score)) => {
                                if score < *best_score {
                                    *best_score = score;
                                    best_fallback = Some((dev, config, score))
                                }
                            }
                        }
                    }
                }

                if let Some((dev, config, score)) = best_fallback {
                    tracing::info!(device = ?DeviceDebug(&dev), ?config, "Selected fallback device");
                    device = dev;
                    is_fallback = true;
                    (config, score)
                } else {
                    anyhow::bail!("No supported stream config found for any device");
                }
            }
        };

        Ok((device, stream_config, is_fallback))
    }

    #[instrument(level = "trace", err, skip(host), fields(host = ?HostDebug(host)))]
    fn host_devices(device_type: DeviceType, host: &cpal::Host) -> Result<Vec<cpal::Device>> {
        match device_type {
            DeviceType::Input => Ok(host
                .input_devices()
                .context("Failed to enumerate input devices")?
                .collect()),
            DeviceType::Output => Ok(host
                .output_devices()
                .context("Failed to enumerate output devices")?
                .collect()),
        }
    }

    #[instrument(level = "trace", err, skip(host), fields(host = ?HostDebug(host)))]
    fn select_device(
        device_type: DeviceType,
        host: &cpal::Host,
        preferred_device_name: Option<&str>,
    ) -> Result<(cpal::Device, bool)> {
        tracing::trace!("Selecting device");

        if let Some(name) = preferred_device_name {
            let devices = Self::host_devices(device_type, host)?;

            if let Some(device) = devices.iter().find(|d| {
                d.name()
                    .map(|n| n.eq_ignore_ascii_case(name))
                    .unwrap_or(false)
            }) {
                tracing::trace!(device = ?DeviceDebug(device), "Selected preferred device");
                return Ok((device.clone(), false));
            }

            if let Some(device) = devices.iter().find(|d| {
                d.name()
                    .map(|n| n.to_lowercase().contains(&name.to_lowercase()))
                    .unwrap_or(false)
            }) {
                tracing::trace!(device = ?DeviceDebug(device), "Selected preferred device (based on substring match)");
                return Ok((device.clone(), false));
            }
        }

        let device = match device_type {
            DeviceType::Input => host
                .default_input_device()
                .context("Failed to get default input device")?,
            DeviceType::Output => host
                .default_output_device()
                .context("Failed to get default output device")?,
        };
        tracing::trace!(device = ?DeviceDebug(&device), "Selected default device");
        Ok((device, true))
    }

    #[instrument(level = "trace", err, skip(device), fields(device = ?DeviceDebug(device)))]
    fn pick_best_stream_config(
        device: &cpal::Device,
        device_type: DeviceType,
    ) -> Result<(SupportedStreamConfig, StreamConfigScore)> {
        tracing::trace!("Picking best stream config");

        let (configs, preferred_channels): (Vec<SupportedStreamConfigRange>, u16) =
            match device_type {
                DeviceType::Input => (
                    device
                        .supported_input_configs()
                        .context("Failed to get supported input configs")?
                        .collect(),
                    1,
                ),
                DeviceType::Output => (
                    device
                        .supported_output_configs()
                        .context("Failed to get supported output configs")?
                        .collect(),
                    2,
                ),
            };

        let mut best: Option<(SupportedStreamConfigRange, StreamConfigScore)> = None;

        for range in configs {
            let score = Self::score_stream_config_range(&range, preferred_channels);
            match &mut best {
                None => best = Some((range, score)),
                Some((_, best_score)) => {
                    if score < *best_score {
                        *best_score = score;
                        best = Some((range, score));
                    }
                }
            }
        }

        let (range, score) =
            best.ok_or_else(|| anyhow::anyhow!("No supported stream config found"))?;
        let sample_rate =
            Self::closest_sample_rate(range.min_sample_rate().0, range.max_sample_rate().0);

        tracing::trace!(?range, ?score, ?sample_rate, "Picked best stream config");
        Ok((range.with_sample_rate(cpal::SampleRate(sample_rate)), score))
    }

    fn score_stream_config_range(
        range: &SupportedStreamConfigRange,
        preferred_channels: u16,
    ) -> StreamConfigScore {
        let sample_rate_distance =
            Self::sample_rate_distance(range.min_sample_rate().0, range.max_sample_rate().0);

        let channels_distance = range.channels().abs_diff(preferred_channels);

        let format_preference = match range.sample_format() {
            SampleFormat::F32 => 0,
            SampleFormat::I16 => 1,
            SampleFormat::U16 => 2,
            _ => 3,
        };

        StreamConfigScore(sample_rate_distance, channels_distance, format_preference)
    }

    fn sample_rate_distance(min: u32, max: u32) -> u32 {
        if min <= TARGET_SAMPLE_RATE && max >= TARGET_SAMPLE_RATE {
            0
        } else if TARGET_SAMPLE_RATE < min {
            min - TARGET_SAMPLE_RATE
        } else {
            TARGET_SAMPLE_RATE - max
        }
    }

    fn closest_sample_rate(min: u32, max: u32) -> u32 {
        if min <= TARGET_SAMPLE_RATE && max >= TARGET_SAMPLE_RATE {
            TARGET_SAMPLE_RATE
        } else if TARGET_SAMPLE_RATE < min {
            min
        } else {
            max
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct StreamConfigScore(u32, u16, u8); // sample_rate_distance, channels_distance, format_preference

impl Ord for StreamConfigScore {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.0, self.1, self.2).cmp(&(other.0, other.1, other.2))
    }
}
impl PartialOrd for StreamConfigScore {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

struct DeviceDebug<'a>(&'a cpal::Device);

impl<'a> Debug for DeviceDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Device")
            .field(&self.0.name().unwrap_or_default())
            .finish()
    }
}

struct HostDebug<'a>(&'a cpal::Host);

impl<'a> Debug for HostDebug<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Host").field(&self.0.id().name()).finish()
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
    #[instrument(level = "trace", err)]
    pub fn new(config: &AudioDeviceConfig, device_type: DeviceType) -> anyhow::Result<Self> {
        tracing::trace!("Initialising device");

        let host = find_host(config.host_name.as_deref())?;
        let device = find_device(&host, &config.device_name, &device_type)?;
        let stream_config = find_supported_stream_config(&device, config, &device_type)?;
        let device = Device {
            device_type,
            stream_config,
            device,
        };

        tracing::debug!(%device, "Device initialised");
        Ok(device)
    }

    pub fn device_name(&self) -> String {
        self.device.name().unwrap_or("<unknown>".to_string())
    }

    #[instrument(level = "debug", err)]
    pub fn find_default(device_type: DeviceType) -> anyhow::Result<Self> {
        tracing::trace!("Finding default device");

        let host = find_host(None)?;
        let device = match device_type {
            DeviceType::Input => host
                .default_input_device()
                .context("Failed to get default input device")?,
            DeviceType::Output => host
                .default_output_device()
                .context("Failed to get default output device")?,
        };
        let stream_config = find_supported_stream_config(
            &device,
            &AudioDeviceConfig::from(device_type),
            &device_type,
        )?;
        let device = Device {
            device_type,
            device,
            stream_config,
        };

        tracing::debug!(%device, "Device initialised");
        Ok(device)
    }

    #[instrument(level = "debug", err)]
    pub fn find_all(device_type: DeviceType) -> anyhow::Result<Vec<Self>> {
        tracing::trace!("Finding all devices for type {device_type}");

        let host = find_host(None)?;

        let devices = match device_type {
            DeviceType::Input => host.input_devices(),
            DeviceType::Output => host.output_devices(),
        }?;

        let devices: Vec<Self> = devices
            .filter_map(|device| {
                if let Ok(stream_config) = find_supported_stream_config(
                    &device,
                    &AudioDeviceConfig::from(device_type),
                    &device_type,
                ) {
                    Some(Device {
                        device_type,
                        device,
                        stream_config,
                    })
                } else {
                    tracing::warn!(
                        device_name = device.name().unwrap_or("<unknown>".to_string()),
                        "Failed to find supported stream config for device"
                    );
                    None
                }
            })
            .collect();

        Ok(devices)
    }

    #[instrument(level = "trace")]
    pub fn find_all_hosts() -> Vec<String> {
        tracing::trace!("Finding all hosts");
        cpal::available_hosts()
            .iter()
            .map(|id| id.name().to_string())
            .collect()
    }

    #[instrument(level = "trace")]
    pub fn find_default_host() -> String {
        tracing::trace!("Finding default host");
        cpal::default_host().id().name().to_string()
    }

    #[instrument(level = "trace", err)]
    pub fn list_devices_with_supported_configs(
        host_name: &str,
        device_type: &DeviceType,
    ) -> anyhow::Result<()> {
        let host = find_host(if host_name.is_empty() {
            None
        } else {
            Some(host_name)
        })?;

        match device_type {
            DeviceType::Input => {
                let devices = host.input_devices()?;
                for device in devices {
                    let supported_configs = device.supported_input_configs()?;
                    for config in supported_configs {
                        tracing::trace!(
                            device_name = ?device.name()?,
                            channels = ?config.channels(),
                            sample_format = ?config.sample_format(),
                            min_sample_rate = ?config.min_sample_rate().0,
                            max_sample_rate = ?config.max_sample_rate().0,
                            "Supported input config");
                    }
                }
            }
            DeviceType::Output => {
                let devices = host.output_devices()?;
                for device in devices {
                    let supported_configs = device.supported_output_configs()?;
                    for config in supported_configs {
                        tracing::trace!(
                            device_name = ?device.name()?,
                            channels = ?config.channels(),
                            sample_format = ?config.sample_format(),
                            min_sample_rate = ?config.min_sample_rate().0,
                            max_sample_rate = ?config.max_sample_rate().0,
                            "Supported output config");
                    }
                }
            }
        };

        Ok(())
    }
}

#[instrument(level = "trace", err)]
fn find_host(host_name: Option<&str>) -> anyhow::Result<cpal::Host> {
    tracing::trace!("Trying to find audio host");

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

#[instrument(level = "trace", skip(host), err)]
fn find_device(
    host: &cpal::Host,
    device_name: &Option<String>,
    device_type: &DeviceType,
) -> anyhow::Result<cpal::Device> {
    tracing::trace!("Trying to find device");

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

#[instrument(level = "trace", skip(device), err)]
fn find_supported_stream_config(
    device: &cpal::Device,
    config: &AudioDeviceConfig,
    device_type: &DeviceType,
) -> anyhow::Result<SupportedStreamConfig> {
    tracing::trace!("Trying to find supported stream config");

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
                && c.min_sample_rate().0 <= TARGET_SAMPLE_RATE
                && c.max_sample_rate().0 >= TARGET_SAMPLE_RATE
        })
        .ok_or_else(|| anyhow::anyhow!("No supported {} stream config found", device_type))?;

    Ok(config_range.with_sample_rate(cpal::SampleRate(TARGET_SAMPLE_RATE)))
}
