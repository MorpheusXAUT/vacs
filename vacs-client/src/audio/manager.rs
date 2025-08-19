use crate::config::AudioConfig;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use vacs_audio::input::AudioInput;
use vacs_audio::output::AudioOutput;
use vacs_audio::sources::opus::OpusSource;
use vacs_audio::sources::waveform::{Waveform, WaveformSource, WaveformTone};
use vacs_audio::sources::AudioSourceId;
use vacs_audio::{Device, DeviceType, EncodedAudioFrame};

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum SourceType {
    Opus,
    Ring,
    Ringback,
    RingbackOneshot,
    Click,
}

impl SourceType {
    fn into_waveform_source(self, output_channels: usize, volume: f32) -> WaveformSource {
        match self {
            SourceType::Opus => {
                unimplemented!("Cannot create waveform source for Opus SourceType")
            }
            SourceType::Ring => WaveformSource::new(
                WaveformTone::new(497.0, Waveform::Triangle, 0.2), // TODO: tune amp
                Duration::from_secs_f32(1.69),
                None,
                Duration::from_millis(10),
                output_channels,
                volume,
            ),
            SourceType::Ringback => WaveformSource::new(
                WaveformTone::new(425.0, Waveform::Sine, 0.2), // TODO: tune amp
                Duration::from_secs(1),
                Some(Duration::from_secs(4)),
                Duration::from_millis(10),
                output_channels,
                volume,
            ),
            SourceType::RingbackOneshot => WaveformSource::new(
                WaveformTone::new(425.0, Waveform::Sine, 0.2), // TODO: tune amp
                Duration::from_secs(1),
                None,
                Duration::from_millis(10),
                2,
                volume,
            ),
            SourceType::Click => WaveformSource::new(
                WaveformTone::new(4000.0, Waveform::Sine, 0.2), // TODO: tune amp
                Duration::from_millis(20),
                None,
                Duration::from_millis(1),
                output_channels,
                volume,
            )
        }
    }
}

pub struct AudioManager {
    output: AudioOutput,
    // TODO remove linter disable
    #[allow(dead_code)]
    input: Option<AudioInput>,
    source_ids: HashMap<SourceType, AudioSourceId>,
}

impl AudioManager {
    pub fn new(audio_config: &AudioConfig) -> Result<Self> {
        let output_device = Device::new(
            &audio_config.device_config(DeviceType::Output),
            DeviceType::Output,
        )?;
        let mut output =
            AudioOutput::start(&output_device).context("Failed to start audio output")?;

        let mut source_ids = HashMap::new();
        source_ids.insert(
            SourceType::Ring,
            output.add_audio_source(Box::new(SourceType::into_waveform_source(
                SourceType::Ring,
                output_device.stream_config.channels() as usize,
                audio_config.chime_volume,
            ))),
        );
        source_ids.insert(
            SourceType::Ringback,
            output.add_audio_source(Box::new(SourceType::into_waveform_source(
                SourceType::Ringback,
                output_device.stream_config.channels() as usize,
                audio_config.output_device_volume,
            ))),
        );
        source_ids.insert(
            SourceType::RingbackOneshot,
            output.add_audio_source(Box::new(SourceType::into_waveform_source(
                SourceType::RingbackOneshot,
                output_device.stream_config.channels() as usize,
                audio_config.output_device_volume,
            ))),
        );
        source_ids.insert(
            SourceType::Click,
            output.add_audio_source(Box::new(SourceType::into_waveform_source(
                SourceType::Click,
                output_device.stream_config.channels() as usize,
                audio_config.click_volume,
            ))),
        );

        Ok(Self {
            output,
            input: None,
            source_ids,
        })
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

    pub fn set_volume(&mut self, source_type: SourceType, volume: f32) {
        if !self.source_ids.contains_key(&source_type) {
            log::trace!(
                "Tried to set volume {volume} for missing audio source {source_type:?}, skipping"
            );
            return;
        }

        log::trace!("Setting volume {volume} for audio source {source_type:?}");
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

    // TODO remove linter disable
    #[allow(unused)]
    pub fn attach_call(&mut self, webrtc_rx: mpsc::Receiver<EncodedAudioFrame>) {
        if self.source_ids.contains_key(&SourceType::Opus) {
            log::warn!("Tried to attach call but a call was already attached");
            return;
        }

        self.source_ids.insert(
            SourceType::Opus,
            self.output
                .add_audio_source(Box::new(OpusSource::from(webrtc_rx))),
        );
        log::info!("Attached call");
    }

    // TODO remove linter disable
    #[allow(unused)]
    pub fn detach_call(&mut self) {
        self.output
            .remove_audio_source(self.source_ids[&SourceType::Opus]);
        if self.source_ids.remove(&SourceType::Opus).is_some() {
            log::info!("Detached call");
        } else {
            log::warn!("Tried to detach call but no call was attached");
        }
    }
}
