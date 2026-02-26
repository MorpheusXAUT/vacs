use crate::TARGET_SAMPLE_RATE;
use crate::sources::AudioSource;
use std::time::Duration;
use tracing::instrument;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Waveform {
    Sine,
    Triangle,
    Square,
    Sawtooth,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaveformSegment {
    Tone(WaveformTone, Duration),
    Silence(Duration),
}

impl WaveformSegment {
    pub fn new(tone: WaveformTone, duration: Duration) -> Self {
        Self::Tone(tone, duration)
    }

    pub fn pause(duration: Duration) -> Self {
        Self::Silence(duration)
    }

    pub fn duration(&self) -> Duration {
        match self {
            Self::Tone(_, d) => *d,
            Self::Silence(d) => *d,
        }
    }
}

impl From<(WaveformTone, Duration)> for WaveformSegment {
    fn from((tone, duration): (WaveformTone, Duration)) -> Self {
        Self::Tone(tone, duration)
    }
}

impl From<Duration> for WaveformSegment {
    fn from(duration: Duration) -> Self {
        Self::Silence(duration)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WaveformSequence {
    pub segments: Vec<WaveformSegment>,
}

impl From<Vec<(WaveformTone, Duration)>> for WaveformSequence {
    fn from(tones: Vec<(WaveformTone, Duration)>) -> Self {
        Self {
            segments: tones.into_iter().map(WaveformSegment::from).collect(),
        }
    }
}

impl From<Vec<WaveformSegment>> for WaveformSequence {
    fn from(segments: Vec<WaveformSegment>) -> Self {
        Self { segments }
    }
}

impl From<WaveformSegment> for WaveformSequence {
    fn from(segment: WaveformSegment) -> Self {
        Self {
            segments: vec![segment],
        }
    }
}

impl WaveformSequence {
    pub fn new(segments: Vec<WaveformSegment>) -> Self {
        Self { segments }
    }

    pub fn single(tone: WaveformTone, duration: Duration) -> Self {
        Self {
            segments: vec![WaveformSegment::new(tone, duration)],
        }
    }

    pub fn repeat(self, n: usize) -> Self {
        let mut new_segments = Vec::with_capacity(self.segments.len() * n);
        for _ in 0..n {
            new_segments.extend(self.segments.clone());
        }
        Self {
            segments: new_segments,
        }
    }

    pub fn concat(mut self, other: impl Into<WaveformSequence>) -> Self {
        self.segments.extend(other.into().segments);
        self
    }
}

#[derive(Debug, Clone, Copy)]
struct PreparedSegment {
    tone: Option<WaveformTone>,
    samples: usize,
}

pub struct WaveformSource {
    segments: Vec<PreparedSegment>,

    output_channels: usize, // >= 1
    volume: f32,            // 0.0 - 1.0

    attack_samples: usize,
    release_samples: usize,
    env_pos: usize,

    active: bool,
    releasing: bool,
    restarting: bool,

    sample_rate: f32,
    total_tone_samples: usize, // duration of all tones in samples
    silence_samples: usize,    // duration of silence in samples
    restart_samples: usize,    // duration of restart silence in samples
    looped: bool,

    current_segment_idx: usize,
    segment_elapsed: usize,
    cycle_pos: usize, // position inside cycle
}

impl WaveformSource {
    pub fn new(
        sequence: impl Into<WaveformSequence>,
        pause_dur: Option<Duration>,
        fade_dur: Duration,
        sample_rate: f32,
        output_channels: usize,
        volume: f32,
    ) -> Self {
        let sequence = sequence.into();
        let mut segments = Vec::with_capacity(sequence.segments.len());
        let mut total_tone_samples = 0;

        for seg in sequence.segments {
            let (tone, duration) = match seg {
                WaveformSegment::Tone(t, d) => {
                    assert!(t.freq > 0.0, "Tone frequency must be greater than 0");
                    assert!(t.amp > 0.0, "Tone amplitude must be greater than 0");
                    (Some(t), d)
                }
                WaveformSegment::Silence(d) => (None, d),
            };

            assert!(
                duration > Duration::new(0, 0),
                "Segment duration must be greater than 0"
            );

            let samples = (duration.as_secs_f32() * sample_rate) as usize;
            total_tone_samples += samples;
            segments.push(PreparedSegment { tone, samples });
        }

        Self {
            segments,

            output_channels: output_channels.max(1),
            volume: volume.clamp(0.0, 1.0),

            attack_samples: (fade_dur.as_secs_f32() * sample_rate) as usize,
            release_samples: (fade_dur.as_secs_f32() * sample_rate) as usize,
            env_pos: 0,

            active: false,
            releasing: false,
            restarting: false,

            sample_rate,
            total_tone_samples,
            silence_samples: pause_dur.map_or(0, |p| (p.as_secs_f32() * sample_rate) as usize),
            restart_samples: (TARGET_SAMPLE_RATE / 10) as usize,
            looped: pause_dur.is_some(),

            current_segment_idx: 0,
            segment_elapsed: 0,
            cycle_pos: 0,
        }
    }

    pub fn single(
        tone: WaveformTone,
        duration: Duration,
        pause_dur: Option<Duration>,
        fade_dur: Duration,
        sample_rate: f32,
        output_channels: usize,
        volume: f32,
    ) -> Self {
        Self::new(
            WaveformSequence::single(tone, duration),
            pause_dur,
            fade_dur,
            sample_rate,
            output_channels,
            volume,
        )
    }

    fn generate_waveform(&self) -> f32 {
        let segment = &self.segments[self.current_segment_idx];
        let tone = match segment.tone {
            Some(t) => t,
            None => return 0.0,
        };

        // Reset phase at the start of each segment
        let time = self.segment_elapsed as f32 / self.sample_rate;
        let phase = (time * tone.freq).rem_euclid(1.0);
        match tone.form {
            Waveform::Sine => {
                let t = 2.0 * std::f32::consts::PI * tone.freq * time;
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

    fn generate_release_envelope(&self) -> f32 {
        if self.releasing {
            if self.release_samples == 0 {
                return 0.0;
            }
            let rel = self.env_pos.min(self.release_samples);
            1.0 - rel as f32 / self.release_samples as f32
        } else {
            1.0
        }
    }

    fn generate_segment_envelope(&self) -> f32 {
        let segment_len = self.segments[self.current_segment_idx].samples;
        let remaining = segment_len.saturating_sub(self.segment_elapsed);

        // Fade In (Attack)
        let start_val = if self.attack_samples > 0 {
            (self.segment_elapsed as f32 / self.attack_samples as f32).min(1.0)
        } else {
            1.0
        };

        // Fade Out (Release)
        let end_val = if self.release_samples > 0 {
            (remaining as f32 / self.release_samples as f32).min(1.0)
        } else {
            1.0
        };

        start_val.min(end_val)
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

            // Only generate tone if cycle_pos is inside tone cycle (0->total_tone_samples)
            if self.cycle_pos < self.total_tone_samples {
                // Generate tone
                let mut segment_val = self.generate_waveform();

                // Apply envelope
                segment_val *= self.generate_segment_envelope();
                segment_val *= self.generate_release_envelope();

                let segment_amp = self.segments[self.current_segment_idx]
                    .tone
                    .map_or(0.0, |t| t.amp);
                sample = segment_val * segment_amp;

                self.env_pos += 1;

                // Advance inside segment
                self.segment_elapsed += 1;
                if self.segment_elapsed >= self.segments[self.current_segment_idx].samples {
                    // Move to next segment if available
                    if self.current_segment_idx + 1 < self.segments.len() {
                        self.current_segment_idx += 1;
                        self.segment_elapsed = 0;
                    }
                    // Else we are at the end of the last segment, next cycle_pos increment will handle the transition to silence/restart
                }
            } else if self.releasing && !self.restarting {
                // Stop if playing silence, releasing and not restarting
                self.active = false;
                self.releasing = false;
                break;
            }

            // Mix into the output buffer
            for s in frame.iter_mut() {
                *s += sample * self.volume;
            }

            // Advance cycle position
            self.cycle_pos += 1;

            // Cycle length is either tone+silence or tone+restart
            let cycle_len = self.total_tone_samples
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
                    // Reset segment state
                    self.current_segment_idx = 0;
                    self.segment_elapsed = 0;
                } else if self.looped {
                    // Reset cycle
                    self.cycle_pos = 0;
                    self.env_pos = 0;
                    self.current_segment_idx = 0;
                    self.segment_elapsed = 0;
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
                    self.cycle_pos = self.total_tone_samples;
                    self.current_segment_idx = self.segments.len().saturating_sub(1);
                    self.segment_elapsed = self.segments.last().map_or(0, |s| s.samples);
                } else {
                    // Stop
                    self.active = false;
                    break;
                }
            }
        }
    }

    #[instrument(level = "trace", skip(self), fields(segment_count = self.segments.len()))]
    fn start(&mut self) {
        self.active = true;
        self.releasing = false;
        self.restarting = false;
        self.env_pos = 0;
        self.cycle_pos = 0;
        self.current_segment_idx = 0;
        self.segment_elapsed = 0;
    }

    #[instrument(level = "trace", skip(self), fields(segment_count = self.segments.len()))]
    fn stop(&mut self) {
        // If we are currently releasing, we ignore the call to stop.
        // If not, we initiate the release. In case we are stopping while playing silence,
        // mix_into will abort early.
        if self.active && !self.releasing {
            self.releasing = true;
            self.env_pos = 0;
        }
    }

    #[instrument(level = "trace", skip(self), fields(segment_count = self.segments.len()))]
    fn restart(&mut self) {
        if self.active {
            self.stop();
            self.restarting = true;
        } else {
            self.start();
        }
    }

    #[instrument(level = "trace", skip(self), fields(segment_count = self.segments.len()))]
    fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }
}
