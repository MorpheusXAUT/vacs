use crate::keybinds::runtime::KeybindRuntime;
use crate::keybinds::{KeyEvent, KeybindsError};
use keyboard_types::{Code, KeyState};
use std::fmt::{Debug, Formatter};
use std::mem::zeroed;
use std::sync::mpsc;
use std::time::Duration;
use std::{ptr, thread};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use windows::Win32::Foundation::{GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyNameTextW, VIRTUAL_KEY};
use windows::Win32::UI::Input::{
    GetRawInputData, HRAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RAWKEYBOARD, RID_INPUT,
    RIDEV_INPUTSINK, RIM_TYPEKEYBOARD, RegisterRawInputDevices,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DispatchMessageW, GWLP_USERDATA,
    GetMessageW, GetWindowLongPtrW, HWND_MESSAGE, MSG, PostQuitMessage, PostThreadMessageW,
    RI_KEY_E0, RegisterClassW, SetWindowLongPtrW, TranslateMessage, WM_DESTROY, WM_INPUT,
    WM_KEYDOWN, WM_KEYUP, WM_NCDESTROY, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP, WNDCLASSW,
};
use windows::core::{PCWSTR, w};

#[derive(Debug)]
pub struct WindowsKeybindRuntime {
    thread_id: u32,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl KeybindRuntime for WindowsKeybindRuntime {
    fn start() -> Result<(Self, UnboundedReceiver<KeyEvent>), KeybindsError>
    where
        Self: Sized,
    {
        log::debug!("Starting windows keybind runtime");
        let (key_event_tx, key_event_rx) = unbounded_channel::<KeyEvent>();
        let (startup_res_tx, start_res_rx) = mpsc::sync_channel::<Result<u32, KeybindsError>>(1);

        let thread_handle = thread::Builder::new().name("VACS_RawInput_MessageLoop".to_string())
            .spawn(move || {
                log::debug!("Message thread started");
                match Self::setup_input_listener(key_event_tx) {
                    Ok(hwnd) => {
                        let thread_id = unsafe { GetCurrentThreadId() };
                        log::trace!("Successfully created hidden message window {hwnd:?}, running message loop on thread {thread_id}");
                        let _ = startup_res_tx.send(Ok(thread_id));
                        Self::run_message_loop();
                    }
                    Err(err) => {
                        let _ = startup_res_tx.send(Err(err));
                    }
                }
                log::debug!("Message thread finished");
            }).map_err(|err| KeybindsError::Runtime(format!("Failed to spawn thread: {err}")))?;

        match start_res_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Ok(thread_id)) => Ok((
                Self {
                    thread_handle: Some(thread_handle),
                    thread_id,
                },
                key_event_rx,
            )),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(KeybindsError::Runtime(
                "WindowsKeybindRuntime startup timed out".to_string(),
            )),
        }
    }

    fn stop(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            log::debug!("Stopping Windows keybind runtime");
            unsafe {
                if let Err(err) = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0))
                {
                    log::warn!(
                        "Failed to send quit message to thread: {err} - {:?}",
                        GetLastError()
                    );
                };
            }
            _ = handle.join();
        }
    }
}

impl Drop for WindowsKeybindRuntime {
    fn drop(&mut self) {
        self.stop();
    }
}

impl WindowsKeybindRuntime {
    fn setup_input_listener(tx: UnboundedSender<KeyEvent>) -> Result<HWND, KeybindsError> {
        let hmodule = unsafe {
            GetModuleHandleW(None).map_err(|_| {
                KeybindsError::Runtime(format!("GetModuleHandleW failed: {:?}", GetLastError()))
            })?
        };
        let hinstance = HINSTANCE(hmodule.0);

        let class_name = w!("VACS_RawInput_HiddenWindow");
        Self::ensure_class(hinstance, class_name)?;

        let hwnd = unsafe {
            CreateWindowExW(
                Default::default(),
                class_name,
                w!(""),
                Default::default(),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(hinstance),
                None,
            )
            .map_err(|_| {
                KeybindsError::Runtime(format!("CreateWindowExW failed: {:?}", GetLastError()))
            })?
        };

        if hwnd.0.is_null() {
            return Err(KeybindsError::Runtime(format!(
                "CreateWindowExW returned null: {:?}",
                unsafe { GetLastError() }
            )));
        }

        unsafe {
            Self::put_key_event_tx(hwnd, Box::new(tx));
        }

        let rid = RAWINPUTDEVICE {
            usUsagePage: 0x01, // Generic Desktop Controls
            usUsage: 0x06,     // Keyboard
            dwFlags: RIDEV_INPUTSINK,
            hwndTarget: hwnd,
        };

        unsafe {
            RegisterRawInputDevices(&[rid], size_of::<RAWINPUTDEVICE>() as u32).map_err(|_| {
                KeybindsError::Runtime(format!(
                    "RegisterRawInputDevices failed: {:?}",
                    GetLastError()
                ))
            })?;
        }

        Ok(hwnd)
    }

