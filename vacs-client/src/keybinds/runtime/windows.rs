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
        use windows::Win32::UI::Input::KeyboardAndMouse::*;
        // mapping based on Standard "102" keyboard layout: https://w3c.github.io/uievents-code/#keyboard-102
        match value.vk {
            // Alphanumerical section
            // Row E
            VK_OEM_3 => Ok(Backquote),
            VK_1 => Ok(Digit1),
            VK_2 => Ok(Digit2),
            VK_3 => Ok(Digit3),
            VK_4 => Ok(Digit4),
            VK_5 => Ok(Digit5),
            VK_6 => Ok(Digit6),
            VK_7 => Ok(Digit7),
            VK_8 => Ok(Digit8),
            VK_9 => Ok(Digit9),
            VK_0 => Ok(Digit0),
            VK_OEM_MINUS => Ok(Minus),
            VK_OEM_PLUS => Ok(Equal),
            VK_BACK => Ok(Backspace),
            // Row D
            VK_TAB => Ok(Tab),
            VK_Q => Ok(KeyQ),
            VK_W => Ok(KeyW),
            VK_E => Ok(KeyE),
            VK_R => Ok(KeyR),
            VK_T => Ok(KeyT),
            VK_Y => Ok(KeyY),
            VK_U => Ok(KeyU),
            VK_I => Ok(KeyI),
            VK_O => Ok(KeyO),
            VK_P => Ok(KeyP),
            VK_OEM_4 => Ok(BracketLeft),
            VK_OEM_6 => Ok(BracketRight),
            VK_OEM_5 => Ok(Backslash),
            // Row C
            VK_CAPITAL => Ok(CapsLock),
            VK_A => Ok(KeyA),
            VK_S => Ok(KeyS),
            VK_D => Ok(KeyD),
            VK_F => Ok(KeyF),
            VK_G => Ok(KeyG),
            VK_H => Ok(KeyH),
            VK_J => Ok(KeyJ),
            VK_K => Ok(KeyK),
            VK_L => Ok(KeyL),
            VK_OEM_1 => Ok(Semicolon),
            VK_OEM_7 => Ok(Quote),
            VK_RETURN => Ok(if value.extended { NumpadEnter } else { Enter }),
            // Row B
            VK_SHIFT | VK_LSHIFT | VK_RSHIFT => Ok(match value.make {
                0x2A => ShiftLeft,
                0x36 => ShiftRight,
                _ => ShiftLeft,
            }),
            VK_OEM_102 => Ok(IntlBackslash),
            VK_Z => Ok(KeyZ),
            VK_X => Ok(KeyX),
            VK_C => Ok(KeyC),
            VK_V => Ok(KeyV),
            VK_B => Ok(KeyB),
            VK_N => Ok(KeyN),
            VK_M => Ok(KeyM),
            VK_OEM_COMMA => Ok(Comma),
            VK_OEM_PERIOD => Ok(Period),
            VK_OEM_2 => Ok(Slash),
            // Row A
            VK_CONTROL | VK_LCONTROL | VK_RCONTROL => Ok(if value.extended {
                ControlRight
            } else {
                ControlLeft
            }),
            VK_LWIN => Ok(MetaLeft),
            VK_MENU | VK_LMENU | VK_RMENU => Ok(if value.extended { AltRight } else { AltLeft }),
            VK_SPACE => Ok(Space),
            VK_RWIN => Ok(MetaRight),
            VK_APPS => Ok(ContextMenu),

            // Control pad section
            // Row E
            VK_INSERT => Ok(Insert),
            VK_HOME => Ok(Home),
            VK_PRIOR => Ok(PageUp),
            // Row D
            VK_DELETE => Ok(Delete),
            VK_END => Ok(End),
            VK_NEXT => Ok(PageDown),

            // Arrow pad section
            // Row B
            VK_UP => Ok(ArrowUp),
            // Row A
            VK_LEFT => Ok(ArrowLeft),
            VK_DOWN => Ok(ArrowDown),
            VK_RIGHT => Ok(ArrowRight),

            // Numpad section
            // Row E
            VK_NUMLOCK => Ok(NumLock),
            VK_DIVIDE => Ok(NumpadDivide),
            VK_MULTIPLY => Ok(NumpadMultiply),
            VK_SUBTRACT => Ok(NumpadSubtract),
            // Row D
            VK_NUMPAD7 => Ok(Numpad7),
            VK_NUMPAD8 => Ok(Numpad8),
            VK_NUMPAD9 => Ok(Numpad9),
            VK_ADD => Ok(NumpadAdd),
            // Row C
            VK_NUMPAD4 => Ok(Numpad4),
            VK_NUMPAD5 => Ok(Numpad5),
            VK_NUMPAD6 => Ok(Numpad6),
            // Row B
            VK_NUMPAD1 => Ok(Numpad1),
            VK_NUMPAD2 => Ok(Numpad2),
            VK_NUMPAD3 => Ok(Numpad3),
            // NumpadEnter
            // Row A
            VK_NUMPAD0 => Ok(Numpad0),
            VK_DECIMAL => Ok(NumpadDecimal),

            // Function section
            // Row K
            VK_ESCAPE => Ok(Escape),
            VK_F1 => Ok(F1),
            VK_F2 => Ok(F2),
            VK_F3 => Ok(F3),
            VK_F4 => Ok(F4),
            VK_F5 => Ok(F5),
            VK_F6 => Ok(F6),
            VK_F7 => Ok(F7),
            VK_F8 => Ok(F8),
            VK_F9 => Ok(F9),
            VK_F10 => Ok(F10),
            VK_F11 => Ok(F11),
            VK_F12 => Ok(F12),
            VK_PRINT | VK_SNAPSHOT => Ok(PrintScreen),
            // "fake" extended Shift triggered at the beginning of a PrintScreen sequence
            VIRTUAL_KEY(0xFF) if value.make == 0x002A && value.extended => Ok(PrintScreen),
            VK_SCROLL => Ok(ScrollLock),
            VK_PAUSE => Ok(Pause),
            // Hidden
            VK_F13 => Ok(F13),
            VK_F14 => Ok(F14),
            VK_F15 => Ok(F15),
            VK_F16 => Ok(F16),
            VK_F17 => Ok(F17),
            VK_F18 => Ok(F18),
            VK_F19 => Ok(F19),
            VK_F20 => Ok(F20),
            VK_F21 => Ok(F21),
            VK_F22 => Ok(F22),
            VK_F23 => Ok(F23),
            VK_F24 => Ok(F24),

            // Media keys
            VK_BROWSER_BACK => Ok(BrowserBack),
            VK_BROWSER_FAVORITES => Ok(BrowserFavorites),
            VK_BROWSER_FORWARD => Ok(BrowserForward),
            VK_BROWSER_HOME => Ok(BrowserHome),
            VK_BROWSER_REFRESH => Ok(BrowserRefresh),
            VK_BROWSER_SEARCH => Ok(BrowserSearch),
            VK_BROWSER_STOP => Ok(BrowserStop),
            VK_LAUNCH_APP1 => Ok(LaunchApp1),
            VK_LAUNCH_APP2 => Ok(LaunchApp2),
            VK_LAUNCH_MAIL => Ok(LaunchMail),
            VK_MEDIA_PLAY_PAUSE => Ok(MediaPlayPause),
            VK_LAUNCH_MEDIA_SELECT => Ok(MediaSelect),
            VK_MEDIA_STOP => Ok(MediaStop),
            VK_MEDIA_NEXT_TRACK => Ok(MediaTrackNext),
            VK_MEDIA_PREV_TRACK => Ok(MediaTrackPrevious),
            VK_SLEEP => Ok(Sleep),
            VK_VOLUME_DOWN => Ok(AudioVolumeDown),
            VK_VOLUME_MUTE => Ok(AudioVolumeMute),
            VK_VOLUME_UP => Ok(AudioVolumeUp),

            _ => Err(KeybindsError::UnrecognizedCode(format!("{:?}", value))),
        }
    }
}
