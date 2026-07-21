use crate::app::state::AppState;
use crate::app::state::signaling::AppStateSignalingExt;
use crate::app::state::webrtc::AppStateWebrtcExt;
use crate::audio::manager::AudioManagerHandle;
use crate::error::Error;
use crate::keybinds::joystick::JoystickServiceHandle;
use crate::keybinds::runtime::{DynKeybindListener, KeybindListener, PlatformListener};
use crate::keybinds::{
    CallMicMode, InputCode, KeyEvent, Keybind, KeybindsConfig, TransmitConfig, Trigger,
};
use crate::radio::{RadioHandle, TransmissionState};
use keyboard_types::KeyState;
use parking_lot::RwLock;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::RwLock as TokioRwLock;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tokio_util::sync::CancellationToken;

#[cfg(target_os = "linux")]
use crate::keybinds::{PortalAction, compose_wayland_triggers};
#[cfg(target_os = "linux")]
use crate::platform::Platform;

#[derive(Debug)]
pub struct KeybindEngine {
    call_mic_mode: CallMicMode,
    call_triggers: Vec<Trigger>,
    radio_triggers: Vec<Trigger>,
    accept_call_triggers: Vec<Trigger>,
    end_call_triggers: Vec<Trigger>,
    toggle_radio_prio_triggers: Vec<Trigger>,
    app: AppHandle,
    listener: RwLock<Option<DynKeybindListener>>,
    rx_task: Option<JoinHandle<()>>,
    shutdown_token: CancellationToken,
    stop_token: Option<CancellationToken>,
    call_pressed: Arc<AtomicBool>,
    radio_pressed: Arc<AtomicBool>,
    call_active: Arc<AtomicBool>,
    radio_prio: Arc<AtomicBool>,
    implicit_radio_prio: Arc<AtomicBool>,
    radio_transmitting: Arc<AtomicBool>,
}

pub type KeybindEngineHandle = Arc<TokioRwLock<KeybindEngine>>;

