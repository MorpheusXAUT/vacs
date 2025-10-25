use crate::keybinds::runtime::{DynKeybindEmitter, KeybindEmitter, PlatformEmitter};
use crate::radio::{Radio, RadioError, TransmissionState};
use keyboard_types::{Code, KeyState};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone)]
pub struct PushToTalkRadio {
    code: Code,
    emitter: DynKeybindEmitter,
    active: Arc<AtomicBool>,
}

impl PushToTalkRadio {
    pub fn new(code: Code) -> Result<Self, RadioError> {
        log::trace!("PushToTalkRadio starting: code {:?}", code);
        Ok(Self {
            code,
            emitter: Arc::new(
                PlatformEmitter::start().map_err(|err| RadioError::Integration(err.to_string()))?,
            ),
            active: Arc::new(AtomicBool::new(false)),
        })
    }
}

impl Radio for PushToTalkRadio {
    fn transmit(&self, state: TransmissionState) -> Result<(), RadioError> {
        let key_state = match state {
            TransmissionState::Active if !self.active.swap(true, Ordering::Relaxed) => {
                KeyState::Down
            }
            TransmissionState::Inactive if self.active.swap(false, Ordering::Relaxed) => {
                KeyState::Up
            }
            _ => return Ok(()),
        };

        log::trace!(
            "Setting transmission {state:?}, emitting {:?} {key_state:?}",
            self.code,
        );
        self.emitter
            .emit(self.code, key_state)
            .map_err(|err| RadioError::Transmit(err.to_string()))
    }
}

impl Drop for PushToTalkRadio {
    fn drop(&mut self) {
        log::trace!("Dropping PushToTalkRadio: code {:?}", self.code);
        if let Err(err) = self.transmit(TransmissionState::Inactive) {
            log::warn!("Failed to set transmission Inactive while dropping: {err}");
        }
    }
}
