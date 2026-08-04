//! X11 keybind emitter using the XTest extension.
//!
//! `XTestFakeInput` injects key events at the X server level, so they are
//! delivered to whatever client would receive a real keypress — including
//! globally-listening radio clients (XRecord/XInput2 based), which is what the
//! AudioForVatsim radio integration relies on. Only X clients can receive the
//! events; Wayland-native applications running alongside Xwayland cannot.

use crate::keybinds::KeybindsError;
use crate::keybinds::runtime::KeybindEmitter;
use crate::keybinds::runtime::linux::x11::code_to_x_keycode;
use keyboard_types::{Code, KeyState};
use std::fmt::{Debug, Formatter};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{KEY_PRESS_EVENT, KEY_RELEASE_EVENT};
use x11rb::protocol::xtest::ConnectionExt as _;
use x11rb::rust_connection::RustConnection;

pub struct X11KeybindEmitter {
    conn: RustConnection,
}

impl Debug for X11KeybindEmitter {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("X11KeybindEmitter")
    }
}

impl KeybindEmitter for X11KeybindEmitter {
    fn start() -> Result<Self, KeybindsError>
    where
        Self: Sized,
    {
        log::debug!("Starting X11 keybind emitter");

        let (conn, _screen_num) = x11rb::connect(None).map_err(|err| {
            KeybindsError::Emitter(format!("Failed to connect to X server: {err}"))
        })?;

        conn.xtest_get_version(2, 2)
            .map_err(|err| KeybindsError::Emitter(format!("Failed to query XTest version: {err}")))?
            .reply()
            .map_err(|err| {
                KeybindsError::Emitter(format!("XTest is not supported by the X server: {err}"))
            })?;

        Ok(Self { conn })
    }

    fn emit(&self, code: Code, state: KeyState) -> Result<(), KeybindsError> {
        let keycode = code_to_x_keycode(code)?;
        let event_type = match state {
            KeyState::Down => KEY_PRESS_EVENT,
            KeyState::Up => KEY_RELEASE_EVENT,
        };

        self.conn
            .xtest_fake_input(
                event_type,
                keycode,
                x11rb::CURRENT_TIME,
                x11rb::NONE,
                0,
                0,
                0,
            )
            .map_err(|err| KeybindsError::Emitter(format!("Failed to emit key event: {err}")))?;

        self.conn.flush().map_err(|err| {
            KeybindsError::Emitter(format!("Failed to flush X connection: {err}"))
        })?;

        Ok(())
    }
}
