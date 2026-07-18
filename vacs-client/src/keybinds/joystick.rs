//! Shared SDL3 joystick button source.
//!
//! # Architecture
//!
//! Joystick support is deliberately *not* a [`KeybindListener`](super::runtime::KeybindListener):
//! the platform listeners are owned by the [`KeybindEngine`](super::engine::KeybindEngine) and
//! only exist while at least one binding is configured, but the settings UI must be able to
//! capture "the next pressed button" before any binding exists. This service is therefore owned
//! by Tauri app state and shared by both consumers:
//!
//! - the engine subscribes when a joystick binding is configured
//! - the binding-capture IPC command subscribes on demand
//!
//! # SDL usage
//!
//! SDL is initialized lazily on the first [`subscribe`](JoystickService::subscribe) call, on a
//! dedicated thread that owns the (joystick+events only, no video/audio) SDL context and pumps
//! events. The thread then stays alive until [`shutdown`](JoystickService::shutdown): repeated
//! SDL init/quit cycles are a known source of platform bugs, and a thread parked in
//! `wait_event_timeout` is effectively free. Users who never touch joystick features never
//! initialize SDL at all.
//!
//! `SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS` is set before init so button events keep arriving
//! while another application (e.g. a radar client) has focus - a hard requirement for PTT.
//!
//! # Permissions (Linux)
//!
//! No root, udev rules, or group membership are required: udev tags joystick-capable devices
//! with `uaccess`, so logind grants the active seat user an ACL on the evdev nodes. This works
//! identically under Wayland and X11.

use crate::keybinds::{JoystickButton, KeyEvent, KeybindsError};
use keyboard_types::KeyState;
use sdl3::event::Event;
use sdl3::sys::joystick::SDL_JoystickID;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::{broadcast, oneshot};

/// Capacity of the broadcast channel between the SDL poller thread and its
/// subscribers (engine + capture). Subscribers that lag behind log and continue.
const EVENT_CHANNEL_CAPACITY: usize = 64;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
const EVENT_WAIT_TIMEOUT: Duration = Duration::from_millis(250);

pub type JoystickServiceHandle = Arc<JoystickService>;

#[derive(Debug, Default)]
pub struct JoystickService {
    poller: tokio::sync::Mutex<Option<RunningPoller>>,
}

