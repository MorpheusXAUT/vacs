use crate::audio::manager::AudioManagerHandle;
use crate::config::{TransmitConfig, TransmitMode};
use crate::error::Error;
use crate::keybinds::KeyEvent;
use crate::keybinds::runtime::{
    DynKeybindEmitter, DynKeybindListener, KeybindEmitter, KeybindListener, PlatformEmitter,
    PlatformListener,
};
use keyboard_types::{Code, KeyState};
use parking_lot::RwLock;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct KeybindEngine {
    mode: TransmitMode,
    code: Option<Code>,
    external_radio_code: Option<Code>,
    app: AppHandle,
    listener: RwLock<Option<DynKeybindListener>>,
    emitter: RwLock<Option<DynKeybindEmitter>>,
    rx_task: Option<JoinHandle<()>>,
    shutdown_token: CancellationToken,
    stop_token: Option<CancellationToken>,
    pressed: Arc<AtomicBool>,
    call_active: Arc<AtomicBool>,
    radio_prio: Arc<AtomicBool>,
    implicit_radio_prio: Arc<AtomicBool>,
}

pub type KeybindEngineHandle = Arc<RwLock<KeybindEngine>>;

impl KeybindEngine {
    pub fn new(app: AppHandle, config: &TransmitConfig, shutdown_token: CancellationToken) -> Self {
        Self {
            mode: config.mode,
            code: Self::select_active_code(config),
            external_radio_code: config.external_radio_push_to_talk,
            app,
            listener: RwLock::new(None),
            emitter: RwLock::new(None),
            rx_task: None,
            shutdown_token,
            stop_token: None,
            pressed: Arc::new(AtomicBool::new(false)),
            call_active: Arc::new(AtomicBool::new(false)),
            radio_prio: Arc::new(AtomicBool::new(false)),
            implicit_radio_prio: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&mut self) -> Result<(), Error> {
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

        let (listener, rx) = PlatformListener::start()?;
        *self.listener.write() = Some(Arc::new(listener));

        if self.external_radio_code.is_some() {
            let emitter = PlatformEmitter::start()?;
            *self.emitter.write() = Some(Arc::new(emitter));
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

        {
            let mut emitter = self.emitter.write();
            if let Some(emitter) = emitter.take()
                && let Some(external_radio_code) = self.external_radio_code
                && let Err(err) = emitter.emit(external_radio_code, KeyState::Up)
            {
                log::warn!(
                    "Failed to send external radio code {external_radio_code} Up while stopping keybind engine: {err}"
                );
            }
        }

        if let Some(stop_token) = self.stop_token.take() {
            stop_token.cancel()
        }

        if let Some(rx_task) = self.rx_task.take() {
            rx_task.abort();
        }
    }

    pub fn shutdown(&mut self) {
        self.shutdown_token.cancel();
        self.stop();
    }

    pub fn set_config(&mut self, config: &TransmitConfig) -> Result<(), Error> {
        self.stop();

        self.code = Self::select_active_code(config);
        self.mode = config.mode;

        self.reset_input_state();

        self.start()?;

        Ok(())
    }

    pub fn set_call_active(&self, active: bool) {
        self.call_active.store(active, Ordering::Relaxed);

        let pressed = self.pressed.load(Ordering::Relaxed);
        let radio_prio = self.radio_prio.load(Ordering::Relaxed);

        if active {
            if matches!(
                self.mode,
                TransmitMode::PushToMute | TransmitMode::PushToTalk
            ) && pressed
                && !radio_prio
            {
                log::trace!(
                    "Setting implicit radio prio after entering call while {:?} key is pressed",
                    self.mode
                );

                self.radio_prio.store(true, Ordering::Relaxed);
                self.implicit_radio_prio.store(true, Ordering::Relaxed);
            }
        } else {
            if self.implicit_radio_prio.swap(false, Ordering::Relaxed) {
                self.radio_prio.store(false, Ordering::Relaxed);
            }

            self.reset_external_radio_code();
        }
    }

    pub fn set_radio_prio(&self, prio: bool) {
        self.radio_prio.store(prio, Ordering::Relaxed);

        if self.mode == TransmitMode::VoiceActivation {
            log::info!(
                "Setting audio input {}",
                if prio { "muted" } else { "unmuted" }
            );
            self.app
                .state::<AudioManagerHandle>()
                .read()
                .set_input_muted(prio);
        }
    }

    pub fn reset_call_state(&self) {
        self.call_active.store(false, Ordering::Relaxed);
        self.radio_prio.store(false, Ordering::Relaxed);
        self.reset_external_radio_code();
    }

    pub fn should_attach_input_muted(&self) -> bool {
        if self.pressed.load(Ordering::Relaxed) {
            return true;
        }
        matches!(self.mode, TransmitMode::PushToTalk)
    }

    fn reset_input_state(&self) {
        self.pressed.store(false, Ordering::Relaxed);

        let muted = match &self.mode {
            TransmitMode::PushToTalk => true,
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
        let emitter = self.emitter.read().clone();
        let external_radio_code = self.external_radio_code;
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

                        let muted_changed = match (&mode, &event.state, pressed.load(Ordering::Relaxed)) {
                            (TransmitMode::PushToTalk, KeyState::Down, false) => {
                                pressed.store(true, Ordering::Relaxed);
                                Some(false)
                            }
                            (TransmitMode::PushToTalk, KeyState::Up, true) => {
                                pressed.store(false, Ordering::Relaxed);
                                Some(true)
                            }
                            (TransmitMode::PushToMute, KeyState::Down, false) => {
                                pressed.store(true, Ordering::Relaxed);
                                Some(true)
                            }
                            (TransmitMode::PushToMute, KeyState::Up, true) => {
                                pressed.store(false, Ordering::Relaxed);
                                Some(false)
                            }
                            _ => None,
                        };

                        let Some(muted) = muted_changed else { continue; };

                        match (call_active.load(Ordering::Relaxed), radio_prio.load(Ordering::Relaxed), external_radio_code) {
                            // No call active, no external radio code defined --> nothing to do
                            (false, _, None) => {
                                continue;
                            }

                            // No call active, external radio code defined --> emit external radio code
                            (false, _, Some(external_radio_code)) => {
                                let Some(emitter) = &emitter else { continue; };
                                log::trace!("No call active, emitting external radio code {external_radio_code:?} {:?}", event.state);
                                Self::emit_external_radio_code(emitter, external_radio_code, event.state);
                            }

                            // Call active, external radio code defined, radio prio active --> emit external radio code
                            (true, true, Some(external_radio_code)) => {
                                let Some(emitter) = &emitter else { continue; };
                                let input_muted = match (&event.state, &mode) {
                                    (KeyState::Down, _) => true,
                                    (KeyState::Up, TransmitMode::PushToTalk) => true,
                                    (KeyState::Up, TransmitMode::PushToMute | TransmitMode::VoiceActivation) => false,
                                };

                                log::trace!(
                                    "Call active, radio prio set, setting audio input {} and emitting external radio code {external_radio_code:?} {:?}",
                                    if input_muted { "muted" } else { "unmuted" },
                                    event.state
                                );
                                Self::set_input_muted(&app, input_muted);
                                Self::emit_external_radio_code(emitter, external_radio_code, event.state);
                            }

                            // Call active, external radio code not defined, radio prio active or
                            // call active, radio prio not active --> change local audio input
                            (true, true, None) | (true, false, _) => {
                                log::trace!("Call active, setting audio input {}", if muted { "muted" } else { "unmuted" });
                                Self::set_input_muted(&app, muted);
                            }
                        }

                        if event.state.is_up() && implicit_radio_prio.swap(false, Ordering::Relaxed) {
                            log::trace!("Implicit radio prio cleared on {:?} key release", mode);
                            radio_prio.store(false, Ordering::Relaxed);
                        }
                    }
                }
            }

            log::trace!("Keybinds engine loop finished");
        });

        self.rx_task = Some(handle);
    }

    #[inline]
    fn reset_external_radio_code(&self) {
        if let Some(external_radio_code) = self.external_radio_code
            && let Some(emitter) = self.emitter.read().deref()
            && let Err(err) = emitter.emit(external_radio_code, KeyState::Up)
        {
            log::warn!(
                "Failed to send external radio code {external_radio_code} Up while resetting: {err}"
            );
        }
    }

    #[inline]
    fn select_active_code(config: &TransmitConfig) -> Option<Code> {
        match config.mode {
            TransmitMode::VoiceActivation => None,
            TransmitMode::PushToTalk => config.push_to_talk,
            TransmitMode::PushToMute => config.push_to_mute,
        }
    }

    #[inline]
    fn set_input_muted(app: &AppHandle, muted: bool) {
        app.state::<AudioManagerHandle>()
            .read()
            .set_input_muted(muted);
    }

    #[inline]
    fn emit_external_radio_code(runtime: &DynKeybindEmitter, code: Code, state: KeyState) {
        if let Err(err) = runtime.emit(code, state) {
            log::warn!(
                "Failed to emit external radio code {code} {:?}: {err}",
                state
            );
        }
    }
}

impl Drop for KeybindEngine {
    fn drop(&mut self) {
        self.stop();
    }
}
