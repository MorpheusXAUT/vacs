use crate::sources::{AudioSource, AudioSourceId};
use std::collections::HashMap;
use std::time::Duration;

#[derive(Default)]
pub struct Mixer {
    sources: HashMap<AudioSourceId, Box<dyn AudioSource>>,
}

impl Mixer {
    pub fn mix(&mut self, output: &mut [f32]) {
        // Initialize the output buffer by writing EQUILIBRIUM to all of its samples. AudioSources will
        // add their own samples on top of this.
        output.fill(cpal::Sample::EQUILIBRIUM);

        // Mix all sources into the output buffer, adding their samples on top of the EQUILIBRIUM.
        for src in self.sources.values_mut() {
            src.mix_into(output);
        }

        // Clamp mixed samples to [-1.0, 1.0] to avoid clipping.
        for sample in output {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }

    pub fn add_source(&mut self, source_id: AudioSourceId, source: Box<dyn AudioSource>) {
        self.sources.insert(source_id, source);
    }

    pub fn remove_source(&mut self, source_id: AudioSourceId) {
        self.sources.remove(&source_id);
    }

    pub fn start_source(&mut self, source_id: AudioSourceId) {
        if let Some(source) = self.sources.get_mut(&source_id) {
            source.start();
        }
    }

    pub fn stop_source(&mut self, source_id: AudioSourceId) {
        if let Some(source) = self.sources.get_mut(&source_id) {
            source.stop();
        }
    }

    pub fn restart_source(&mut self, source_id: AudioSourceId) {
        if let Some(source) = self.sources.get_mut(&source_id) {
            source.restart();
        }
    }

    pub fn set_source_volume(&mut self, source_id: AudioSourceId, volume: f32) {
        if let Some(source) = self.sources.get_mut(&source_id) {
            source.set_volume(volume);
        }
    }

    pub fn skip_in_source(&mut self, source_id: AudioSourceId, duration: Duration) {
        if let Some(source) = self.sources.get_mut(&source_id) {
            source.skip(duration);
        }
    }

    pub fn rewind_in_source(&mut self, source_id: AudioSourceId, duration: Duration) {
        if let Some(source) = self.sources.get_mut(&source_id) {
            source.rewind(duration);
        }
    }
}
