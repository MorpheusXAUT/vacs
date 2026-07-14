use crate::app::state::AppState;
use crate::app::state::signaling::AppStateSignalingExt;
use crate::app::state::webrtc::AppStateWebrtcExt;
use crate::audio::manager::AudioManagerHandle;
use crate::config::{CallMicMode, KeybindsConfig, TransmitConfig};
use crate::error::Error;
use crate::keybinds::runtime::{DynKeybindListener, KeybindListener, PlatformListener};
use crate::keybinds::{KeyEvent, Keybind};
use crate::radio::{DynRadio, RadioHandle, TransmissionState};
use keyboard_types::{Code, KeyState};
use parking_lot::RwLock;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::RwLock as TokioRwLock;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

#[cfg(target_os = "linux")]
use crate::platform::Platform;

#[derive(Debug)]
pub struct KeybindEngine {
    call_mic_mode: CallMicMode,
    call_code: Option<Code>,
    radio_code: Option<Code>,
    /// Keybinds
    accept_call_code: Option<Code>,
    end_call_code: Option<Code>,
    toggle_radio_prio_code: Option<Code>,
    ///
    app: AppHandle,
    listener: RwLock<Option<DynKeybindListener>>,
    rx_task: Option<JoinHandle<()>>,
    shutdown_token: CancellationToken,
    stop_token: Option<CancellationToken>,
    ///
    call_pressed: Arc<AtomicBool>,
    radio_pressed: Arc<AtomicBool>,
    call_active: Arc<AtomicBool>,
    radio_prio: Arc<AtomicBool>,
    implicit_radio_prio: Arc<AtomicBool>,
}

pub type KeybindEngineHandle = Arc<TokioRwLock<KeybindEngine>>;

