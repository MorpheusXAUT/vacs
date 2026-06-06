use crate::dsp::downmix_interleaved_to_mono;
use crate::sources::AudioSource;
use anyhow::{Context, Result};
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};
use std::path::Path;
use std::time::Duration;

pub struct WavSource {
    samples: Vec<f32>, // mono f32, resampled to output sample_rate

    sample_rate: u32,
    output_channels: usize,
    volume: f32,

    active: bool,
    pos: usize,

    update_interval: usize,                     // in ms, defaults to 100ms
    on_update: Option<Box<dyn Fn(f32) + Send>>, // progress from 0.0 to 1.0
}

impl WavSource {
    pub fn from_file(
        path: impl AsRef<Path>,
        sample_rate: u32,
        output_channels: usize,
        volume: f32,
        update_interval: Option<usize>,
        on_update: Option<Box<dyn Fn(f32) + Send>>,
    ) -> Result<Self> {
        let mut reader = hound::WavReader::open(path)?;
        let spec = reader.spec();
        let file_channels = spec.channels as usize;

        let interleaved: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<_, _>>()?,
            hound::SampleFormat::Int => {
                let max_val = (1u32 << (spec.bits_per_sample - 1)) as f32;
                reader
                    .samples::<i32>()
                    .map(|s| s.map(|x| x as f32 / max_val))
                    .collect::<Result<_, _>>()?
            }
        };

        let mut samples = if file_channels == 1 {
            interleaved
        } else {
            let mut mono = Vec::new();
            downmix_interleaved_to_mono(&interleaved, file_channels, &mut mono);
            mono
        };

        if spec.sample_rate != sample_rate {
            samples = resample(&samples, spec.sample_rate as usize, sample_rate as usize)?;
        }

        Ok(Self {
            samples,
            pos: 0,
            sample_rate,
            output_channels: output_channels.max(1),
            volume: volume.clamp(0.0, 1.0),
            active: false,
            update_interval: update_interval.unwrap_or(500),
            on_update,
        })
    }
}

impl AudioSource for WavSource {
    fn mix_into(&mut self, output: &mut [f32]) {
        if !self.active || self.volume == 0.0 {
            return;
        }

        if self.samples.is_empty() {
            if let Some(on_update) = &self.on_update {
                on_update(1.0);
            }
            self.active = false;
            return;
        }

        for frame in output.chunks_mut(self.output_channels) {
            let sample = self.samples[self.pos] * self.volume;
            self.pos += 1;
            for s in frame.iter_mut() {
                *s += sample;
            }

            if self.pos >= self.samples.len() {
                if let Some(on_update) = &self.on_update {
                    on_update(1.0);
                }
                self.active = false;
                break;
            }

            if let Some(on_update) = &self.on_update
                && self.pos.is_multiple_of(self.update_interval)
            {
                let elapsed = self.pos as f32 / self.samples.len() as f32;
                on_update(elapsed);
            }
        }
    }

    fn start(&mut self) {
        self.active = true;
    }

    fn stop(&mut self) {
        self.active = false;
    }

    fn restart(&mut self) {
        self.pos = 0;
        self.active = true;
    }

    fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    fn skip(&mut self, duration: Duration) {
        let frames = (duration.as_secs_f32() * self.sample_rate as f32).round() as usize;
        self.pos = (self.pos + frames).min(self.samples.len().saturating_sub(1)); // "- 1" to allow mix_into to finish
    }

    fn rewind(&mut self, duration: Duration) {
        let frames = (duration.as_secs_f32() * self.sample_rate as f32).round() as usize;
        self.pos = self.pos - frames.min(self.pos);
    }
}

fn resample(samples: &[f32], in_rate: usize, out_rate: usize) -> anyhow::Result<Vec<f32>> {
    let mut resampler = Fft::<f32>::new(in_rate, out_rate, 1024, 2, 1, FixedSync::Input)
        .context("Failed to construct WAV resampler")?;

    let input_frames = samples.len();
    let output_frames = resampler.process_all_needed_output_len(input_frames);
    let mut out = vec![0.0f32; output_frames];

    let input_adapter = InterleavedSlice::new(samples, 1, input_frames)
        .context("Failed to create resampler input adapter")?;
    let mut output_adapter = InterleavedSlice::new_mut(&mut out, 1, output_frames)
        .context("Failed to create resampler output adapter")?;

    resampler
        .process_all_into_buffer(&input_adapter, &mut output_adapter, input_frames, None)
        .context("Failed to resample WAV audio")?;

    Ok(out)
}
