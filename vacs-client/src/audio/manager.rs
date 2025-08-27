use crate::config::AudioConfig;
use crate::error::Error;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use vacs_audio::device::{DeviceSelector, DeviceType};
use vacs_audio::error::AudioError;
use vacs_audio::sources::opus::OpusSource;
use vacs_audio::sources::waveform::{Waveform, WaveformSource, WaveformTone};
use vacs_audio::sources::AudioSourceId;
use vacs_audio::stream::capture::{CaptureStream, InputLevel};
use vacs_audio::stream::playback::PlaybackStream;
use vacs_audio::EncodedAudioFrame;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum SourceType {
    Opus,
    Ring,
    Ringback,
    RingbackOneshot,
    Click,
}

impl SourceType {
    fn into_waveform_source(
        self,
        sample_rate: f32,
        output_channels: usize,
        volume: f32,
    ) -> WaveformSource {
        match self {
            SourceType::Opus => {
                unimplemented!("Cannot create waveform source for Opus SourceType")
            }
            SourceType::Ring => WaveformSource::new(
                WaveformTone::new(497.0, Waveform::Triangle, 0.2),
                Duration::from_secs_f32(1.69),
                None,
                Duration::from_millis(10),
                sample_rate,
                output_channels,
                volume,
            ),
            SourceType::Ringback => WaveformSource::new(
                WaveformTone::new(425.0, Waveform::Sine, 0.2),
                Duration::from_secs(1),
                Some(Duration::from_secs(4)),
                Duration::from_millis(10),
                sample_rate,
                output_channels,
                volume,
            ),
            SourceType::RingbackOneshot => WaveformSource::new(
                WaveformTone::new(425.0, Waveform::Sine, 0.2),
                Duration::from_secs(1),
                None,
                Duration::from_millis(10),
                sample_rate,
                2,
                volume,
            ),
            SourceType::Click => WaveformSource::new(
                WaveformTone::new(4000.0, Waveform::Sine, 0.2),
                Duration::from_millis(20),
                None,
                Duration::from_millis(1),
                sample_rate,
                output_channels,
                volume,
            ),
        }
    }
}

pub struct AudioManager {
    output: PlaybackStream,
    input: Option<CaptureStream>,
    source_ids: HashMap<SourceType, AudioSourceId>,
}

impl AudioManager {
    pub fn new(audio_config: &AudioConfig) -> Result<Self, Error> {
        let (output, source_ids) = Self::create_playback_stream(audio_config)?;

        Ok(Self {
            output,
            input: None,
            source_ids,
        })
    }

    pub fn output_device_name(&self) -> String {
        self.output.device_name()
    }

    pub fn switch_output_device(&mut self, audio_config: &AudioConfig) -> Result<(), Error> {
        let (output, source_ids) = Self::create_playback_stream(audio_config)?;
        self.output = output;
        self.source_ids = source_ids;
        Ok(())
    }

    pub fn attach_input_device(
        &mut self,
        audio_config: &AudioConfig,
        tx: mpsc::Sender<EncodedAudioFrame>,
    ) -> Result<(), Error> {
        // TODO handle is_fallback?
        let (device, _) = DeviceSelector::open(
            DeviceType::Input,
            audio_config.host_name.as_deref(),
            audio_config.input_device_name.as_deref(),
        )?;
        self.input = Some(
            CaptureStream::start(
                device,
                tx,
                audio_config.input_device_volume,
                audio_config.input_device_volume_amp,
            )?,
        );
        Ok(())
    }

    pub fn attach_input_level_meter(
        &mut self,
        audio_config: &AudioConfig,
        emit: Box<dyn Fn(InputLevel) + Send>,
    ) -> Result<(), Error> {
        // TODO handle is_fallback?
        let (device, _) = DeviceSelector::open(
            DeviceType::Input,
            audio_config.host_name.as_deref(),
            audio_config.input_device_name.as_deref(),
        )?;
        self.input = Some(
            CaptureStream::start_level_meter(
                device,
                emit,
                audio_config.input_device_volume,
                audio_config.input_device_volume_amp,
            )?,
        );
        Ok(())
    }

    pub fn is_input_device_attached(&self) -> bool {
        self.input.is_some()
    }

    pub fn detach_input_device(&mut self) {
        self.input = None;
        log::info!("Detached input device");
    }

