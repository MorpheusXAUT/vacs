use crate::keybinds::runtime::KeybindListener;
use crate::keybinds::runtime::linux::wayland::PortalShortcutId;
use crate::keybinds::{KeyEvent, KeybindsError};
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut, Shortcut};
use ashpd::zbus::export::futures_core::Stream;
use futures_util::StreamExt;
use keyboard_types::KeyState;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::async_runtime::JoinHandle;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct WaylandKeybindListener {
    cancellation_token: CancellationToken,
    cleanup_token: CancellationToken,
    task_handle: Option<JoinHandle<()>>,
    shortcuts: ShortcutMap,
}

impl KeybindListener for WaylandKeybindListener {
    async fn start() -> Result<(Self, UnboundedReceiver<KeyEvent>), KeybindsError>
    where
        Self: Sized,
    {
        log::debug!("Starting Wayland keybind listener");

        let (key_event_tx, key_event_rx) = unbounded_channel::<KeyEvent>();
        let (startup_tx, startup_rx) = oneshot::channel::<Result<(), KeybindsError>>();

        let cancellation_token = CancellationToken::new();
        let cleanup_token = CancellationToken::new();
        let shortcuts = Arc::new(RwLock::new(HashMap::new()));

        let task_handle = {
            let cancellation_token = cancellation_token.clone();
            let cleanup_token = cleanup_token.clone();
            let shortcuts = shortcuts.clone();

            tauri::async_runtime::spawn(async move {
                match setup_shortcuts_listener(
                    key_event_tx,
                    startup_tx,
                    cancellation_token,
                    cleanup_token,
                    shortcuts,
                )
                .await
                {
                    Ok(()) => log::trace!("Wayland keybind listener task finished"),
                    Err(err) => log::error!("Wayland keybind listener failed: {err}"),
                };
            })
        };

        match tokio::time::timeout(Duration::from_secs(10), startup_rx).await {
            Ok(Ok(Ok(()))) => {
                log::debug!("Wayland keybind listener started successfully");

                Ok((
                    Self {
                        cancellation_token,
                        cleanup_token,
                        task_handle: Some(task_handle),
                        shortcuts,
                    },
                    key_event_rx,
                ))
            }
            Ok(Ok(Err(err))) => {
                log::error!("Wayland keybind listener startup failed: {err}");

                cancellation_token.cancel();
                task_handle.abort();

                Err(err)
            }
            Ok(Err(_)) => {
                log::error!("Wayland keybind listener startup channel closed unexpectedly");

                cancellation_token.cancel();
                task_handle.abort();

                Err(KeybindsError::Listener(
                    "WaylandKeybindListener startup channel closed".to_string(),
                ))
            }
            Err(err) => {
                log::error!("Wayland keybind listener startup timed out: {err}");

                cancellation_token.cancel();
                task_handle.abort();

                Err(KeybindsError::Listener(
                    "WaylandKeybindListener startup timed out".to_string(),
                ))
            }
        }
    }

    fn get_external_binding(&self, mode: crate::config::TransmitMode) -> Option<String> {
        if let Some(id) = PortalShortcutId::from_transmit_mode(mode) {
            self.get_shortcut_binding(id)
        } else {
            None
        }
    }
}

impl Drop for WaylandKeybindListener {
    fn drop(&mut self) {
        log::debug!("Stopping Wayland keybind listener");

        self.cancellation_token.cancel();

        if let Some(handle) = self.task_handle.take() {
            let cleanup_token = self.cleanup_token.clone();

            tauri::async_runtime::spawn(async move {
                tokio::select! {
                    _ = cleanup_token.cancelled() => {
                        log::debug!("Wayland keybind listener cleanup completed");
                    }
                    _ = tokio::time::sleep(Duration::from_secs(2)) => {
                        log::warn!("Wayland keybind listener cleanup timed out, aborting");
                        handle.abort();
                    }
                }
            });
        }
    }
}

impl WaylandKeybindListener {
    pub fn get_shortcut_binding(&self, shortcut_id: PortalShortcutId) -> Option<String> {
        self.shortcuts.read().get(&shortcut_id).cloned()
    }
}