impl KeybindEngine {
    pub fn new(
        app: AppHandle,
        transmit_config: &TransmitConfig,
        call_control_config: &KeybindsConfig,
        radio_integration_enabled: bool,
        shutdown_token: CancellationToken,
    ) -> Self {
        Self {
            call_mic_mode: transmit_config.call_mic_mode,
            call_code: Self::select_active_transmit_code(transmit_config),
            radio_code: Self::select_active_radio_code(radio_integration_enabled, transmit_config),
            accept_call_code: Self::select_accept_call_code(call_control_config),
            end_call_code: Self::select_end_call_code(call_control_config),
            toggle_radio_prio_code: Self::select_toggle_radio_prio_code(call_control_config),
            app,
            listener: RwLock::new(None),
            rx_task: None,
            shutdown_token,
            stop_token: None,
            call_pressed: Arc::new(AtomicBool::new(false)),
            radio_pressed: Arc::new(AtomicBool::new(false)),
            call_active: Arc::new(AtomicBool::new(false)),
            radio_prio: Arc::new(AtomicBool::new(false)),
            implicit_radio_prio: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn start(&mut self) -> Result<(), Error> {
        if self.rx_task.is_some() {
            return Ok(());
        }
        let has_call_controls = self.accept_call_code.is_some()
            || self.end_call_code.is_some()
            || self.toggle_radio_prio_code.is_some();

        if self.call_mic_mode == CallMicMode::VoiceActivation
            && self.radio_code.is_none()
            && !has_call_controls
        {
            log::trace!(
                "TransmitMode set to voice activation, no radio PTT set and no call controls defined -> no keybind engine required"
            );
            return Ok(());
        } else if self.call_mic_mode != CallMicMode::VoiceActivation && self.call_code.is_none() {
            log::trace!(
                "No keybind set for TransmitMode {:?}, keybind engine not starting",
                self.call_mic_mode
            );
            return Ok(());
        }

        self.stop_token = Some(self.shutdown_token.child_token());

        let (listener, rx) = PlatformListener::start().await?;
        *self.listener.write() = Some(Arc::new(listener));

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

        // TODO: should be done somewhere else
        /* self.radio.write().take();
        self.app.state::<RadioHandle>().write().take();
        self.app.emit("radio:integration-available", false).ok(); */

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

    pub async fn set_config(
        &mut self,
        transmit_config: &TransmitConfig,
        keybinds_config: &KeybindsConfig,
        radio_integration_enabled: bool,
    ) -> Result<(), Error> {
        self.stop();

        self.call_mic_mode = transmit_config.call_mic_mode;
        self.call_code = Self::select_active_transmit_code(transmit_config);
        self.radio_code =
            Self::select_active_radio_code(radio_integration_enabled, transmit_config);

        self.accept_call_code = Self::select_accept_call_code(keybinds_config);
        self.end_call_code = Self::select_end_call_code(keybinds_config);
        self.toggle_radio_prio_code = Self::select_toggle_radio_prio_code(keybinds_config);

        self.reset_input_state();

        self.start().await?;

        Ok(())
    }

    // TODO: should be done somewhere else
    /* pub async fn set_radio_config(&mut self, config: &RadioConfig) -> Result<(), Error> {
        self.stop();

        self.radio_config = config.clone();

        self.reset_input_state();

        self.start().await?;

        Ok(())
    } */

    /* pub async fn reconnect_radio(&self) -> Result<(), Error> {
        let radio = self.radio.read().clone();
        if let Some(radio) = radio {
            log::info!("Reconnecting radio integration");
            radio
                .reconnect()
                .await
                .map_err(|err| Error::Radio(Box::new(err)))?;
        }
        Ok(())
    } */

    pub fn set_call_active(&self, active: bool) {
        self.call_active.store(active, Ordering::Relaxed);

        if active {
            if self.radio_code.is_some()
                && self.radio_code == self.call_code
                && self.radio_pressed.load(Ordering::Relaxed)
                && !self.radio_prio.load(Ordering::Relaxed)
                && self.call_mic_mode != CallMicMode::VoiceActivation
            {
                log::trace!(
                    "Setting implicit radio prio after entering call while {:?} key is pressed",
                    self.call_mic_mode
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

    pub fn call_active(&self) -> bool {
        self.call_active.load(Ordering::Relaxed)
    }

    pub fn set_radio_prio(&self, prio: bool) {
        let prev_prio = self.radio_prio.swap(prio, Ordering::Relaxed);
        if !prio && prev_prio && self.radio_pressed.load(Ordering::Relaxed) {
            log::trace!(
                "Radio prio unset while {:?} key is pressed, setting implicit radio prio for cleanup",
                self.call_mic_mode
            );
            self.implicit_radio_prio.store(true, Ordering::Relaxed);
        }

        match (
            &self.call_mic_mode,
            self.call_pressed.load(Ordering::Relaxed),
        ) {
            (CallMicMode::VoiceActivation, _) | (CallMicMode::PushToMute, false) => {
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

    pub fn radio_prio(&self) -> bool {
        self.radio_prio.load(Ordering::Relaxed) || self.implicit_radio_prio.load(Ordering::Relaxed)
    }

    pub fn should_attach_input_muted(&self) -> bool {
        let call_pressed = self.call_pressed.load(Ordering::Relaxed);
        let radio_pressed = self.radio_pressed.load(Ordering::Relaxed);
        let radio_prio = self.radio_prio.load(Ordering::Relaxed);
        let separate_keys = self.radio_code.is_some() && self.radio_code != self.call_code;
        match self.call_mic_mode {
            CallMicMode::VoiceActivation => false,
            CallMicMode::PushToTalk => {
                if separate_keys {
                    // PTT-Diff: call PTT alone determines MIC state; prio has no effect (§8.4)
                    !call_pressed
                } else {
                    // PTT-Same/None: prio can force mute even while key held
                    !call_pressed || (radio_pressed && radio_prio)
                }
            }
            CallMicMode::PushToMute => call_pressed,
        }
    }

    // TODO: this needs to be elsewhere
    /* pub fn radio_state(&self) -> RadioState {
        if let Some(radio) = self.radio.read().as_ref() {
            radio.state()
        } else {
            RadioState::NotConfigured
        }
    }

    pub fn radio(&self) -> Option<DynRadio> {
        self.radio.read().clone()
    } */

    /// Get the external (OS-configured) key for a keybind, if available.
    ///
    /// On Wayland, keybinds are configured at the OS level via the XDG Global Shortcuts
    /// portal. This method queries the listener to get the actual key combination the
    /// user configured in their desktop environment.
    ///
    /// Returns `None` on all other platforms where keybinds are configured in-app.
    #[cfg(target_os = "linux")]
    pub fn get_external_binding(&self, keybind: Keybind) -> Option<String> {
        if matches!(Platform::get(), Platform::LinuxWayland) {
            return self
                .listener
                .read()
                .as_ref()
                .and_then(|l| l.get_external_binding(keybind));
        }
        None
    }

    /// Get the external (OS-configured) key for a keybind, if available.
    ///
    /// Returns `None` on all other platforms where keybinds are configured in-app.
    #[cfg(not(target_os = "linux"))]
    pub fn get_external_binding(&self, _keybind: Keybind) -> Option<String> {
        None
    }

    fn reset_input_state(&self) {
        self.call_pressed.store(false, Ordering::Relaxed);
        self.radio_pressed.store(false, Ordering::Relaxed);

        let muted = match &self.call_mic_mode {
            CallMicMode::PushToTalk => true,
            CallMicMode::PushToMute | CallMicMode::VoiceActivation => false,
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

    async fn handle_call_control_event(
        app: &AppHandle,
        code: &Code,
        accept_call: &Option<Code>,
        end_call: &Option<Code>,
        toggle_radio_prio: &Option<Code>,
    ) {
        let shared_call_controls = accept_call == end_call;

        if shared_call_controls
            && (accept_call.is_some_and(|c| c == *code) || end_call.is_some_and(|c| c == *code))
        {
            log::trace!("Shared call control key pressed");

            let state = app.state::<AppState>();
            let mut state = state.lock().await;

            if state.active_call_id().is_some() || state.outgoing_call_id().is_some() {
                match state.end_call(app, None).await {
                    Ok(found) if !found => log::trace!("No active call to end via keybind"),
                    Err(err) => log::warn!("Failed to end active call via keybind: {err}"),
                    _ => {}
                }
            } else {
                match state.accept_call(app, None).await {
                    Ok(found) if !found => log::trace!("No incoming call to accept via keybind"),
                    Err(err) => log::warn!("Failed to accept incoming call via keybind: {err}"),
                    _ => {}
                }
            }
        } else if accept_call.is_some_and(|c| c == *code) {
            log::trace!("Accept call key pressed");

            let state = app.state::<AppState>();
            let mut state = state.lock().await;

            match state.accept_call(app, None).await {
                Ok(found) if !found => log::trace!("No incoming call to accept via keybind"),
                Err(err) => log::warn!("Failed to accept incoming call via keybind: {err}"),
                _ => {}
            }
        } else if end_call.is_some_and(|c| c == *code) {
            log::trace!("End call key pressed");

            let state = app.state::<AppState>();
            let mut state = state.lock().await;

            match state.end_call(app, None).await {
                Ok(found) if !found => log::trace!("No active call to end via keybind"),
                Err(err) => log::warn!("Failed to end active call via keybind: {err}"),
                _ => {}
            }
        } else if toggle_radio_prio.is_some_and(|c| c == *code) {
            log::trace!("Toggle radio prio key pressed");

            let keybind_engine = app.state::<KeybindEngineHandle>();
            let keybind_engine = keybind_engine.read().await;

            if keybind_engine.call_active() {
                let prio = !keybind_engine.radio_prio();
                log::trace!("Toggled radio prio {}", if prio { "on" } else { "off" });
                keybind_engine.set_radio_prio(prio);
                app.emit("audio:radio-prio", prio).ok();
            }
        }
    }

    fn spawn_rx_loop(&mut self, mut rx: UnboundedReceiver<KeyEvent>) {
        let app = self.app.clone();
        let call_code = self.call_code;
        let radio_code = self.radio_code;
        let accept_call = self.accept_call_code;
        let end_call = self.end_call_code;
        let toggle_radio_prio = self.toggle_radio_prio_code;

        if call_code.is_none()
            && accept_call.is_none()
            && end_call.is_none()
            && toggle_radio_prio.is_none()
            && radio_code.is_none()
        {
            return;
        }

        let mode = self.call_mic_mode;
        let stop_token = self
            .stop_token
            .clone()
            .unwrap_or(self.shutdown_token.child_token());
        let radio_handle = self.app.state::<RadioHandle>().clone();
        let call_pressed = self.call_pressed.clone();
        let radio_pressed = self.radio_pressed.clone();
        let call_active = self.call_active.clone();
        let radio_prio_arc = self.radio_prio.clone();
        let implicit_radio_prio = self.implicit_radio_prio.clone();

        let handle = tauri::async_runtime::spawn(async move {
            log::debug!(
                "Keybind engine starting: mode={mode:?}, transmit={call_code:?}, accept_call={accept_call:?}, end_call={end_call:?}",
            );

            loop {
                tokio::select! {
                    biased;
                    _ = stop_token.cancelled() => break,
                    res = rx.recv() => {
                        let Some(event) = res else { break; };

                        if event.state == KeyState::Down {
                            Self::handle_call_control_event(&app, &event.code, &accept_call, &end_call, &toggle_radio_prio).await;
                        }

                        let is_call_key = Some(event.code) == call_code;
                        let is_radio_key = Some(event.code) == radio_code;

                        if !is_call_key && !is_radio_key { continue; }

                        let key_down = event.state == KeyState::Down;

                        if is_call_key && call_pressed.swap(key_down, Ordering::Relaxed) == key_down { continue; }
                        if is_radio_key && radio_pressed.swap(key_down, Ordering::Relaxed) == key_down { continue; }

                        let call_active = call_active.load(Ordering::Relaxed);
                        let radio_prio = radio_prio_arc.load(Ordering::Relaxed);
                        // Implicit prio (set at call entry for radio TX continuity) must not affect
                        // MIC dispatch — only explicit (user-toggled) prio changes MIC behaviour.
                        let effective_prio = radio_prio && !implicit_radio_prio.load(Ordering::Relaxed);

                        let separate = is_call_key ^ is_radio_key;

                        if is_radio_key && (separate || !call_active || radio_prio || mode != CallMicMode::PushToTalk) {
                            Self::set_radio_transmit(&radio_handle, event.state.into()).await;
                            log::debug!("Radio transmit: {key_down}");
                        }

                        if call_active {
                            let mic_action = match (mode, is_call_key, effective_prio) {
                                (CallMicMode::VoiceActivation, ..) => None,

                                // PTT call key: follows key state, or mute-locked when explicit prio is on (§8.3/§8.4/§8.5)
                                (CallMicMode::PushToTalk, true, false) => Some(!key_down),
                                (CallMicMode::PushToTalk, true, true) => Some(true),

                                // PTM: follows key state; explicit prio suppresses MIC changes (§8.6/§8.7)
                                (CallMicMode::PushToMute, _, false) => Some(key_down),

                                // PTT radio key (Diff config): MIC unchanged; radio TX handled above (§8.4)
                                _ => None,
                            };

                            if let Some(muted) = mic_action {
                                Self::set_input_muted(&app, muted);
                            }
                        }

                        if !key_down && is_radio_key {
                            if implicit_radio_prio.swap(false, Ordering::Relaxed) {
                                if radio_prio_arc.swap(false, Ordering::Relaxed) {
                                    app.emit("audio:implicit-radio-prio", false).ok();
                                } else {
                                    // prio was already cleared externally; ensure radio TX stops
                                    Self::set_radio_transmit(&radio_handle, TransmissionState::Inactive).await;
                                    log::debug!("Radio transmit: false (implicit)");
                                }
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
    fn select_active_transmit_code(config: &TransmitConfig) -> Option<Code> {
        #[cfg(target_os = "linux")]
        if matches!(Platform::get(), Platform::LinuxWayland) {
            // Wayland Code Mapping Strategy:
            //
            // On Wayland, shortcuts are configured at the OS level via the XDG Global Shortcuts
            // portal. The portal allows complex key combinations (e.g., Ctrl+Alt+Shift+P) that
            // cannot be represented as a single keyboard_types::Code.
            //
            // To work around this, we map each transmit mode to a unique, unlikely-to-be-pressed
            // function key (F33-F35). These keys don't exist on most keyboards, so there's no
            // conflict with user input. When the portal activates a shortcut, we emit the
            // corresponding F-key code, and the rest of the keybind engine works unchanged.
            //
            // This effectively overrides the user-configured codes in the config file on Wayland,
            // since the actual key binding is managed by the desktop environment.
            let code = match config.mode {
                CallMicMode::VoiceActivation => None,
                CallMicMode::PushToTalk => Some(Code::F33),
                CallMicMode::PushToMute => Some(Code::F34),
                // CallMicMode::RadioIntegration => Some(Code::F35), // TODO
            };
            log::trace!(
                "Using portal shortcut code {code:?} for transmit mode {:?}",
                config.mode
            );
            return code;
        }

        match config.call_mic_mode {
            CallMicMode::VoiceActivation => None,
            CallMicMode::PushToTalk => config.push_to_talk,
            CallMicMode::PushToMute => config.push_to_mute,
        }
    }

    #[inline]
    fn select_active_radio_code(enabled: bool, config: &TransmitConfig) -> Option<Code> {
        if !enabled {
            return None;
        }

        #[cfg(target_os = "linux")]
        if matches!(Platform::get(), Platform::LinuxWayland) {
            // Wayland Code Mapping Strategy:
            //
            // On Wayland, shortcuts are configured at the OS level via the XDG Global Shortcuts
            // portal. The portal allows complex key combinations (e.g., Ctrl+Alt+Shift+P) that
            // cannot be represented as a single keyboard_types::Code.
            //
            // To work around this, we map each transmit mode to a unique, unlikely-to-be-pressed
            // function key (F33-F35). These keys don't exist on most keyboards, so there's no
            // conflict with user input. When the portal activates a shortcut, we emit the
            // corresponding F-key code, and the rest of the keybind engine works unchanged.
            //
            // This effectively overrides the user-configured codes in the config file on Wayland,
            // since the actual key binding is managed by the desktop environment.
            // TODO
            let code = match config.mode {
                CallMicMode::VoiceActivation => None,
                CallMicMode::PushToTalk => Some(Code::F33),
                CallMicMode::PushToMute => Some(Code::F34),
                // CallMicMode::RadioIntegration => Some(Code::F35),
            };
            log::trace!(
                "Using portal shortcut code {code:?} for transmit mode {:?}",
                config.mode
            );
            return code;
        }

        match config.call_mic_mode {
            CallMicMode::VoiceActivation => config.radio_push_to_talk,
            CallMicMode::PushToTalk => config
                .radio_push_to_talk
                .or_else(|| Self::select_active_transmit_code(config)),
            CallMicMode::PushToMute => config.push_to_mute,
        }
    }

    #[inline]
    fn select_accept_call_code(config: &KeybindsConfig) -> Option<Code> {
        #[cfg(target_os = "linux")]
        if matches!(Platform::get(), Platform::LinuxWayland) {
            // Wayland Code Mapping Strategy:
            // Same as with the transmit code, we define our global shortcuts on OS level.
            // As we cannot bind the same key to multiple actions, we'll always use F32
            // as both accept and end call key.
            return Some(Code::F32);
        }

        config.accept_call
    }

    #[inline]
    fn select_end_call_code(config: &KeybindsConfig) -> Option<Code> {
        #[cfg(target_os = "linux")]
        if matches!(Platform::get(), Platform::LinuxWayland) {
            // Wayland Code Mapping Strategy:
            // Same as with the transmit code, we define our global shortcuts on OS level.
            // As we cannot bind the same key to multiple actions, we'll always use F32
            // as both accept and end call key.
            return Some(Code::F32);
        }

        config.end_call
    }

    #[inline]
    fn select_toggle_radio_prio_code(config: &KeybindsConfig) -> Option<Code> {
        #[cfg(target_os = "linux")]
        if matches!(Platform::get(), Platform::LinuxWayland) {
            // Wayland Code Mapping Strategy:
            // Same as with the transmit code, we define our global shortcuts on OS level.
            return Some(Code::F31);
        }

        config.toggle_radio_prio
    }

    #[inline]
    fn set_input_muted(app: &AppHandle, muted: bool) {
        app.state::<AudioManagerHandle>()
            .read()
            .set_input_muted(muted);
    }

    #[inline]
    async fn set_radio_transmit(radio_handle: &RadioHandle, state: TransmissionState) {
        if let Some(radio) = radio_handle.read().as_ref() {
            if let Err(err) = radio.transmit(state).await {
                log::warn!("Failed to set radio transmission state {state:?}: {err}");
            }
        }
    }
}

impl Drop for KeybindEngine {
    fn drop(&mut self) {
        self.stop();
    }
}