    pub fn start(&mut self, source_type: SourceType) {
        log::trace!("Starting audio source {source_type:?}");
        self.output
            .start_audio_source(self.source_ids[&source_type])
    }

    pub fn restart(&mut self, source_type: SourceType) {
        log::trace!("Restarting audio source {source_type:?}");
        self.output
            .restart_audio_source(self.source_ids[&source_type])
    }

    pub fn stop(&mut self, source_type: SourceType) {
        log::trace!("Stopping audio source {source_type:?}");
        self.output.stop_audio_source(self.source_ids[&source_type])
    }

    pub fn set_output_volume(&mut self, source_type: SourceType, volume: f32) {
        if !self.source_ids.contains_key(&source_type) {
            log::trace!(
                "Tried to set output volume {volume} for missing audio source {source_type:?}, skipping"
            );
            return;
        }

        log::trace!("Setting output volume {volume} for audio source {source_type:?}");
        self.output
            .set_volume(self.source_ids[&source_type], volume);

        match source_type {
            SourceType::Ring | SourceType::Click | SourceType::RingbackOneshot => {
                self.output
                    .restart_audio_source(self.source_ids[&source_type]);
            }
            _ => {}
        }
    }

    pub fn set_input_volume(&mut self, volume: f32) {
        if let Some(input) = &mut self.input {
            input.set_volume(volume);
        }
    }

    pub fn set_input_muted(&mut self, muted: bool) {
        if let Some(input) = &mut self.input {
            input.set_muted(muted);
        }
    }

    pub fn attach_call_output(
        &mut self,
        webrtc_rx: mpsc::Receiver<EncodedAudioFrame>,
        volume: f32,
        amp: f32,
    ) -> Result<(), Error> {
        if self.source_ids.contains_key(&SourceType::Opus) {
            log::warn!("Tried to attach call but a call was already attached");
            return Err(Error::AudioDevice(Box::new(AudioError::Other(
                anyhow::anyhow!("Tried to attach call but a call was already attached"),
            ))));
        }

        self.source_ids.insert(
            SourceType::Opus,
            self.output.add_audio_source(Box::new(OpusSource::new(
                webrtc_rx,
                self.output.resampler()?,
                self.output.channels(),
                volume,
                amp,
            )?)),
        );
        log::info!("Attached call");

        Ok(())
    }

    pub fn detach_call_output(&mut self) {
        if let Some(source_id) = self.source_ids.remove(&SourceType::Opus) {
            self.output.remove_audio_source(source_id);
            log::info!("Detached call output");
        } else {
            log::info!("Tried to detach call output but no call was attached");
        }
    }

    fn create_playback_stream(
        audio_config: &AudioConfig,
    ) -> Result<(PlaybackStream, HashMap<SourceType, AudioSourceId>), Error> {
        // TODO: handle is_fallback?
        let (output_device, _) = DeviceSelector::open(
            DeviceType::Output,
            audio_config.host_name.as_deref(),
            audio_config.output_device_name.as_deref(),
        )?;

        let sample_rate = output_device.sample_rate() as f32;
        let channels = output_device.channels() as usize;

        let mut output =
            PlaybackStream::start(output_device)?;

        let mut source_ids = HashMap::new();
        source_ids.insert(
            SourceType::Ring,
            output.add_audio_source(Box::new(SourceType::into_waveform_source(
                SourceType::Ring,
                sample_rate,
                channels,
                audio_config.chime_volume,
            ))),
        );
        source_ids.insert(
            SourceType::Ringback,
            output.add_audio_source(Box::new(SourceType::into_waveform_source(
                SourceType::Ringback,
                sample_rate,
                channels,
                audio_config.output_device_volume,
            ))),
        );
        source_ids.insert(
            SourceType::RingbackOneshot,
            output.add_audio_source(Box::new(SourceType::into_waveform_source(
                SourceType::RingbackOneshot,
                sample_rate,
                channels,
                audio_config.output_device_volume,
            ))),
        );
        source_ids.insert(
            SourceType::Click,
            output.add_audio_source(Box::new(SourceType::into_waveform_source(
                SourceType::Click,
                sample_rate,
                channels,
                audio_config.click_volume,
            ))),
        );

        Ok((output, source_ids))
    }
}
