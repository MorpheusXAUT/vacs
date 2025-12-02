use crate::audio::manager::AudioManagerHandle;
use crate::config::{RadioConfig, TransmitConfig, TransmitMode};
use crate::error::Error;
use crate::keybinds::KeyEvent;
use crate::keybinds::runtime::{DynKeybindListener, KeybindListener, PlatformListener};
use crate::platform::Platform;
use crate::radio::{DynRadio, TransmissionState};
use keyboard_types::{Code, KeyState};
use parking_lot::RwLock;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::RwLock as TokioRwLock;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct KeybindEngine {
    mode: TransmitMode,
    code: Option<Code>,
    radio_config: RadioConfig,
    app: AppHandle,
    listener: RwLock<Option<DynKeybindListener>>,
    radio: RwLock<Option<DynRadio>>,
    rx_task: Option<JoinHandle<()>>,
    shutdown_token: CancellationToken,
    stop_token: Option<CancellationToken>,
    pressed: Arc<AtomicBool>,
    call_active: Arc<AtomicBool>,
    radio_prio: Arc<AtomicBool>,
    implicit_radio_prio: Arc<AtomicBool>,
}

pub type KeybindEngineHandle = Arc<TokioRwLock<KeybindEngine>>;

impl KeybindEngine {
    pub fn new(
        app: AppHandle,
        transmit_config: &TransmitConfig,
        radio_config: &RadioConfig,
        shutdown_token: CancellationToken,
    ) -> Self {
        Self {
            mode: transmit_config.mode,
            code: Self::select_active_code(transmit_config),
            radio_config: radio_config.clone(),
            app,
            listener: RwLock::new(None),
            radio: RwLock::new(None),
            rx_task: None,
            shutdown_token,
            stop_token: None,
            pressed: Arc::new(AtomicBool::new(false)),
            call_active: Arc::new(AtomicBool::new(false)),
            radio_prio: Arc::new(AtomicBool::new(false)),
            implicit_radio_prio: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start(&mut self) -> Result<(), Error> {
        if self.rx_task.is_some() {
            debug_assert!(self.listener.read().is_some());
            debug_assert!(self.code.is_some());
            return Ok(());
        }
        if self.mode == TransmitMode::VoiceActivation {
            log::trace!("TransmitMode set to voice activation, no keybind engine required");
            return Ok(());
        } else if self.code.is_none() {
            log::trace!(
                "No keybind set for TransmitMode {:?}, keybind engine not starting",
                self.mode
            );
            return Ok(());
        }

        self.stop_token = Some(self.shutdown_token.child_token());

        let (listener, rx) = PlatformListener::start().await?;
        *self.listener.write() = Some(Arc::new(listener));

        if self.mode == TransmitMode::RadioIntegration {
            let radio = self.radio_config.radio()?;
            self.app
                .emit("radio:integration-available", radio.is_some())
                .ok();
            *self.radio.write() = radio;
        } else {
            self.app.emit("radio:integration-available", false).ok();
        }

        self.spawn_rx_loop(rx);

        Ok(())
    }

    pub fn stop(&mut self) {
        {
            let mut listener = self.listener.write();
            if listener.take().is_some() {
                self.reset_input_state();
            }
        }

        self.radio.write().take();
        self.app.emit("radio:integration-available", false).ok();

        if let Some(stop_token) = self.stop_token.take() {
            stop_token.cancel();
        }

        if let Some(rx_task) = self.rx_task.take() {
            rx_task.abort();
        }
    }

    pub fn shutdown(&mut self) {
        self.shutdown_token.cancel();
        self.stop();
    }

    pub async fn set_config(&mut self, config: &TransmitConfig) -> Result<(), Error> {
        self.stop();

        self.code = Self::select_active_code(config);
        self.mode = config.mode;

        self.reset_input_state();

        self.start().await?;

        Ok(())
    }

    pub async fn set_radio_config(&mut self, config: &RadioConfig) -> Result<(), Error> {
        self.stop();

        self.radio_config = config.clone();

        self.reset_input_state();

        self.start().await?;

        Ok(())
    }

    pub fn set_call_active(&self, active: bool) {
        self.call_active.store(active, Ordering::Relaxed);

        if active {
            if matches!(self.mode, TransmitMode::RadioIntegration)
                && self.pressed.load(Ordering::Relaxed)
                && !self.radio_prio.load(Ordering::Relaxed)
            {
                log::trace!(
                    "Setting implicit radio prio after entering call while {:?} key is pressed",
                    self.mode
                );

                self.radio_prio.store(true, Ordering::Relaxed);
                self.implicit_radio_prio.store(true, Ordering::Relaxed);
                self.app.emit("audio:implicit-radio-prio", true).ok();
            }
        } else {
            self.implicit_radio_prio.store(false, Ordering::Relaxed);
            self.radio_prio.store(false, Ordering::Relaxed);
            self.app.emit("audio:implicit-radio-prio", false).ok();
        }
    }

    pub fn set_radio_prio(&self, prio: bool) {
        let prev_prio = self.radio_prio.swap(prio, Ordering::Relaxed);
        if !prio && prev_prio && self.pressed.load(Ordering::Relaxed) {
            log::trace!(
                "Radio prio unset while {:?} key is pressed, setting implicit radio prio for cleanup",
                self.mode
            );
            self.implicit_radio_prio.store(true, Ordering::Relaxed);
        }

        match (&self.mode, self.pressed.load(Ordering::Relaxed)) {
            (TransmitMode::VoiceActivation, _) | (TransmitMode::PushToMute, false) => {
                log::info!(
                    "Setting audio input {}",
                    if prio { "muted" } else { "unmuted" }
                );
                self.app
                    .state::<AudioManagerHandle>()
                    .read()
                    .set_input_muted(prio);
            }
            _ => {}
        }
    }

    pub fn should_attach_input_muted(&self) -> bool {
        match (&self.mode, self.pressed.load(Ordering::Relaxed)) {
            (TransmitMode::PushToTalk, false) => true,
            (TransmitMode::PushToMute, true) => true,
            (TransmitMode::RadioIntegration, false) => true,
            (TransmitMode::RadioIntegration, true) => self.radio_prio.load(Ordering::Relaxed),
            _ => false,
        }
    }

    pub fn has_radio(&self) -> bool {
        self.radio.read().is_some()
    }

    pub fn get_external_binding(&self, mode: TransmitMode) -> Option<String> {
        #[cfg(target_os = "linux")]
        if matches!(Platform::detect(), Platform::LinuxWayland) {
            return self
                .listener
                .read()
                .as_ref()
                .and_then(|l| l.get_external_binding(mode));
        }

        None
    }

    fn reset_input_state(&self) {
        self.pressed.store(false, Ordering::Relaxed);

        let muted = match &self.mode {
            TransmitMode::PushToTalk | TransmitMode::RadioIntegration => true,
            TransmitMode::PushToMute | TransmitMode::VoiceActivation => false,
        };

        log::trace!(
            "Resetting audio input {}",
            if muted { "muted" } else { "unmuted" }
        );

        self.app
            .state::<AudioManagerHandle>()
            .read()
            .set_input_muted(muted);
    }

    fn spawn_rx_loop(&mut self, mut rx: UnboundedReceiver<KeyEvent>) {
        let app = self.app.clone();
        let Some(active) = self.code else {
            return;
        };
        let mode = self.mode;
        let stop_token = self
            .stop_token
            .clone()
            .unwrap_or(self.shutdown_token.child_token());
        let radio = self.radio.read().clone();
        let pressed = self.pressed.clone();
        let call_active = self.call_active.clone();
        let radio_prio = self.radio_prio.clone();
        let implicit_radio_prio = self.implicit_radio_prio.clone();

        let handle = tauri::async_runtime::spawn(async move {
            log::debug!(
                "Keybind engine starting: mode={:?}, code={:?}",
                mode,
                active
            );

            loop {
                tokio::select! {
                    biased;
                    _ = stop_token.cancelled() => break,
                    res = rx.recv() => {
                        let Some(event) = res else { break; };
                        if event.code != active { continue; }

                        let muted = match (&mode, &event.state) {
                            (TransmitMode::PushToTalk | TransmitMode::RadioIntegration, KeyState::Down) if !pressed.swap(true, Ordering::Relaxed) => false,
                            (TransmitMode::PushToTalk | TransmitMode::RadioIntegration, KeyState::Up) if pressed.swap(false, Ordering::Relaxed) => true,
                            (TransmitMode::PushToMute, KeyState::Down) if !pressed.swap(true, Ordering::Relaxed) => true,
                            (TransmitMode::PushToMute, KeyState::Up) if pressed.swap(false, Ordering::Relaxed) => false,
                            _ => continue,
                        };

                        match (&mode, call_active.load(Ordering::Relaxed), radio_prio.load(Ordering::Relaxed)) {
                            (TransmitMode::RadioIntegration, false, _) => {
                                let state = event.state.into();
                                if let Some(radio) = radio.as_ref() {
                                    log::trace!("No call active, setting radio transmission {state:?}");
                                    Self::set_radio_transmit(&app, radio, state);
                                } else {
                                    log::trace!("No call active, radio not initialized, cannot set transmission {state:?}");
                                }
                            },
                            (TransmitMode::RadioIntegration, true, false) => {
                                log::trace!("Call active, no radio prio, setting audio input {}", if muted { "muted" } else { "unmuted" });
                                Self::set_input_muted(&app, muted);
                            },
                            (TransmitMode::RadioIntegration, true, true) => {
                                let state = event.state.into();
                                if let Some(radio) = radio.as_ref() {
                                    log::trace!("Call active, radio prio set, setting audio input muted and radio transmission {state:?}");
                                    Self::set_input_muted(&app, true);
                                    Self::set_radio_transmit(&app, radio, state);
                                } else {
                                    log::trace!("Call active, radio prio set, radio not initialized, setting audio input muted, but cannot set transmission {state:?}");
                                    Self::set_input_muted(&app, true);
                                }
                            }
                            (TransmitMode::PushToTalk | TransmitMode::PushToMute, true, false) => {
                                log::trace!("Call active, setting audio input {}", if muted { "muted" } else { "unmuted" });
                                Self::set_input_muted(&app, muted);
                            },
                            (TransmitMode::PushToTalk, true, true) => {
                                log::trace!("Call active, would set audio input {}, but radio prio is set, so keeping audio input muted", if muted { "muted" } else { "unmuted" });
                                Self::set_input_muted(&app, true);
                            }
                            _ => {}

                        }

                        if event.state.is_up() && implicit_radio_prio.swap(false, Ordering::Relaxed) {
                            if radio_prio.swap(false, Ordering::Relaxed) {
                                log::trace!("Implicit radio prio cleared on {:?} key release", mode);
                                app.emit("audio:implicit-radio-prio", false).ok();
                            } else if let Some(radio) = radio.as_ref() {
                                log::trace!("Implicit radio prio cleared on {mode:?} key release, but radio prio was not set. Setting transmission Inactive");
                                Self::set_radio_transmit(&app, radio, TransmissionState::Inactive);
                            } else {
                                log::trace!("Implicit radio prio cleared on {mode:?} key release, but radio not initialized, ignoring");
                            }
                        }
                    }
                }
            }

            log::trace!("Keybinds engine loop finished");
        });

        self.rx_task = Some(handle);
    }

    #[inline]
    fn select_active_code(config: &TransmitConfig) -> Option<Code> {
        #[cfg(target_os = "linux")]
        if matches!(Platform::detect(), Platform::LinuxWayland) {
            // As shortcut binding is performed on OS level and can contain keys including multiple
            // modifiers, so we can't reliably map them to a single Code. Instead, we emit a distinct
            // code for each transmit mode that's highly unlikely to be used otherwise (as most
            // keyboards don't even map the respective keys. This effectively disables/overrides the
            // code defined in the vacs-config, even though webview-based capture might still work.
            let code = match config.mode {
                TransmitMode::VoiceActivation => None,
                TransmitMode::PushToTalk => Some(Code::F33),
                TransmitMode::PushToMute => Some(Code::F34),
                TransmitMode::RadioIntegration => Some(Code::F35),
            };
            log::trace!(
                "Using portal shortcut code {code:?} for transmit mode {:?}",
                config.mode
            );
            return code;
        }

        match config.mode {
            TransmitMode::VoiceActivation => None,
            TransmitMode::PushToTalk => config.push_to_talk,
            TransmitMode::PushToMute => config.push_to_mute,
            TransmitMode::RadioIntegration => config.radio_push_to_talk,
        }
    }

    #[inline]
    fn set_input_muted(app: &AppHandle, muted: bool) {
        app.state::<AudioManagerHandle>()
            .read()
            .set_input_muted(muted);
    }

    #[inline]
    fn set_radio_transmit(app: &AppHandle, radio: &DynRadio, state: TransmissionState) {
        if let Err(err) = radio.transmit(state) {
            log::warn!("Failed to set radio transmission state {state:?}: {err}");
        } else {
            app.emit("radio:transmission-state", state).ok();
        }
    }
}

impl Drop for KeybindEngine {
    fn drop(&mut self) {
        self.stop();
    }
}
