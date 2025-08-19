use crate::SAMPLE_RATE;
use crate::sources::AudioSource;
use std::time::Duration;
use tracing::instrument;

#[derive(Debug, Clone, Copy)]
pub enum Waveform {
    Sine,
    Triangle,
    Square,
    Sawtooth,
}

#[derive(Debug, Clone, Copy)]
pub struct WaveformTone {
    pub freq: f32, // Hz
    pub form: Waveform,
    pub amp: f32, // 0.0 - 1.0
}

impl WaveformTone {
    pub fn new(freq: f32, form: Waveform, amp: f32) -> Self {
        Self { freq, form, amp }
    }
}

pub struct WaveformSource {
    tone: WaveformTone,

    output_channels: usize, // >= 1
    volume: f32,            // 0.0 - 1.0

    attack_samples: usize,
    release_samples: usize,
    env_pos: usize,

    active: bool,
    releasing: bool,
    restarting: bool,

    sample_rate: f32,
    tone_samples: usize,    // duration of tone in samples
    silence_samples: usize, // duration of silence in samples
    restart_samples: usize, // duration of restart silence in samples
    looped: bool,

    cycle_pos: usize, // position inside cycle
}

impl WaveformSource {
    pub fn new(
        tone: WaveformTone,
        tone_dur: Duration,
        pause_dur: Option<Duration>,
        fade_dur: Duration,
        output_channels: usize,
        volume: f32,
    ) -> Self {
        let sample_rate = SAMPLE_RATE as f32;

        assert!(tone.freq > 0.0, "Tone frequency must be greater than 0");
        assert!(tone.amp > 0.0, "Tone amplitude must be greater than 0");
        assert!(
            tone_dur > Duration::new(0, 0),
            "Tone duration must be greater than 0"
        );
        assert!(
            fade_dur > Duration::new(0, 0),
            "Fade duration must be greater than 0"
        );

        Self {
            tone,

            output_channels: output_channels.max(1),
            volume: volume.clamp(0.0, 1.0),

            attack_samples: (fade_dur.as_secs_f32() * sample_rate) as usize,
            release_samples: (fade_dur.as_secs_f32() * sample_rate) as usize,
            env_pos: 0,

            active: false,
            releasing: false,
            restarting: false,

            sample_rate,
            tone_samples: (tone_dur.as_secs_f32() * sample_rate) as usize,
            silence_samples: pause_dur.map_or(0, |p| (p.as_secs_f32() * sample_rate) as usize),
            restart_samples: (SAMPLE_RATE / 10) as usize,
            looped: pause_dur.is_some(),

            cycle_pos: 0,
        }
    }

    fn generate_waveform(&self) -> f32 {
        let time = self.cycle_pos as f32 / self.sample_rate;
        let phase = (time * self.tone.freq).rem_euclid(1.0);
        match self.tone.form {
            Waveform::Sine => {
                let t = 2.0 * std::f32::consts::PI * self.tone.freq * time;
                t.sin()
            }
            Waveform::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
            Waveform::Square => {
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Sawtooth => 2.0 * phase - 1.0,
        }
    }

    fn generate_envelope(&self) -> f32 {
        if self.releasing {
            let rel = self.env_pos.min(self.release_samples);
            1.0 - rel as f32 / self.release_samples as f32
        } else {
            // Check if we're near the end of the beep and need to apply release envelope
            let remaining_samples = self.tone_samples - self.cycle_pos;
            if remaining_samples <= self.release_samples {
                // Apply release envelope for natural end of beep
                let release_progress = self.release_samples - remaining_samples;
                let rel_amp = 1.0 - release_progress as f32 / self.release_samples as f32;

                // Also apply attack envelope if we're still in attack phase
                let att = self.env_pos.min(self.attack_samples);
                let att_amp = att as f32 / self.attack_samples as f32;

                att_amp.min(rel_amp)
            } else {
                // Normal attack envelope
                let att = self.env_pos.min(self.attack_samples);
                att as f32 / self.attack_samples as f32
            }
        }
    }
}

impl AudioSource for WaveformSource {
    fn mix_into(&mut self, output: &mut [f32]) {
        // Abort early if not active or muted
        if !self.active || self.volume == 0.0 {
            return;
        }

        for frame in output.chunks_mut(self.output_channels) {
            let mut sample = 0.0;

            // Only generate tone if cycle_pos is inside tone cycle (0->tone_samples)
            if self.cycle_pos < self.tone_samples {
                // Generate tone
                sample = self.generate_waveform();

                // Apply envelope
                sample *= self.generate_envelope();

                self.env_pos += 1;
            } else if self.releasing && !self.restarting {
                // Stop if playing silence, releasing and not restarting
                self.active = false;
                self.releasing = false;
                break;
            }

            // Mix into the output buffer
            for s in frame.iter_mut() {
                *s += sample * self.tone.amp * self.volume;
            }

            // Advance cycle position
            self.cycle_pos += 1;

            // Cycle length is either tone+silence or tone+restart
            let cycle_len = self.tone_samples
                + if self.restarting {
                    self.restart_samples
                } else {
                    self.silence_samples
                };

            if self.cycle_pos >= cycle_len {
                if self.restarting {
                    // Restart silence is completed. Reset state and cycle.
                    self.restarting = false;
                    self.releasing = false;
                    self.cycle_pos = 0;
                    self.env_pos = 0;
                } else if self.looped {
                    // Reset cycle
                    self.cycle_pos = 0;
                    self.env_pos = 0;
                } else {
                    // Stop
                    self.active = false;
                    break;
                }
            }

            // Check if envelope completed
            if self.releasing && self.env_pos >= self.release_samples {
                self.releasing = false;

                if self.restarting {
                    // Set cycle position after tone, so that the restart silence is immediately applied,
                    // even if we initiated the restart during the tone.
                    self.cycle_pos = self.tone_samples;
                } else {
                    // Stop
                    self.active = false;
                    break;
                }
            }
        }
    }

    #[instrument(level = "trace", skip(self), fields(
        tone = self.tone.freq,
        form = ?self.tone.form,
        amp = self.tone.amp,
    ))]
    fn start(&mut self) {
        tracing::trace!("Starting waveform source");
        self.active = true;
        self.releasing = false;
        self.restarting = false;
        self.env_pos = 0;
        self.cycle_pos = 0;
    }

    #[instrument(level = "trace", skip(self), fields(
        tone = self.tone.freq,
        form = ?self.tone.form,
        amp = self.tone.amp,
    ))]
    fn stop(&mut self) {
        tracing::trace!("Stopping waveform source");
        // If we are currently releasing, we ignore the call to stop.
        // If not, we initiate the release. In case we are stopping while playing silence,
        // mix_into will abort early.
        if self.active && !self.releasing {
            self.releasing = true;
            self.env_pos = 0;
        }
    }

    #[instrument(level = "trace", skip(self), fields(
        tone = self.tone.freq,
        form = ?self.tone.form,
        amp = self.tone.amp,
    ))]
    fn restart(&mut self) {
        tracing::trace!("Restarting waveform source");
        if self.active {
            self.stop();
            self.restarting = true;
        } else {
            self.start();
        }
    }

    #[instrument(level = "trace", skip(self), fields(
        tone = self.tone.freq,
        form = ?self.tone.form,
        amp = self.tone.amp,
    ))]
    fn set_volume(&mut self, volume: f32) {
        tracing::trace!("Setting volume for waveform source");
        self.volume = volume.clamp(0.0, 1.0);
    }
}
