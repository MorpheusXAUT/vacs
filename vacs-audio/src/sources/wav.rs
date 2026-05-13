use crate::sources::AudioSource;
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};
use std::path::Path;

pub struct WavSource {
    samples: Vec<f32>, // mono f32, resampled to output sample_rate

    output_channels: usize,
    volume: f32,

    active: bool,
    pos: usize,
}

impl WavSource {
    pub fn from_file(
        path: impl AsRef<Path>,
        sample_rate: u32,
        output_channels: usize,
        volume: f32,
    ) -> Result<Self, hound::Error> {
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

        let mut mono = mix_to_mono(interleaved, file_channels);

        if spec.sample_rate != sample_rate {
            mono = resample(&mono, spec.sample_rate, sample_rate);
        }

        Ok(Self {
            samples: mono,
            pos: 0,
            output_channels: output_channels.max(1),
            volume: volume.clamp(0.0, 1.0),
            active: false,
        })
    }
}

impl AudioSource for WavSource {
    fn mix_into(&mut self, output: &mut [f32]) {
        if !self.active || self.volume == 0.0 || self.pos >= self.samples.len() {
            return;
        }

        for frame in output.chunks_mut(self.output_channels) {
            if self.pos >= self.samples.len() {
                self.active = false;
                break;
            }
            let sample = self.samples[self.pos] * self.volume;
            self.pos += 1;
            for s in frame.iter_mut() {
                *s += sample;
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
}

fn mix_to_mono(interleaved: Vec<f32>, channels: usize) -> Vec<f32> {
    if channels == 1 {
        return interleaved;
    }
    interleaved
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn resample(samples: &[f32], in_rate: u32, out_rate: u32) -> Vec<f32> {
    let in_rate = in_rate as usize;
    let out_rate = out_rate as usize;

    let mut resampler = Fft::<f32>::new(in_rate, out_rate, 1024, 2, 1, FixedSync::Input)
        .expect("Failed to construct WAV resampler");

    let input_frames = samples.len();
    let output_frames = resampler.process_all_needed_output_len(input_frames);
    let mut out = vec![0.0f32; output_frames];

    let input_adapter = InterleavedSlice::new(samples, 1, input_frames)
        .expect("Failed to create resampler input adapter");
    let mut output_adapter = InterleavedSlice::new_mut(&mut out, 1, output_frames)
        .expect("Failed to create resampler output adapter");

    resampler
        .process_all_into_buffer(&input_adapter, &mut output_adapter, input_frames, None)
        .expect("Failed to resample WAV audio");

    out
}
