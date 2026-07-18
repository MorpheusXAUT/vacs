//! Push-to-talk radio integration for external radio clients.
//!
//! This module implements radio integration by emitting key presses to external applications
//! like Audio For VATSIM or TrackAudio when the user transmits in vacs.
//!
//! # How It Works
//!
//! When the user presses their PTT key in vacs, the `PushToTalkRadio` emits a corresponding
//! key press to the configured external radio client. This allows using a single PTT key
//! for both vacs and the radar client's radio.
//!
//! # Platform Limitations
//!
//! **This feature does NOT work on Linux/Wayland** because:
//! - Wayland's security model prevents applications from injecting input events
//! - There's no standard cross-desktop API for global input injection
//! - The `KeybindEmitter` on Linux is a no-op stub
//!
//! On Windows and macOS, this works correctly using platform-specific APIs.
//!
//! # Alternative on Linux
//!
//! Users on Linux should either:
//! - Use a radio integration that provides direct API support (not key-based)
//! - Configure vacs and their radio client separately with different PTT keys
//! - Use "Push-to-Mute" transmit mode instead of "Radio Integration"

use crate::app::state::AppState;
use crate::app::state::radio::AppStateRadioExt;
use crate::keybinds::runtime::{DynKeybindEmitter, KeybindEmitter, PlatformEmitter};
use crate::playback::PlaybackError;
use crate::radio::{Radio, RadioError, RadioState, TransmissionState};
use crate::utils::BackoffStrategy;
use keyboard_types::{Code, KeyState};
use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio_util::sync::CancellationToken;

/// Radio integration that emits key presses to external applications.
///
/// **Note**: This requires a functional `KeybindEmitter`, which is only available
/// on Windows and macOS. On Linux, the emitter is a no-op stub, so this will
/// silently do nothing.

#[derive(Clone)]
pub struct PushToTalkRadio {
    app: AppHandle,
    code: Code,
    emitter: DynKeybindEmitter,
    active: Arc<AtomicBool>,
    cancel: CancellationToken,
}

impl PushToTalkRadio {
    pub fn new(app: AppHandle, code: Code) -> Result<Self, RadioError> {
        log::trace!("PushToTalkRadio starting: code {:?}", code);

        let cancel = CancellationToken::new();
        let radio = Self {
            app: app.clone(),
            code,
            emitter: Arc::new(
                PlatformEmitter::start().map_err(|err| RadioError::Integration(err.to_string()))?,
            ),
            active: Arc::new(AtomicBool::new(false)),
            cancel: cancel.clone(),
        };

        tauri::async_runtime::spawn(async move {
            let strategy =
                BackoffStrategy::new(Duration::from_millis(500), Duration::from_secs(30), 2.0);
            let mut attempt = 0usize;

            loop {
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => {
                        log::debug!("Push to talk radio recorder task shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(strategy.timeout(attempt)) => {
                        let state = app.state::<AppState>();
                        let state = state.lock().await;

                        let radio = state.radio_handle().read().clone();

                        if let Some(radio) = radio {
                            log::info!("Push to talk radio created; starting recorder");
                            match state.config.client.playback.start(&app, radio).await {
                                Ok(()) => break,
                                // `Unsupported` is a permanent platform limitation (no AFV
                                // loopback capture on this OS), so retrying can never succeed.
                                Err(PlaybackError::Unsupported) => {
                                    log::info!(
                                        "Playback recorder unsupported for this integration on this platform; not retrying"
                                    );
                                    break;
                                }
                                Err(err) => log::warn!("Failed to start recorder: {err}"),
                            }
                        }

                        attempt += 1;
                        log::warn!("Attempting to restart recorder (attempt = {attempt})");
                    }
                }
            }
        });

        radio.app.emit("radio:state", RadioState::RxIdle).ok();

        Ok(radio)
    }
}

#[async_trait::async_trait]
impl Radio for PushToTalkRadio {
    async fn transmit(&self, state: TransmissionState) -> Result<(), RadioError> {
        let (key_state, radio_state) = match state {
            TransmissionState::Active if !self.active.swap(true, Ordering::Relaxed) => {
                (KeyState::Down, RadioState::TxActive)
            }
            TransmissionState::Inactive if self.active.swap(false, Ordering::Relaxed) => {
                (KeyState::Up, RadioState::RxIdle)
            }
            _ => return Ok(()),
        };

        log::trace!(
            "Setting transmission {state:?}, emitting {:?} {key_state:?}",
            self.code,
        );

        self.emitter
            .emit(self.code, key_state)
            .map_err(|err| RadioError::Transmit(err.to_string()))?;

        self.app.emit("radio:state", radio_state).ok();

        Ok(())
    }

    fn state(&self) -> RadioState {
        if self.active.load(Ordering::Relaxed) {
            RadioState::TxActive
        } else {
            RadioState::RxIdle
        }
    }

    fn as_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
}

impl std::fmt::Debug for PushToTalkRadio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PushToTalkRadio")
            .field("code", &self.code)
            .field("active", &self.active)
            .finish()
    }
}

impl Drop for PushToTalkRadio {
    fn drop(&mut self) {
        log::trace!("Dropping PushToTalkRadio: code {:?}", self.code);

        if self.active.load(Ordering::Relaxed)
            && let Err(err) = self.emitter.emit(self.code, KeyState::Up)
        {
            log::warn!("Failed to release PTT key while dropping: {err}");
        }

        self.cancel.cancel();

        self.app.emit("radio:state", RadioState::NotConfigured).ok();
    }
}