async fn setup_shortcuts_listener(
    key_event_tx: UnboundedSender<KeyEvent>,
    startup_tx: oneshot::Sender<Result<(), KeybindsError>>,
    cancellation_token: CancellationToken,
    cleanup_token: CancellationToken,
    shortcuts_map: ShortcutMap,
) -> ashpd::Result<()> {
    log::debug!("Initializing Wayland global shortcuts");

    let mut startup_tx = Some(startup_tx);

    let proxy = match tokio::time::timeout(Duration::from_secs(5), GlobalShortcuts::new()).await {
        Ok(Ok(proxy)) => proxy,
        Ok(Err(err)) => {
            log::error!("Failed to create GlobalShortcuts proxy: {err}");
            if let Some(tx) = startup_tx.take() {
                let _ = tx.send(Err(KeybindsError::Listener(
                    "Portal unavailable".to_string(),
                )));
            }
            return Err(err);
        }
        Err(_) => {
            log::error!("Timed out creating GlobalShortcuts proxy");
            if let Some(tx) = startup_tx.take() {
                let _ = tx.send(Err(KeybindsError::Listener(
                    "Portal unavailable".to_string(),
                )));
            }
            return Err(ashpd::Error::NoResponse);
        }
    };

    let session = match tokio::time::timeout(Duration::from_secs(5), proxy.create_session()).await {
        Ok(Ok(session)) => session,
        Ok(Err(err)) => {
            log::error!("Failed to create shortcuts session: {err}");
            if let Some(tx) = startup_tx.take() {
                let _ = tx.send(Err(KeybindsError::Listener(
                    "Portal session failed".to_string(),
                )));
            }
            return Err(err);
        }
        Err(_) => {
            log::error!("Timed out creating shortcuts session");
            if let Some(tx) = startup_tx.take() {
                let _ = tx.send(Err(KeybindsError::Listener(
                    "Portal session failed".to_string(),
                )));
            }
            return Err(ashpd::Error::NoResponse);
        }
    };

    log::trace!("Checking for existing shortcuts");
    let request = proxy.list_shortcuts(&session).await.map_err(|err| {
        log::error!("Failed to list shortcuts: {err}");
        if let Some(tx) = startup_tx.take() {
            let _ = tx.send(Err(KeybindsError::Listener(
                "Failed to list shortcuts".to_string(),
            )));
        }
        err
    })?;

    let needs_bind = match request.response() {
        Ok(response) if !response.shortcuts().is_empty() => {
            let shortcuts = response.shortcuts();
            log::trace!("Found {} existing shortcuts", shortcuts.len());

            let own_ids = PortalShortcutId::all()
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>();
            if shortcuts.iter().any(|s| own_ids.contains(&s.id())) {
                log::trace!("Existing shortcuts found, skipping binding");
                update_shortcuts_map(&shortcuts_map, shortcuts);
                false
            } else {
                log::trace!("No matching shortcuts found, binding");
                true
            }
        }
        Ok(_) => {
            log::trace!("No existing shortcuts found, binding");
            true
        }
        Err(err) => {
            log::warn!("Failed to get list shortcuts response, binding: {err}");
            true
        }
    };

    if needs_bind {
        let shortcuts = PortalShortcutId::all()
            .iter()
            .map(NewShortcut::from)
            .collect::<Vec<_>>();

        log::trace!(
            "Binding {} shortcuts, signaling startup completion to avoid timeout during setup",
            shortcuts.len()
        );
        let _ = startup_tx.take().map(|tx| tx.send(Ok(())));

        let request = proxy
            .bind_shortcuts(&session, &shortcuts, None)
            .await
            .map_err(|err| {
                log::error!("Failed to bind shortcuts: {err}");
                if let Some(tx) = startup_tx.take() {
                    let _ = tx.send(Err(KeybindsError::Listener(
                        "Failed to bind shortcuts".to_string(),
                    )));
                }
                err
            })?;

        let response = request.response().map_err(|err| {
            log::error!("Failed to get bind shortcuts response: {err}");
            if let Some(tx) = startup_tx.take() {
                let _ = tx.send(Err(KeybindsError::Listener(
                    "Failed to bind shortcuts".to_string(),
                )));
            }
            err
        })?;

        let bound_shortcuts = response.shortcuts();
        log::trace!("Received {} bound shortcuts", bound_shortcuts.len());

        update_shortcuts_map(&shortcuts_map, bound_shortcuts);

        let configured_shortcuts = bound_shortcuts
            .iter()
            .filter(|s| !s.trigger_description().is_empty())
            .collect::<Vec<_>>();
        if configured_shortcuts.is_empty() {
            log::warn!("No shortcuts configured");

            match proxy.configure_shortcuts(&session, None, None).await {
                Ok(()) => {
                    log::debug!(
                        "Shortcut configuration UI shown, please configure at least one shortcut"
                    );
                }
                Err(ashpd::Error::RequiresVersion(required, actual)) => {
                    log::warn!(
                        "Portal version {actual} does not support configure_shortcuts (requires v{required}), please configure shortcuts manually in your desktop environment settings"
                    );
                }
                Err(err) => {
                    log::warn!("Failed to show shortcut configuration UI: {err}");
                }
            }

            if let Some(tx) = startup_tx.take() {
                let _ = tx.send(Err(KeybindsError::Listener(
                    "No shortcuts configured".to_string(),
                )));
            }
            return Err(ashpd::Error::Response(ashpd::desktop::ResponseError::Other));
        }

        log::trace!("Shortcuts configured: {:?}", configured_shortcuts);
    } else {
        log::trace!("Shortcuts already bound, signaling startup completion");
        let _ = startup_tx.take().map(|tx| tx.send(Ok(())));
    }

    let activated = proxy.receive_activated().await?;
    let deactivated = proxy.receive_deactivated().await?;
    let shortcuts_changed = proxy.receive_shortcuts_changed().await?;

    let res = run_shortcuts_listener(
        key_event_tx,
        cancellation_token,
        &shortcuts_map,
        activated,
        deactivated,
        shortcuts_changed,
    )
    .await;

    log::trace!("Cleaning up Wayland global shortcuts session");
    if let Err(err) = tokio::time::timeout(Duration::from_secs(2), session.close()).await {
        log::warn!("Failed to close global shortcuts session: {err}");
    }

    cleanup_token.cancel();

    res
}