    fn ensure_class(hinstance: HINSTANCE, class_name: PCWSTR) -> Result<(), KeybindsError> {
        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(Self::wnd_proc),
            hInstance: hinstance,
            lpszClassName: class_name,
            ..Default::default()
        };

        let atom = unsafe { RegisterClassW(&wnd_class) };
        if atom == 0 {
            let err = unsafe { GetLastError() };
            if err != windows::Win32::Foundation::ERROR_CLASS_ALREADY_EXISTS {
                return Err(KeybindsError::Runtime(format!(
                    "RegisterClassW failed: {:?}",
                    err
                )));
            }
        }

        Ok(())
    }

    extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_INPUT => unsafe {
                if let Some((raw_key, state)) = Self::read_raw_input(HRAWINPUT(lparam.0 as _)) {
                    let code: Result<Code, KeybindsError> = raw_key.try_into();
                    match code {
                        Ok(code) => {
                            let label = Self::physical_key_label(raw_key.make, raw_key.extended)
                                .unwrap_or_else(|| code.to_string());
                            #[cfg(feature = "log-key-events")]
                            log::trace!("{code:?} [{label}] ({raw_key:?}) -> {state:?}");
                            Self::with_key_event_tx(hwnd, |tx| {
                                if let Err(err) = tx.send(KeyEvent { code, label, state }) {
                                    log::error!("Failed to send keybinds event: {err}")
                                }
                            });
                        }
                        Err(err) => {
                            log::warn!("Failed to convert virtual key to code: {err}");
                        }
                    }
                }

                LRESULT(0)
            },
            WM_DESTROY => unsafe {
                PostQuitMessage(0);
                LRESULT(0)
            },
            WM_NCDESTROY => unsafe {
                Self::drop_key_event_tx(hwnd);
                DefWindowProcW(hwnd, msg, wparam, lparam)
            },
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }

    fn read_raw_input(hraw: HRAWINPUT) -> Option<(RawKey, KeyState)> {
        let mut needed: u32 = 0;
        let header_size = size_of::<RAWINPUTHEADER>();

        if unsafe { GetRawInputData(hraw, RID_INPUT, None, &mut needed, header_size as u32) } != 0
            || needed == 0
        {
            return None;
        }

        let mut buf = vec![0u8; needed as usize];
        let read = unsafe {
            GetRawInputData(
                hraw,
                RID_INPUT,
                Some(buf.as_mut_ptr() as *mut _),
                &mut needed,
                header_size as u32,
            )
        };
        if read == 0 || read != needed {
            return None;
        }

        if buf.len() < header_size {
            return None;
        }

        let header: RAWINPUTHEADER =
            unsafe { ptr::read_unaligned(buf.as_ptr() as *const RAWINPUTHEADER) };
        if header.dwType != RIM_TYPEKEYBOARD.0 {
            return None;
        }

        let need = header_size + size_of::<RAWKEYBOARD>();
        if buf.len() < need {
            return None;
        }

        let kb_ptr = unsafe { buf.as_ptr().add(header_size) } as *const RAWKEYBOARD;
        let kb: RAWKEYBOARD = unsafe { ptr::read_unaligned(kb_ptr) };

        let state = match kb.Message {
            WM_KEYDOWN | WM_SYSKEYDOWN => KeyState::Down,
            WM_KEYUP | WM_SYSKEYUP => KeyState::Up,
            _ => return None,
        };
        let extended = (kb.Flags & RI_KEY_E0 as u16) != 0;

        Some((
            RawKey {
                vk: VIRTUAL_KEY(kb.VKey),
                make: kb.MakeCode,
                extended,
            },
            state,
        ))
    }

    fn physical_key_label(scan_code: u16, extended: bool) -> Option<String> {
        let lparam: i32 = ((scan_code as i32) << 16) | if extended { 1 << 24 } else { 0 };

        let mut buf = [0u16; 64];
        let n = unsafe { GetKeyNameTextW(lparam, &mut buf) };
        if n > 0 {
            String::from_utf16(&buf[..n as usize])
                .map(|s| s.to_uppercase())
                .ok()
        } else {
            None
        }
    }

    fn run_message_loop() {
        unsafe {
            let mut msg: MSG = zeroed();
            loop {
                let r = GetMessageW(&mut msg, None, 0, 0);
                if r.0 == -1 {
                    log::error!("GetMessageW failed: {:?}", GetLastError());
                    break;
                } else if r.0 == 0 {
                    // WM_QUIT
                    log::trace!("Received WM_QUIT, exiting message loop");
                    break;
                } else {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
    }

    /// Stores a boxed `UnboundedSender<KeyEvent>` in the window’s `GWLP_USERDATA`.
    ///
    /// This transfers ownership of `tx` into the window. The pointer must later be
    /// reclaimed exactly once (e.g. via [`Self::take_key_event_tx`] or [`Self::drop_key_event_tx`])
    /// to avoid a memory leak.
    ///
    /// # Safety
    ///
    /// - `hwnd` must be a valid window handle for the lifetime of the stored pointer.
    /// - You must not overwrite a previously stored pointer without first reclaiming it
    ///   (otherwise you will leak or later double-free).
    /// - The pointer stored in `GWLP_USERDATA` is assumed to be produced by
    ///   `Box::into_raw::<UnboundedSender<KeyEvent>>` and not mutated to another type.
    /// - This function transfers ownership of `tx`; do not use `tx` after this call.
    #[inline]
    unsafe fn put_key_event_tx(hwnd: HWND, tx: Box<UnboundedSender<KeyEvent>>) {
        unsafe {
            debug_assert_eq!(
                GetWindowLongPtrW(hwnd, GWLP_USERDATA),
                0,
                "GWLP_USERDATA not empty"
            );
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(tx) as isize);
        }
    }

    /// Retrieves the `UnboundedSender<KeyEvent>` from `GWLP_USERDATA` (if any) and
    /// passes a shared reference to the provided closure `f`.
    ///
    /// Ownership is **not** taken; the pointer remains stored in the window.
    ///
    /// # Safety
    ///
    /// - `hwnd` must be a valid window handle, and its `GWLP_USERDATA` (if non-null)
    ///   must point to a valid `UnboundedSender<KeyEvent>` that has not been freed.
    /// - No other code may concurrently free or mutate the stored pointer during this call.
    /// - The reference passed to `f` must not escape the closure (no storing it with
    ///   a longer lifetime than the underlying allocation).
    #[inline]
    unsafe fn with_key_event_tx<F: FnOnce(&UnboundedSender<KeyEvent>)>(hwnd: HWND, f: F) {
        unsafe {
            let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut UnboundedSender<KeyEvent>;
            if !p.is_null() {
                f(&*p);
            }
        }
    }

    /// Takes ownership of the `UnboundedSender<KeyEvent>` stored in `GWLP_USERDATA`,
    /// if present, by reconstructing the `Box` from the raw pointer.
    ///
    /// After a successful take, the pointer is no longer valid to read/deref until
    /// reinstalled. This function does **not** clear `GWLP_USERDATA`; pair it with
    /// a `SetWindowLongPtrW(..., 0)` if you want to explicitly clear the slot (e.g., using [`Self::drop_key_Event_tx`]).
    ///
    /// # Safety
    ///
    /// - `hwnd` must be a valid window handle.
    /// - If `GWLP_USERDATA` is non-null, it must have been produced by
    ///   `Box::into_raw::<UnboundedSender<KeyEvent>>` and not previously taken or freed.
    /// - Calling this twice without reinstalling a fresh pointer will cause a double free.
    /// - No other code may concurrently take/free the same pointer.
    #[inline]
    unsafe fn take_key_event_tx(hwnd: HWND) -> Option<Box<UnboundedSender<KeyEvent>>> {
        unsafe {
            let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut UnboundedSender<KeyEvent>;
            if p.is_null() {
                return None;
            }
            Some(Box::from_raw(p))
        }
    }

    /// Drops (frees) the `UnboundedSender<KeyEvent>` stored in `GWLP_USERDATA` (if any)
    /// and clears the slot to `0`.
    ///
    /// This is a convenience that combines [`Self::take_key_event_tx`] with clearing the
    /// window data to prevent accidental reuse of a dangling pointer.
    ///
    /// # Safety
    ///
    /// - `hwnd` must be a valid window handle.
    /// - The pointer in `GWLP_USERDATA` (if non-null) must have been produced by
    ///   `Box::into_raw::<UnboundedSender<KeyEvent>>` and not already freed.
    /// - No other code may concurrently take/free the same pointer.
    /// - After this call, `GWLP_USERDATA` is set to `0`.
    #[inline]
    unsafe fn drop_key_event_tx(hwnd: HWND) {
        unsafe {
            if let Some(tx) = Self::take_key_event_tx(hwnd) {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(tx);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RawKey {
    vk: VIRTUAL_KEY,
    make: u16, // Scan 1 Make code: https://learn.microsoft.com/en-us/windows/win32/inputdev/about-keyboard-input#scan-codes
    extended: bool,
}

impl Debug for RawKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawKey")
            .field("vk", &format_args!("{:#X}", self.vk.0))
            .field("make", &format_args!("{:#06X}", self.make))
            .field("extended", &self.extended)
            .finish()
    }
}

impl TryFrom<RawKey> for Code {
    type Error = KeybindsError;

    fn try_from(value: RawKey) -> Result<Self, Self::Error> {
        use Code::*;
        use windows::Win32::UI::Input::KeyboardAndMouse::VK_CONTROL;
        // mapping based on Standard "102" keyboard layout: https://w3c.github.io/uievents-code/#keyboard-102
        // and Scan 1 Make codes: https://learn.microsoft.com/en-us/windows/win32/inputdev/about-keyboard-input#scan-codes
        match value.make {
            // Alphanumerical section
            // Row E
            0x0029 => Ok(Backquote),
            0x0002 => Ok(Digit1),
            0x0003 => Ok(Digit2),
            0x0004 => Ok(Digit3),
            0x0005 => Ok(Digit4),
            0x0006 => Ok(Digit5),
            0x0007 => Ok(Digit6),
            0x0008 => Ok(Digit7),
            0x0009 => Ok(Digit8),
            0x000A => Ok(Digit9),
            0x000B => Ok(Digit0),
            0x000C => Ok(Minus),
            0x000D => Ok(Equal),
            0x000E => Ok(Backspace),
            // Row D
            0x000F => Ok(Tab),
            0x0010 => Ok(KeyQ),
            0x0011 => Ok(KeyW),
            0x0012 => Ok(KeyE),
            0x0013 => Ok(KeyR),
            0x0014 => Ok(KeyT),
            0x0015 => Ok(KeyY),
            0x0016 => Ok(KeyU),
            0x0017 => Ok(KeyI),
            0x0018 => Ok(KeyO),
            0x0019 => Ok(KeyP),
            0x001A => Ok(BracketLeft),
            0x001B => Ok(BracketRight),
            0x002B => Ok(Backslash),
            // Row C
            0x003A => Ok(CapsLock),
            0x001E => Ok(KeyA),
            0x001F => Ok(KeyS),
            0x0020 => Ok(KeyD),
            0x0021 => Ok(KeyF),
            0x0022 => Ok(KeyG),
            0x0023 => Ok(KeyH),
            0x0024 => Ok(KeyJ),
            0x0025 => Ok(KeyK),
            0x0026 => Ok(KeyL),
            0x0027 => Ok(Semicolon),
            0x0028 => Ok(Quote),
            0x001C => Ok(if value.extended { NumpadEnter } else { Enter }),
            // Row B
            0x002A => Ok(if value.extended && value.vk == VIRTUAL_KEY(0xFF) {
                // "fake" extended Shift triggered at the beginning of a PrintScreen sequence
                PrintScreen
            } else {
                ShiftLeft
            }),
            0x0056 => Ok(IntlBackslash),
            0x002C => Ok(KeyZ),
            0x002D => Ok(KeyX),
            0x002E => Ok(KeyC),
            0x002F => Ok(KeyV),
            0x0030 => Ok(KeyB),
            0x0031 => Ok(KeyN),
            0x0032 => Ok(KeyM),
            0x0033 => Ok(Comma),
            0x0034 => Ok(Period),
            0x0035 => Ok(if value.extended { NumpadDivide } else { Slash }),
            0x0036 => Ok(ShiftRight),
            // Row A
            0x001D => Ok(if value.extended {
                ControlRight
            } else {
                ControlLeft
            }),
            0x005B => Ok(MetaLeft),
            0x0038 => Ok(if value.extended {
                if value.vk == VK_CONTROL {
                    ControlRight
                } else {
                    AltRight
                }
            } else {
                AltLeft
            }),
            0x0039 => Ok(Space),
            0xE038 => Ok(AltRight),
            0x005C => Ok(MetaRight),
            0x005D => Ok(ContextMenu),
            0xE01D => Ok(ControlRight),

            // Arrow pad section
            // Row B
            0xE048 => Ok(ArrowUp),
            // Row A
            0xE04B => Ok(ArrowLeft),
            0xE050 => Ok(ArrowDown),
            0xE04D => Ok(ArrowRight),

            // Control pad section
            // Numpad section
            // Row E
            0x0045 | 0xE045 => Ok(NumLock),
            0x0037 => Ok(if value.extended {
                PrintScreen
            } else {
                NumpadMultiply
            }),
            0x004A => Ok(NumpadSubtract),
            // Row D
            0x0047 => Ok(if value.extended { Home } else { Numpad7 }),
            0x0048 => Ok(Numpad8),
            0x0049 => Ok(if value.extended { PageUp } else { Numpad9 }),
            0x004E => Ok(NumpadAdd),
            // Row C
            0x004B => Ok(Numpad4),
            0x004C => Ok(Numpad5),
            0x004D => Ok(Numpad6),
            // Row B
            0x004F => Ok(if value.extended { End } else { Numpad1 }),
            0x0050 => Ok(Numpad2),
            0x0051 => Ok(if value.extended { PageDown } else { Numpad3 }),
            // Row A
            0x0052 => Ok(if value.extended { Insert } else { Numpad0 }),
            0x0053 => Ok(if value.extended {
                Delete
            } else {
                NumpadDecimal
            }),

            // Function section
            // Row K
            0x0001 => Ok(Escape),
            0x003B => Ok(F1),
            0x003C => Ok(F2),
            0x003D => Ok(F3),
            0x003E => Ok(F4),
            0x003F => Ok(F5),
            0x0040 => Ok(F6),
            0x0041 => Ok(F7),
            0x0042 => Ok(F8),
            0x0043 => Ok(F9),
            0x0044 => Ok(F10),
            0x0057 => Ok(F11),
            0x0058 => Ok(F12),
            0xE037 | 0x0054 => Ok(PrintScreen),
            0x0046 => Ok(ScrollLock),
            0xE046 => Ok(Pause),
            // Hidden
            0x0064 => Ok(F13),
            0x0065 => Ok(F14),
            0x0066 => Ok(F15),
            0x0067 => Ok(F16),
            0x0068 => Ok(F17),
            0x0069 => Ok(F18),
            0x006A => Ok(F19),
            0x006B => Ok(F20),
            0x006C => Ok(F21),
            0x006D => Ok(F22),
            0x006E => Ok(F23),
            0x0076 => Ok(F24),

            // Media keys
            0xE06A => Ok(BrowserBack),
            0xE066 => Ok(BrowserFavorites),
            0xE069 => Ok(BrowserForward),
            0xE032 => Ok(BrowserHome),
            0xE067 => Ok(BrowserRefresh),
            0xE065 => Ok(BrowserSearch),
            0xE068 => Ok(BrowserStop),
            0xE06D => Ok(LaunchControlPanel),
            0xE06C => Ok(LaunchMail),
            0xE022 => Ok(MediaPlayPause),
            0xE024 => Ok(MediaStop),
            0xE019 => Ok(MediaTrackNext),
            0xE010 => Ok(MediaTrackPrevious),
            0xE05E => Ok(Power),
            0xE05F => Ok(Sleep),
            0xE063 => Ok(WakeUp),
            0xE02E => Ok(AudioVolumeDown),
            0xE020 => Ok(AudioVolumeMute),
            0xE030 => Ok(AudioVolumeUp),

            _ => Err(KeybindsError::UnrecognizedCode(format!("{:?}", value))),
        }
    }
}
