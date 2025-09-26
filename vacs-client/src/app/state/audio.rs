use crate::app::state::{AppStateInner, sealed};
use crate::audio::manager::AudioManager;

pub trait AppStateAudioExt: sealed::Sealed {
    fn audio_manager(&self) -> &AudioManager;
    fn audio_manager_mut(&mut self) -> &mut AudioManager;
}

impl AppStateAudioExt for AppStateInner {
    fn audio_manager(&self) -> &AudioManager {
        &self.audio_manager
    }

    fn audio_manager_mut(&mut self) -> &mut AudioManager {
        &mut self.audio_manager
    }
}