async fn run_shortcuts_listener(
    key_event_tx: UnboundedSender<KeyEvent>,
    cancellation_token: CancellationToken,
    shortcuts_map: &ShortcutMap,
    mut activated: impl Stream<Item = ashpd::desktop::global_shortcuts::Activated> + Unpin,
    mut deactivated: impl Stream<Item = ashpd::desktop::global_shortcuts::Deactivated> + Unpin,
    mut shortcuts_changed: impl Stream<Item = ashpd::desktop::global_shortcuts::ShortcutsChanged>
    + Unpin,
) -> ashpd::Result<()> {
    log::trace!("Starting Wayland shortcuts listener");
    loop {
        tokio::select! {
            biased;

            _ = cancellation_token.cancelled() => {
                log::debug!("Wayland shortcuts listener cancelled");
                break;
            }

            Some(signal) = activated.next() => {
                let shortcut_id = signal.shortcut_id();
                if let Ok(shortcut_id) = PortalShortcutId::try_from(shortcut_id) {
                    log::trace!("Shortcut activated: {shortcut_id:?}");

                    let _ = key_event_tx.send(KeyEvent {
                        code: shortcut_id.into(),
                        label: shortcut_id.as_str().to_string(),
                        state: KeyState::Down
                    });
                } else {
                    log::warn!("Unknown shortcut activated: {shortcut_id}");
                }
            }

            Some(signal) = deactivated.next() => {
                let shortcut_id = signal.shortcut_id();
                if let Ok(shortcut_id) = PortalShortcutId::try_from(shortcut_id) {
                    log::trace!("Shortcut deactivated: {shortcut_id:?}");

                    let _ = key_event_tx.send(KeyEvent {
                        code: shortcut_id.into(),
                        label: shortcut_id.as_str().to_string(),
                        state: KeyState::Up
                    });
                } else {
                    log::warn!("Unknown shortcut deactivated: {shortcut_id}");
                }
            }

            Some(signal) = shortcuts_changed.next() => {
                let updated_shortcuts = signal.shortcuts();
                log::debug!("Shortcuts changed, updating {} entries", updated_shortcuts.len());

                {
                    let mut map = shortcuts_map.write();
                    for shortcut in updated_shortcuts {
                        if let Ok(id) = PortalShortcutId::try_from(shortcut.id()) {
                            let trigger = shortcut.trigger_description();

                            if trigger.is_empty() {
                                if map.remove(&id).is_some() {
                                    log::trace!("Removed shortcut binding: {id:?}");
                                }
                            } else {
                                let previous = map.insert(id, trigger.to_string());
                                if let Some(previous) = previous {
                                    if previous != trigger {
                                        log::trace!("Updated shortcut binding {}: {} -> {trigger}", shortcut.id(), previous);
                                    }
                                } else {
                                    log::trace!("Shortcut configured: {} -> {trigger}", shortcut.id());
                                }
                            }
                        }
                    }

                    log::debug!("Updated shortcuts map with {} entries", map.len());
                }
            }

            else => {
                log::warn!("Signal streams ended unexpectedly");
                break;
            }
        }
    }

    log::trace!("Wayland shortcuts listener finished");
    Ok(())
}

fn update_shortcuts_map(shortcut_map: &ShortcutMap, bound_shortcuts: &[Shortcut]) {
    let mut map = shortcut_map.write();
    map.clear();

    for shortcut in bound_shortcuts {
        if let Ok(id) = PortalShortcutId::try_from(shortcut.id()) {
            let trigger = shortcut.trigger_description();
            if !trigger.is_empty() {
                map.insert(id, trigger.to_string());
            }
        }
    }

    if map.is_empty() {
        log::warn!("No shortcuts bound");
    } else {
        log::debug!("Updated {} shortcuts", map.len());
    }
}

type ShortcutMap = Arc<RwLock<HashMap<PortalShortcutId, String>>>;
