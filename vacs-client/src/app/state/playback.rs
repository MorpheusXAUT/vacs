use crate::app::state::{AppStateInner, sealed};
use crate::playback::recorder::PlaybackRecorderHandle;

pub trait AppStatePlaybackExt: sealed::Sealed {
    fn playback_recorder_handle(&self) -> PlaybackRecorderHandle;
}

impl AppStatePlaybackExt for AppStateInner {
    fn playback_recorder_handle(&self) -> PlaybackRecorderHandle {
        self.playback_recorder.clone()
    }
}
