use crate::keybinds::runtime::KeybindListener;
use crate::keybinds::runtime::macos::KeyEventConverter;
use crate::keybinds::{KeyEvent, KeybindsError};
use objc2_core_foundation::{
    CFMachPort, CFRetained, CFRunLoop, CFRunLoopSource, CFRunLoopSourceContext,
    kCFRunLoopCommonModes,
};
use objc2_core_graphics::{
    CGEvent, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventTapProxy,
    CGEventType, kCGEventMaskForAllEvents,
};
use std::ffi::c_void;
use std::ptr::NonNull;
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::oneshot;

#[derive(Debug)]
struct ShutdownSource {
    source: CFRetained<CFRunLoopSource>,
    runloop: CFRetained<CFRunLoop>,
}

/// SAFETY: We only call thread-safe CF functions (CFRunLoopSourceSignal + CFRunLoopStop) from
/// other threads. The perform callback runs on the runloop thread. Safe to mark Send + Sync.
/// Source: ChatGPT ;)
unsafe impl Send for ShutdownSource {}
unsafe impl Sync for ShutdownSource {}

#[derive(Debug)]
pub struct MacOsKeybindListener {
    shutdown_source: ShutdownSource,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl KeybindListener for MacOsKeybindListener {
    async fn start() -> Result<(Self, UnboundedReceiver<KeyEvent>), KeybindsError>
    where
        Self: Sized,
    {
        log::debug!("Starting macos keybind listener");
        let (key_event_tx, key_event_rx) = unbounded_channel::<KeyEvent>();
        let (startup_res_tx, start_res_rx) =
            oneshot::channel::<Result<ShutdownSource, KeybindsError>>();

        let thread_handle = thread::Builder::new()
            .name("VACS_Tap_CFRunLoop".to_string())
            .spawn(move || {
                log::debug!("Message thread started");
                match Self::setup_input_listener(key_event_tx) {
                    Ok(shutdown_source) => {
                        log::trace!("Successfully created CGEventTap, running CFRunLoop");
                        let _ = startup_res_tx.send(Ok(shutdown_source));
                        Self::run_message_loop();
                    }
                    Err(err) => {
                        let _ = startup_res_tx.send(Err(err));
                    }
                }
                log::debug!("Message thread finished");
            })
            .map_err(|err| KeybindsError::Listener(format!("Failed to spawn thread: {err}")))?;

        match tokio::time::timeout(Duration::from_secs(1), start_res_rx).await {
            Ok(Ok(Ok(shutdown_source))) => Ok((
                Self {
                    shutdown_source,
                    thread_handle: Some(thread_handle),
                },
                key_event_rx,
            )),
            Ok(Ok(Err(err))) => Err(err),
            Ok(Err(_)) => Err(KeybindsError::Listener(
                "MacOsKeybindListener startup channel closed".to_string(),
            )),
            Err(_) => Err(KeybindsError::Listener(
                "MacOsKeybindListener startup timed out".to_string(),
            )),
        }
    }
}

impl Drop for MacOsKeybindListener {
    fn drop(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            log::debug!("Stopping macos keybind listener");
            self.shutdown_source.source.signal();
            self.shutdown_source.runloop.stop();
            _ = handle.join();
        }
    }
}

impl MacOsKeybindListener {
    fn setup_input_listener(
        key_event_tx: UnboundedSender<KeyEvent>,
    ) -> Result<ShutdownSource, KeybindsError> {
        let ctx_ptr = Box::into_raw(Box::new(CallbackContext {
            tx: key_event_tx,
            converter: KeyEventConverter::new(),
        })) as *mut c_void;

        let tap = unsafe {
            CGEvent::tap_create(
                CGEventTapLocation::HIDEventTap,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::ListenOnly,
                kCGEventMaskForAllEvents.into(),
                Some(callback),
                ctx_ptr,
            )
            .ok_or_else(|| KeybindsError::Listener("CGEvent::tap_create failed".to_string()))?
        };

        let loop_source =
            CFMachPort::new_run_loop_source(None, Some(&tap), 0).ok_or_else(|| {
                KeybindsError::Listener("CFMachPort::new_run_loop_source failed".to_string())
            })?;

        let current_loop = CFRunLoop::current().unwrap();

        current_loop.add_source(Some(&loop_source), unsafe { kCFRunLoopCommonModes });
        CGEvent::tap_enable(&tap, true);

        let mut shutdown_context: CFRunLoopSourceContext = unsafe { std::mem::zeroed() };
        shutdown_context.perform = Some(shutdown_perform);

        let shutdown_source = unsafe {
            CFRunLoopSource::new(None, 0, &shutdown_context as *const _ as *mut _)
                .ok_or_else(|| KeybindsError::Listener("CFRunLoopSource::new failed".to_string()))?
        };

        current_loop.add_source(Some(&shutdown_source), unsafe { kCFRunLoopCommonModes });

        log::debug!("Started macos keybind listener");
        Ok(ShutdownSource {
            source: loop_source,
            runloop: current_loop,
        })
    }

    fn run_message_loop() {
        CFRunLoop::run();
    }
}

struct CallbackContext {
    tx: UnboundedSender<KeyEvent>,
    converter: KeyEventConverter,
}

unsafe extern "C-unwind" fn callback(
    _proxy: CGEventTapProxy,
    event_type: CGEventType,
    cg_event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    if user_info.is_null()
        || (event_type != CGEventType::KeyDown
            && event_type != CGEventType::KeyUp
            && event_type != CGEventType::FlagsChanged)
    {
        return cg_event.as_ptr();
    }

    let ctx = unsafe { &mut *(user_info as *mut CallbackContext) };

    let event = unsafe { cg_event.as_ref() };

    match ctx.converter.event_to_key_event(event_type, event) {
        Ok(key_event) => {
            if let Err(err) = ctx.tx.send(key_event) {
                log::error!("Failed to send keybinds event: {err}");
            }
        }
        Err(err) => {
            log::warn!("Failed to convert event to code: {err}");
        }
    }

    cg_event.as_ptr()
}

unsafe extern "C-unwind" fn shutdown_perform(_: *mut c_void) {
    CFRunLoop::stop(&CFRunLoop::current().unwrap());
}
