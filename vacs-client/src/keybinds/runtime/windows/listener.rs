use crate::keybinds::runtime::KeybindListener;
use crate::keybinds::runtime::windows::RawKey;
use crate::keybinds::{KeyEvent, KeybindsError};
use keyboard_types::{Code, KeyState};
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
pub struct WindowsKeybindListener {
    thread_id: u32,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl KeybindListener for WindowsKeybindListener {
    fn start() -> Result<(Self, UnboundedReceiver<KeyEvent>), KeybindsError>
    where
        Self: Sized,
    {
        log::debug!("Starting windows keybind listener");
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
            }).map_err(|err| KeybindsError::Listener(format!("Failed to spawn thread: {err}")))?;

        match start_res_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(Ok(thread_id)) => Ok((
                Self {
                    thread_handle: Some(thread_handle),
                    thread_id,
                },
                key_event_rx,
            )),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(KeybindsError::Listener(
                "WindowsKeybindListener startup timed out".to_string(),
            )),
        }
    }
}

impl Drop for WindowsKeybindListener {
    fn drop(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            log::debug!("Stopping Windows keybind listener");
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

impl WindowsKeybindListener {
    fn setup_input_listener(tx: UnboundedSender<KeyEvent>) -> Result<HWND, KeybindsError> {
        let hmodule = unsafe {
            GetModuleHandleW(None).map_err(|_| {
                KeybindsError::Listener(format!("GetModuleHandleW failed: {:?}", GetLastError()))
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
                KeybindsError::Listener(format!("CreateWindowExW failed: {:?}", GetLastError()))
            })?
        };

        if hwnd.0.is_null() {
            return Err(KeybindsError::Listener(format!(
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
                KeybindsError::Listener(format!(
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
                return Err(KeybindsError::Listener(format!(
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
                            Self::with_key_event_tx(hwnd, |tx| {
                                if let Err(err) = tx.send(KeyEvent { code, label, state }) {
                                    log::error!("Failed to send keybinds event: {err}");
                                }
                            });
                        }
                        Err(KeybindsError::FakeMarker) => {
                            // ignore fake markers and don't emit them
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