#[derive(Debug)]
struct RunningPoller {
    tx: broadcast::Sender<KeyEvent>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl JoystickService {
    pub fn new() -> JoystickServiceHandle {
        Arc::new(Self::default())
    }

    /// Subscribe to joystick button events, lazily starting the SDL poller
    /// thread on first use.
    pub async fn subscribe(&self) -> Result<broadcast::Receiver<KeyEvent>, KeybindsError> {
        let mut poller = self.poller.lock().await;

        if let Some(running) = poller.as_ref() {
            return Ok(running.tx.subscribe());
        }

        log::debug!("Starting SDL joystick poller");

        let (tx, rx) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        let (startup_tx, startup_rx) = oneshot::channel();
        let stop = Arc::new(AtomicBool::new(false));

        let thread = {
            let tx = tx.clone();
            let stop = stop.clone();
            std::thread::Builder::new()
                .name("VACS_SDL_Joystick".to_string())
                .spawn(move || run_poller(tx, stop, startup_tx))
                .map_err(|err| {
                    KeybindsError::Listener(format!("Failed to spawn joystick thread: {err}"))
                })?
        };

        match tokio::time::timeout(STARTUP_TIMEOUT, startup_rx).await {
            Ok(Ok(Ok(()))) => {
                log::debug!("SDL joystick poller started successfully");
                *poller = Some(RunningPoller {
                    tx,
                    stop,
                    thread: Some(thread),
                });
                Ok(rx)
            }
            Ok(Ok(Err(err))) => {
                log::error!("SDL joystick poller startup failed: {err}");
                Err(err)
            }
            Ok(Err(_)) => {
                log::error!("SDL joystick poller startup channel closed unexpectedly");
                stop.store(true, Ordering::Relaxed);
                Err(KeybindsError::Listener(
                    "Joystick poller startup channel closed".to_string(),
                ))
            }
            Err(_) => {
                log::error!("SDL joystick poller startup timed out");
                stop.store(true, Ordering::Relaxed);
                Err(KeybindsError::Listener(
                    "Joystick poller startup timed out".to_string(),
                ))
            }
        }
    }

    /// Stop the poller thread (if running) and wait for it to exit.
    pub async fn shutdown(&self) {
        let running = self.poller.lock().await.take();
        if let Some(mut running) = running {
            log::debug!("Stopping SDL joystick poller");
            running.stop.store(true, Ordering::Relaxed);
            if let Some(thread) = running.thread.take()
                && thread.join().is_err()
            {
                log::warn!("SDL joystick poller thread panicked");
            }
        }
    }
}

struct OpenDevice {
    // Keeps the SDL device handle alive; button events stop without it.
    _joystick: sdl3::joystick::Joystick,
    guid: String,
    name: String,
    held: HashSet<u32>,
}

fn run_poller(
    tx: broadcast::Sender<KeyEvent>,
    stop: Arc<AtomicBool>,
    startup_tx: oneshot::Sender<Result<(), KeybindsError>>,
) {
    // Must be set before SDL_Init for the Windows backends to deliver events
    // while another window (e.g. a radar client) has focus.
    sdl3::hint::set("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS", "1");
    // SDL's events subsystem catches SIGINT/SIGTERM by default and converts
    // them into an SDL_EVENT_QUIT (which this poller ignores), which would
    // prevent the whole application from terminating on Ctrl+C. Tauri owns the
    // process lifecycle, so keep SDL away from signal handling.
    sdl3::hint::set("SDL_NO_SIGNAL_HANDLERS", "1");

    let init = || -> Result<(sdl3::JoystickSubsystem, sdl3::EventPump), String> {
        let sdl = sdl3::init().map_err(|err| format!("SDL init failed: {err}"))?;
        let joystick = sdl
            .joystick()
            .map_err(|err| format!("SDL joystick subsystem init failed: {err}"))?;
        let pump = sdl
            .event_pump()
            .map_err(|err| format!("SDL event pump init failed: {err}"))?;
        Ok((joystick, pump))
    };

    let (joystick, mut pump) = match init() {
        Ok(res) => res,
        Err(err) => {
            let _ = startup_tx.send(Err(KeybindsError::Listener(err)));
            return;
        }
    };

    let _ = startup_tx.send(Ok(()));

    // Keyed by SDL joystick instance id. Devices present at init produce
    // synthetic JoyDeviceAdded events, so no separate initial scan is needed.
    let mut open: HashMap<u32, OpenDevice> = HashMap::new();

    while !stop.load(Ordering::Relaxed) {
        let Some(event) = pump.wait_event_timeout(EVENT_WAIT_TIMEOUT) else {
            continue;
        };

        match event {
            Event::JoyDeviceAdded { which, .. } => match joystick.open(SDL_JoystickID(which)) {
                Ok(device) => {
                    let guid = device.guid().to_string();
                    let name = device.name();
                    log::info!("Joystick connected: {name} (guid {guid}, instance {which})");
                    open.insert(
                        which,
                        OpenDevice {
                            _joystick: device,
                            guid,
                            name,
                            held: HashSet::new(),
                        },
                    );
                }
                Err(err) => log::warn!("Failed to open joystick (instance {which}): {err}"),
            },
            Event::JoyDeviceRemoved { which, .. } => {
                if let Some(device) = open.remove(&which) {
                    log::info!(
                        "Joystick disconnected: {} (guid {})",
                        device.name,
                        device.guid
                    );
                    // Synthesize releases for held buttons so a PTT bound to this
                    // device cannot get stuck transmitting after an unplug.
                    for button in device.held {
                        let _ = tx.send(button_event(
                            &device.guid,
                            &device.name,
                            button,
                            KeyState::Up,
                        ));
                    }
                }
            }
            Event::JoyButtonDown {
                which, button_idx, ..
            } => {
                if let Some(device) = open.get_mut(&which) {
                    let button = u32::from(button_idx);
                    device.held.insert(button);
                    let _ = tx.send(button_event(
                        &device.guid,
                        &device.name,
                        button,
                        KeyState::Down,
                    ));
                }
            }
            Event::JoyButtonUp {
                which, button_idx, ..
            } => {
                if let Some(device) = open.get_mut(&which) {
                    let button = u32::from(button_idx);
                    device.held.remove(&button);
                    let _ = tx.send(button_event(
                        &device.guid,
                        &device.name,
                        button,
                        KeyState::Up,
                    ));
                }
            }
            _ => {}
        }
    }

    log::trace!("SDL joystick poller finished");
}

fn button_event(guid: &str, name: &str, button: u32, state: KeyState) -> KeyEvent {
    KeyEvent::button(
        JoystickButton {
            device: guid.to_string(),
            button,
            name: Some(name.to_string()),
        },
        format!("{name} B{button}"),
        state,
    )
}
