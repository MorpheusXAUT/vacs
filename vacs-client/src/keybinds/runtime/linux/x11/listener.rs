//! X11 keybind listener using XInput2 raw events.
//!
//! # Why XInput2 Raw Events?
//!
//! Selecting `RawKeyPress`/`RawKeyRelease` on the root window delivers every key
//! transition regardless of which window has focus and **without grabbing**:
//! unlike `XGrabKey`-based global hotkeys, the focused application still receives
//! the key, which is exactly what push-to-talk needs.
//!
//! # Threading & Shutdown
//!
//! A dedicated thread owns the X connection. `wait_for_event` cannot be
//! interrupted from another thread, so the loop polls with a short sleep and
//! checks a stop flag each iteration; `Drop` sets the flag and joins (bounded by
//! the poll interval).
//!
//! Key repeats are filtered at the source via the `KEY_REPEAT` raw event flag.

use crate::keybinds::runtime::KeybindListener;
use crate::keybinds::runtime::linux::x11::x_keycode_to_code;
use crate::keybinds::{KeyEvent, KeybindsError};
use keyboard_types::KeyState;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;
use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::xinput::{self, ConnectionExt as _};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(5);

#[derive(Debug)]
pub struct X11KeybindListener {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl KeybindListener for X11KeybindListener {
    async fn start(key_event_tx: UnboundedSender<KeyEvent>) -> Result<Self, KeybindsError>
    where
        Self: Sized,
    {
        log::debug!("Starting X11 keybind listener");

        let (startup_tx, startup_rx) = oneshot::channel();
        let stop = Arc::new(AtomicBool::new(false));

        let thread = {
            let stop = stop.clone();
            std::thread::Builder::new()
                .name("VACS_X11_Keybinds".to_string())
                .spawn(move || run_listener(key_event_tx, stop, startup_tx))
                .map_err(|err| {
                    KeybindsError::Listener(format!("Failed to spawn X11 listener thread: {err}"))
                })?
        };

        match tokio::time::timeout(STARTUP_TIMEOUT, startup_rx).await {
            Ok(Ok(Ok(()))) => {
                log::debug!("X11 keybind listener started successfully");
                Ok(Self {
                    stop,
                    thread: Some(thread),
                })
            }
            Ok(Ok(Err(err))) => {
                log::error!("X11 keybind listener startup failed: {err}");
                Err(err)
            }
            Ok(Err(_)) => {
                log::error!("X11 keybind listener startup channel closed unexpectedly");
                stop.store(true, Ordering::Relaxed);
                Err(KeybindsError::Listener(
                    "X11KeybindListener startup channel closed".to_string(),
                ))
            }
            Err(_) => {
                log::error!("X11 keybind listener startup timed out");
                stop.store(true, Ordering::Relaxed);
                Err(KeybindsError::Listener(
                    "X11KeybindListener startup timed out".to_string(),
                ))
            }
        }
    }
}

impl Drop for X11KeybindListener {
    fn drop(&mut self) {
        log::debug!("Stopping X11 keybind listener");
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            log::warn!("X11 keybind listener thread panicked");
        }
    }
}

fn run_listener(
    key_event_tx: UnboundedSender<KeyEvent>,
    stop: Arc<AtomicBool>,
    startup_tx: oneshot::Sender<Result<(), KeybindsError>>,
) {
    let init = || -> Result<x11rb::rust_connection::RustConnection, String> {
        let (conn, screen_num) =
            x11rb::connect(None).map_err(|err| format!("Failed to connect to X server: {err}"))?;

        conn.xinput_xi_query_version(2, 0)
            .map_err(|err| format!("Failed to query XInput version: {err}"))?
            .reply()
            .map_err(|err| format!("XInput2 is not supported by the X server: {err}"))?;

        let root = conn.setup().roots[screen_num].root;
        conn.xinput_xi_select_events(
            root,
            &[xinput::EventMask {
                // 1 = XIAllMasterDevices: raw events reach the root window once
                // per attached master. Selecting XIAllDevices (0) instead would
                // deliver every transition twice - once attributed to the slave
                // device and once to its master.
                deviceid: 1,
                mask: vec![
                    xinput::XIEventMask::RAW_KEY_PRESS | xinput::XIEventMask::RAW_KEY_RELEASE,
                ],
            }],
        )
        .map_err(|err| format!("Failed to select XInput2 raw key events: {err}"))?;
        conn.flush()
            .map_err(|err| format!("Failed to flush X connection: {err}"))?;

        Ok(conn)
    };

    let conn = match init() {
        Ok(conn) => conn,
        Err(err) => {
            let _ = startup_tx.send(Err(KeybindsError::Listener(err)));
            return;
        }
    };

    let _ = startup_tx.send(Ok(()));

    while !stop.load(Ordering::Relaxed) {
        match conn.poll_for_event() {
            Ok(Some(Event::XinputRawKeyPress(event))) => {
                forward_key_event(&key_event_tx, event.detail, event.flags, KeyState::Down);
            }
            Ok(Some(Event::XinputRawKeyRelease(event))) => {
                forward_key_event(&key_event_tx, event.detail, event.flags, KeyState::Up);
            }
            Ok(Some(_)) => {}
            Ok(None) => std::thread::sleep(POLL_INTERVAL),
            Err(err) => {
                log::error!("X11 keybind listener connection failed: {err}");
                break;
            }
        }
    }

    log::trace!("X11 keybind listener finished");
}

fn forward_key_event(
    key_event_tx: &UnboundedSender<KeyEvent>,
    keycode: u32,
    flags: xinput::KeyEventFlags,
    state: KeyState,
) {
    if u32::from(flags) & u32::from(xinput::KeyEventFlags::KEY_REPEAT) != 0 {
        return;
    }

    match x_keycode_to_code(keycode) {
        Ok(code) => {
            let _ = key_event_tx.send(KeyEvent::key(code, code.to_string(), state));
        }
        Err(err) => log::trace!("Ignoring unmapped X11 key event: {err}"),
    }
}
