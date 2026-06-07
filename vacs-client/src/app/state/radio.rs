use crate::app::state::{AppStateInner, sealed};
use crate::radio::RadioHandle;

pub trait AppStateRadioExt: sealed::Sealed {
    fn radio_handle(&self) -> RadioHandle;
}

impl AppStateRadioExt for AppStateInner {
    fn radio_handle(&self) -> RadioHandle {
        self.radio.clone()
    }
}