impl KeybindEngine {
    pub async fn new(
        app: AppHandle,
        transmit_config: &TransmitConfig,
        call_control_config: &KeybindsConfig,
        radio_integration_enabled: bool,
        shutdown_token: CancellationToken,
    ) -> Self {
        Self {
            call_mic_mode: transmit_config.call_mic_mode,
            call_triggers: transmit_config.active_call_triggers(),
            radio_triggers: transmit_config
                .active_radio_triggers(radio_integration_enabled)
                .await,
            accept_call_triggers: Self::select_accept_call_triggers(call_control_config),
            end_call_triggers: Self::select_end_call_triggers(call_control_config),
            toggle_radio_prio_triggers: Self::select_toggle_radio_prio_triggers(
                call_control_config,
            ),
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
            radio_transmitting: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Whether any active trigger is a joystick button (requiring the shared
    /// joystick service to be running).
    fn any_button_trigger(&self) -> bool {
        [
            &self.call_triggers,
            &self.radio_triggers,
            &self.accept_call_triggers,
            &self.end_call_triggers,
            &self.toggle_radio_prio_triggers,
        ]
        .into_iter()
        .flatten()
        .any(|trigger| matches!(trigger, Trigger::Input(InputCode::Button(_))))
    }

    pub async fn start(&mut self) -> Result<(), Error> {
        if self.rx_task.is_some() {
            return Ok(());
        }
        let has_call_controls = !self.accept_call_triggers.is_empty()
            || !self.end_call_triggers.is_empty()
            || !self.toggle_radio_prio_triggers.is_empty();

        if self.call_mic_mode == CallMicMode::VoiceActivation
            && self.radio_triggers.is_empty()
            && !has_call_controls
        {
            log::trace!(
                "TransmitMode set to voice activation, no radio PTT set and no call controls defined -> no keybind engine required"
            );
            return Ok(());
        } else if self.call_mic_mode != CallMicMode::VoiceActivation
            && self.call_triggers.is_empty()
            && self.radio_triggers.is_empty()
        {
            log::trace!(
                "No keybind set for TransmitMode {:?}, keybind engine not starting",
                self.call_mic_mode
            );
            return Ok(());
        }

        self.stop_token = Some(self.shutdown_token.child_token());

        let any_button = self.any_button_trigger();

        // All input sources write into this one channel; the engine loop below
        // reads the merged stream directly.
        let (key_event_tx, key_event_rx) = unbounded_channel();

        // A keyboard listener failure (e.g. portal unavailable on Wayland) must
        // not disable joystick bindings, and vice versa: start whichever sources
        // are available and only fail if none are.
        let keyboard_ok = match PlatformListener::start(key_event_tx.clone()).await {
            Ok(listener) => {
                *self.listener.write() = Some(Arc::new(listener));
                true
            }
            Err(err) if any_button => {
                log::error!(
                    "Keybind listener failed to start, continuing with joystick bindings only: {err}"
                );
                false
            }
            Err(err) => return Err(err.into()),
        };

        if any_button
            && let Err(err) = self
                .app
                .state::<JoystickServiceHandle>()
                .register(key_event_tx)
                .await
        {
            if !keyboard_ok {
                return Err(err.into());
            }
            log::error!(
                "Joystick service failed to start, continuing with keyboard bindings only: {err}"
            );
        }

        self.spawn_rx_loop(key_event_rx);

        Ok(())
    }

    pub fn stop(&mut self) {
        // The engine may run without a platform listener (joystick-only mode
        // when the keyboard listener failed to start), so the running state is
        // tracked by the rx task, not the listener.
        let was_running = self.rx_task.is_some();

        self.listener.write().take();

        if let Some(stop_token) = self.stop_token.take() {
            stop_token.cancel();
        }

        if let Some(rx_task) = self.rx_task.take() {
            rx_task.abort();
        }

        if was_running {
            self.reset_input_state();
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
        self.call_triggers = transmit_config.active_call_triggers();
        self.radio_triggers = transmit_config
            .active_radio_triggers(radio_integration_enabled)
            .await;

        self.accept_call_triggers = Self::select_accept_call_triggers(keybinds_config);
        self.end_call_triggers = Self::select_end_call_triggers(keybinds_config);
        self.toggle_radio_prio_triggers = Self::select_toggle_radio_prio_triggers(keybinds_config);

        self.reset_input_state();

        self.start().await?;

        Ok(())
    }

    pub fn set_call_active(&self, active: bool) {
        self.call_active.store(active, Ordering::Relaxed);

        if active {
            if !self.radio_triggers.is_empty()
                && same_triggers(&self.radio_triggers, &self.call_triggers)
                && self.radio_pressed.load(Ordering::Relaxed)
                && self.radio_transmitting.load(Ordering::Relaxed)
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
        let separate_keys = !self.radio_triggers.is_empty()
            && !same_triggers(&self.radio_triggers, &self.call_triggers);
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
        self.radio_transmitting.store(false, Ordering::Relaxed);

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
        trigger: &Trigger,
        accept_call: &[Trigger],
        end_call: &[Trigger],
        toggle_radio_prio: &[Trigger],
    ) {
        let is_accept = accept_call.contains(trigger);
        let is_end = end_call.contains(trigger);

        // A trigger bound to both accept and end (the same key configured for
        // both, or the shared Wayland portal call-control shortcut) toggles:
        // end the active/outgoing call if there is one, otherwise accept.
        if is_accept && is_end {
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
        } else if is_accept {
            log::trace!("Accept call key pressed");

            let state = app.state::<AppState>();
            let mut state = state.lock().await;

            match state.accept_call(app, None).await {
                Ok(found) if !found => log::trace!("No incoming call to accept via keybind"),
                Err(err) => log::warn!("Failed to accept incoming call via keybind: {err}"),
                _ => {}
            }
        } else if is_end {
            log::trace!("End call key pressed");

            let state = app.state::<AppState>();
            let mut state = state.lock().await;

            match state.end_call(app, None).await {
                Ok(found) if !found => log::trace!("No active call to end via keybind"),
                Err(err) => log::warn!("Failed to end active call via keybind: {err}"),
                _ => {}
            }
        } else if toggle_radio_prio.contains(trigger) {
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
        let call_triggers = self.call_triggers.clone();
        let radio_triggers = self.radio_triggers.clone();
        let accept_call = self.accept_call_triggers.clone();
        let end_call = self.end_call_triggers.clone();
        let toggle_radio_prio = self.toggle_radio_prio_triggers.clone();

        if call_triggers.is_empty()
            && accept_call.is_empty()
            && end_call.is_empty()
            && toggle_radio_prio.is_empty()
            && radio_triggers.is_empty()
        {
            return;
        }

        let mode = self.call_mic_mode;
        let stop_token = self
            .stop_token
            .clone()
            .unwrap_or(self.shutdown_token.child_token());
        let radio_handle = self.app.state::<RadioHandle>().inner().clone();
        let call_pressed = self.call_pressed.clone();
        let radio_pressed = self.radio_pressed.clone();
        let call_active = self.call_active.clone();
        let radio_prio_arc = self.radio_prio.clone();
        let implicit_radio_prio = self.implicit_radio_prio.clone();
        let radio_transmitting = self.radio_transmitting.clone();

        let handle = tauri::async_runtime::spawn(async move {
            log::debug!(
                "Keybind engine starting: mode={mode:?}, transmit={call_triggers:?}, radio={radio_triggers:?}, accept_call={accept_call:?}, end_call={end_call:?}",
            );

            loop {
                tokio::select! {
                    biased;
                    _ = stop_token.cancelled() => break,
                    res = rx.recv() => {
                        let Some(event) = res else { break; };

                        if event.state == KeyState::Down {
                            Self::handle_call_control_event(&app, &event.trigger, &accept_call, &end_call, &toggle_radio_prio).await;
                        }

                        let is_call_key = call_triggers.contains(&event.trigger);
                        let is_radio_key = radio_triggers.contains(&event.trigger);

                        if !is_call_key && !is_radio_key { continue; }

                        let key_down = event.state == KeyState::Down;

                        if is_call_key && call_pressed.swap(key_down, Ordering::Relaxed) == key_down { continue; }
                        if is_radio_key && radio_pressed.swap(key_down, Ordering::Relaxed) == key_down { continue; }

                        let call_active = call_active.load(Ordering::Relaxed);
                        let radio_prio = radio_prio_arc.load(Ordering::Relaxed);
                        // Implicit prio (set at call entry for radio TX continuity) must not affect
                        // MIC dispatch - only explicit (user-toggled) prio changes MIC behaviour.
                        let effective_prio = radio_prio && !implicit_radio_prio.load(Ordering::Relaxed);

                        let separate = is_call_key ^ is_radio_key;

                        if is_radio_key && (separate || !call_active || radio_prio || mode != CallMicMode::PushToTalk) {
                            radio_transmitting.store(key_down, Ordering::Relaxed);
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

                        if !key_down && is_radio_key && implicit_radio_prio.swap(false, Ordering::Relaxed) {
                            if radio_prio_arc.swap(false, Ordering::Relaxed) {
                                app.emit("audio:implicit-radio-prio", false).ok();
                            } else {
                                radio_transmitting.store(false, Ordering::Relaxed);
                                // prio was already cleared externally; ensure radio TX stops
                                Self::set_radio_transmit(&radio_handle, TransmissionState::Inactive).await;
                                log::debug!("Radio transmit: false (implicit)");
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
    fn select_accept_call_triggers(config: &KeybindsConfig) -> Vec<Trigger> {
        #[cfg(target_os = "linux")]
        if matches!(Platform::get(), Platform::LinuxWayland) {
            // The portal exposes a single shared call-control shortcut (end
            // active / accept next), so both accept and end carry the same
            // portal action; a configured joystick button replaces it.
            return compose_wayland_triggers(Some(PortalAction::CallControl), &config.accept_call);
        }

        config
            .accept_call
            .clone()
            .map(Trigger::Input)
            .into_iter()
            .collect()
    }

    #[inline]
    fn select_end_call_triggers(config: &KeybindsConfig) -> Vec<Trigger> {
        #[cfg(target_os = "linux")]
        if matches!(Platform::get(), Platform::LinuxWayland) {
            // See select_accept_call_triggers: shared portal call-control shortcut.
            return compose_wayland_triggers(Some(PortalAction::CallControl), &config.end_call);
        }

        config
            .end_call
            .clone()
            .map(Trigger::Input)
            .into_iter()
            .collect()
    }

    #[inline]
    fn select_toggle_radio_prio_triggers(config: &KeybindsConfig) -> Vec<Trigger> {
        #[cfg(target_os = "linux")]
        if matches!(Platform::get(), Platform::LinuxWayland) {
            return compose_wayland_triggers(
                Some(PortalAction::ToggleRadioPrio),
                &config.toggle_radio_prio,
            );
        }

        config
            .toggle_radio_prio
            .clone()
            .map(Trigger::Input)
            .into_iter()
            .collect()
    }

    #[inline]
    fn set_input_muted(app: &AppHandle, muted: bool) {
        app.state::<AudioManagerHandle>()
            .read()
            .set_input_muted(muted);
    }

    #[inline]
    async fn set_radio_transmit(radio_handle: &RadioHandle, state: TransmissionState) {
        let radio = radio_handle.read().clone();
        if let Some(radio) = radio
            && let Err(err) = radio.transmit(state).await
        {
            log::warn!("Failed to set radio transmission state {state:?}: {err}");
        }
    }
}

/// Whether two trigger lists bind the same set of inputs, regardless of order.
///
/// Trigger lists are tiny (at most two entries) and duplicate-free by
/// construction, so a containment check both suffices and avoids needing an
/// `Ord` impl consistent with `JoystickButton`'s name-ignoring equality.
fn same_triggers(a: &[Trigger], b: &[Trigger]) -> bool {
    a.len() == b.len() && a.iter().all(|trigger| b.contains(trigger))
}

impl Drop for KeybindEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybinds::{JoystickButton, PortalAction};
    use keyboard_types::Code;

    fn key(code: Code) -> Trigger {
        Trigger::Input(InputCode::Key(code))
    }

    fn button(device: &str, index: u32) -> Trigger {
        Trigger::Input(InputCode::Button(JoystickButton {
            device: device.to_string(),
            button: index,
            name: None,
        }))
    }

    #[test]
    fn same_triggers_ignores_order() {
        let portal = Trigger::Portal(PortalAction::PushToTalk);
        let a = vec![portal.clone(), button("guid", 2)];
        let b = vec![button("guid", 2), portal];

        assert!(same_triggers(&a, &b));
        assert!(same_triggers(&[], &[]));
        assert!(same_triggers(&[key(Code::F13)], &[key(Code::F13)]));
    }

    #[test]
    fn same_triggers_rejects_differing_sets() {
        // subset
        assert!(!same_triggers(
            &[key(Code::F13)],
            &[key(Code::F13), button("guid", 2)]
        ));
        // disjoint
        assert!(!same_triggers(&[key(Code::F13)], &[key(Code::F14)]));
        // same button index on different devices
        assert!(!same_triggers(
            &[button("yoke", 2)],
            &[button("throttle", 2)]
        ));
    }
}
