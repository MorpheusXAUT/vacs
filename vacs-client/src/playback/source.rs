//! Audio source abstraction for the playback recorder.
//!
//! Implementations produce a stream of [`PlaybackSourceEvent`] values containing both PCM
//! frames and gating signals. The recorder is backend-agnostic and consumes whatever
//! `TapId` variants the source emits.
//!
//! Concrete sources live in OS-specific submodules. They differ in *both* the gating
//! signal source (e.g. TrackAudio's WebSocket events) and the PCM capture backend
//! (PipeWire on Linux, WASAPI loopback on Windows, ScreenCaptureKit-Audio on macOS),
//! and there is no meaningful overlap to share.

use crate::playback::{PlaybackError, TapId};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use trackaudio::Frequency;

/// One unit of work emitted by an [`PlaybackSource`].
#[derive(Debug, Clone)]
#[cfg_attr(not(any(target_os = "linux", target_os = "windows")), allow(dead_code))]
pub enum PlaybackSourceEvent {
    TapOpened {
        tap: TapId,
        sample_rate: u32,
        channels: u16,
    },
    TapClosed {
        tap: TapId,
    },
    Frame {
        tap: TapId,
        samples: Arc<[f32]>,
        captured_at: Instant,
    },
    RxBegin {
        tap: TapId,
        callsign: String,
        frequency: Frequency,
    },
    RxEnd {
        callsign: String,
        frequency: Frequency,
        active_transmitters: Option<Vec<String>>,
    },
}

/// A source of playback audio + gating events.
#[async_trait::async_trait]
pub trait PlaybackSource: Send {
    async fn start(&mut self) -> Result<mpsc::Receiver<PlaybackSourceEvent>, PlaybackError>;
    async fn stop(&mut self);
}

pub mod capture;

#[cfg(any(target_os = "linux", target_os = "windows"))]
pub mod track_audio;

#[cfg(target_os = "windows")]
pub mod audio_for_vatsim;

#[cfg(any(target_os = "linux", target_os = "windows"))]
pub use track_audio::TrackAudioLoopbackSource;

#[cfg(target_os = "windows")]
pub use audio_for_vatsim::AudioForVatsimLoopbackSource;
