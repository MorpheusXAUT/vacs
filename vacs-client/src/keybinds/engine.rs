use crate::audio::manager::AudioManagerHandle;
use crate::config::{TransmitConfig, TransmitMode};
use crate::error::Error;
use crate::keybinds::runtime::{KeybindRuntime, PlatformKeybindRuntime};
use crate::keybinds::KeyEvent;
use keyboard_types::{Code, KeyState};
use parking_lot::Mutex;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct KeybindEngine {
    mode: TransmitMode,
    code: Option<Code>,
    app: AppHandle,
    runtime: Option<PlatformKeybindRuntime>,
    rx_task: Option<JoinHandle<()>>,
    shutdown_token: CancellationToken,
    stop_token: Option<CancellationToken>,
}

pub type KeybindEngineHandle = Mutex<KeybindEngine>;

impl KeybindEngine {
    pub fn new(app: AppHandle, config: &TransmitConfig, shutdown_token: CancellationToken) -> Self {
        Self {
            mode: config.mode,
            code: Self::select_active_code(config),
            app,
            runtime: None,
            rx_task: None,
            shutdown_token,
            stop_token: None,
        }
    }

    pub fn start(&mut self) -> Result<(), Error> {
        if self.rx_task.is_some() {
            debug_assert!(self.runtime.is_some());
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

        let (runtime, rx) = PlatformKeybindRuntime::start()?;
        self.runtime = Some(runtime);

        self.spawn_rx_loop(rx);

        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(mut runtime) = self.runtime.take() {
            runtime.stop();
            self.reset_input_state();
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

    fn reset_input_state(&self) {
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

        let handle = tauri::async_runtime::spawn(async move {
            log::debug!(
                "Keybind engine starting: mode={:?}, code={:?}",
                mode,
                active
            );
            let mut pressed = false;

            loop {
                tokio::select! {
                    biased;
                    _ = stop_token.cancelled() => break,
                    res = rx.recv() => {
                        match res {
                            Some(event) => {
                                if event.code != active {
                                    continue;
                                }

                                let muted_changed = match (&mode, &event.state) {
                                    (TransmitMode::PushToTalk, KeyState::Down) if !pressed => {
                                        pressed = true;
                                        Some(false)
                                    }
                                    (TransmitMode::PushToTalk, KeyState::Up) if pressed => {
                                        pressed = false;
                                        Some(true)
                                    }
                                    (TransmitMode::PushToMute, KeyState::Down) if !pressed => {
                                        pressed = true;
                                        Some(true)
                                    }
                                    (TransmitMode::PushToMute, KeyState::Up) if pressed => {
                                        pressed = false;
                                        Some(false)
                                    }
                                    _ => None,
                                };

                                if let Some(muted) = muted_changed {
                                    log::trace!("Setting audio input {}", if muted {"muted"} else {"unmuted"});
                                    app.state::<AudioManagerHandle>().read().set_input_muted(muted);
                                }
                            }
                            None => {
                                break;
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
        match config.mode {
            TransmitMode::VoiceActivation => None,
            TransmitMode::PushToTalk => config.push_to_talk,
            TransmitMode::PushToMute => config.push_to_mute,
        }
    }
}

impl Drop for KeybindEngine {
    fn drop(&mut self) {
        self.stop();
    }
}
