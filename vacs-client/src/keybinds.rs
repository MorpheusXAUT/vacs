use crate::app::state::AppState;
use crate::app::state::audio::AppStateAudioExt;
use crate::config::{TransmitConfig, TransmitMode};
use crate::error::Error;
use keyboard_types::Code;
use once_cell::sync::{Lazy, OnceCell};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::mem::zeroed;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use windows::Win32::Foundation::{GetLastError, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::{
    GetRawInputData, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RAWKEYBOARD, RID_INPUT,
    RIDEV_INPUTSINK, RIM_TYPEKEYBOARD, RegisterRawInputDevices,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, GWLP_USERDATA, GetMessageW, GetWindowLongPtrW, GetWindowThreadProcessId, MSG,
    PostQuitMessage, RI_KEY_E0, RegisterClassW, SetWindowLongPtrW, TranslateMessage, WM_DESTROY,
    WM_INPUT, WM_KEYDOWN, WM_KEYUP, WM_NCDESTROY, WM_SYSKEYDOWN, WM_SYSKEYUP, WNDCLASSW,
};
use windows::core::w;

pub mod commands;

pub trait KeybindsTrait {
    fn register_keybinds(&self, app: &AppHandle) -> Result<(), Error>;
    fn unregister_keybinds(&self, app: &AppHandle) -> Result<(), Error>;
}

impl KeybindsTrait for TransmitConfig {
    fn register_keybinds(&self, app: &AppHandle) -> Result<(), Error> {
        match self.mode {
            TransmitMode::PushToTalk => {
                self.push_to_talk
                    .map(|s| -> Result<(), ()> {
                        let (tx, mut rx) = unbounded_channel::<KeybindsEvent>();

                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let mut is_pressed = false;
                            while let Some(event) = rx.recv().await {
                                let code: Result<Code, Error> = event.vk_code.try_into();
                                if let Ok(code) = code
                                    && code == s
                                {
                                    match event.state {
                                        KeybindsState::Pressed => {
                                            if !is_pressed {
                                                is_pressed = true;
                                                let state = app.state::<AppState>();
                                                let state = state.lock().await;
                                                state.audio_manager().set_input_muted(false);
                                                println!("Unmuted");
                                            }
                                        }
                                        KeybindsState::Released => {
                                            is_pressed = false;
                                            let state = app.state::<AppState>();
                                            let state = state.lock().await;
                                            state.audio_manager().set_input_muted(true);
                                            println!("Muted");
                                        }
                                    }
                                }
                            }
                        });

                        thread::spawn(move || {
                            start_listener(tx);
                        });

                        Ok(())
                    })
                    .transpose()
                    .expect("Failed to register push to talk");
                Ok(())
            }
            TransmitMode::PushToMute => {
                self.push_to_mute
                    .map(|s| -> Result<(), ()> { Ok(()) })
                    .transpose()
                    .expect("Failed to register push to mute");
                Ok(())
            }
            TransmitMode::VoiceActivation => Ok(()),
        }
    }

    fn unregister_keybinds(&self, app: &AppHandle) -> Result<(), Error> {
        match self.mode {
            TransmitMode::PushToTalk => {
                // TODO
                Ok(())
            }
            TransmitMode::PushToMute => {
                // TODO
                Ok(())
            }
            TransmitMode::VoiceActivation => Ok(()),
        }
    }
}

extern "system" fn wnd_proc(hwnd: HWND, msg: u32, _wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_INPUT => unsafe {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut UnboundedSender<KeybindsEvent>;
            if !ptr.is_null() {
                // Process raw input here
                let hraw = HRAWINPUT(lparam.0 as _);

                let mut needed: u32 = 0;
                let header_size = size_of::<RAWINPUTHEADER>() as u32;

                let ret = GetRawInputData(hraw, RID_INPUT, None, &mut needed, header_size);
                if ret == 0 && needed > 0 {
                    // allocate buffer
                    let mut buffer: Vec<u8> = vec![0u8; needed as usize];

                    let read = GetRawInputData(
                        hraw,
                        RID_INPUT,
                        Some(buffer.as_mut_ptr() as *mut _),
                        &mut needed,
                        header_size,
                    );

                    if (read) == needed {
                        // cast buffer pointer to RAWINPUT
                        let raw = &*(buffer.as_ptr() as *const RAWINPUT);

                        if raw.header.dwType == RIM_TYPEKEYBOARD.0 {
                            // Access keyboard union
                            let kb = raw.data.keyboard;
                            let vk_code = precise_vkey(&kb);
                            let scancode = kb.MakeCode;
                            let flags = kb.Flags;
                            let message = kb.Message;

                            let state = match message {
                                WM_KEYDOWN | WM_SYSKEYDOWN => KeybindsState::Pressed,
                                WM_KEYUP | WM_SYSKEYUP => KeybindsState::Released,
                                _ => return LRESULT(0),
                            };

                            if let Err(err) = (&*ptr).send(KeybindsEvent {
                                vk_code: VirtualKeyCode(vk_code),
                                state,
                            }) {
                                log::error!("Failed to send keybinds event: {}", err);
                            }
                        }
                    } else {
                        log::warn!(
                            "GetRawInputData returned unexpected size (got {}, expected {})",
                            read,
                            needed
                        );
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
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut UnboundedSender<KeybindsEvent>;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            DefWindowProcW(hwnd, msg, _wparam, lparam)
        },
        _ => unsafe { DefWindowProcW(hwnd, msg, _wparam, lparam) },
    }
}

fn precise_vkey(kb: &RAWKEYBOARD) -> u16 {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;

    let is_e0 = (kb.Flags & RI_KEY_E0 as u16) != 0;

    match VIRTUAL_KEY(kb.VKey) {
        VK_SHIFT => match kb.MakeCode {
            0x2A => VK_LSHIFT,
            0x36 => VK_RSHIFT,
            _ => VK_SHIFT,
        },
        VK_CONTROL => {
            if is_e0 { VK_RCONTROL } else { VK_LCONTROL } // VK_RCONTROL / VK_LCONTROL
        }
        VK_MENU => {
            if is_e0 { VK_RMENU } else { VK_LMENU } // VK_RMENU / VK_LMENU
        }
        vk => vk, // other keys unchanged
    }
    .0
}

fn start_listener(tx: UnboundedSender<KeybindsEvent>) {
    unsafe {
        println!("Starting CreateWindowExW test...");

        let hmodule = GetModuleHandleW(None).unwrap();
        let hinstance = HINSTANCE(hmodule.0);
        println!("Module handle: {:?}", hmodule);

        let class_name = w!("RawInputHiddenWindowClass\0");

        let wnd_class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance,
            lpszClassName: class_name,
            ..zeroed()
        };

        let atom = RegisterClassW(&wnd_class);
        if atom == 0 {
            let err = GetLastError();
            panic!("RegisterClassW failed: {}", err.0);
        }
        println!("Window class registered (atom={} )", atom);

        let hwnd = CreateWindowExW(
            Default::default(),
            class_name,
            w!("Raw Input Listener\0"),
            Default::default(),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            300,
            200,
            None,
            None,
            Some(hinstance),
            None,
        )
        .unwrap();

        if hwnd.0 == std::ptr::null_mut() {
            let err = GetLastError();
            panic!("CreateWindowExW failed: {} ({:?})", err.0, err);
        }

        println!("CreateWindowExW succeeded! HWND = {:?}", hwnd);

        let tx = Box::new(tx);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(tx) as isize);

        let rid = RAWINPUTDEVICE {
            usUsagePage: 0x01, // Generic Desktop Controls
            usUsage: 0x06,     // Keyboard
            dwFlags: RIDEV_INPUTSINK,
            hwndTarget: hwnd,
        };

        if !RegisterRawInputDevices(&[rid], size_of::<RAWINPUTDEVICE>() as u32).is_ok() {
            let err = GetLastError();
            panic!("RegisterRawInputDevices failed: {}", err.0);
        }

        // Run a simple message loop so window stays open
        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeybindsEvent {
    pub vk_code: VirtualKeyCode,
    pub state: KeybindsState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeybindsState {
    Pressed,
    Released,
}

// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
// pub enum Key {
//     KeyA,
//     KeyB,
//     KeyC,
//     KeyD,
//     KeyE,
//     KeyF,
//     KeyG,
//     KeyH,
//     KeyI,
//     KeyJ,
//     KeyK,
//     KeyL,
//     KeyM,
//     KeyN,
//     KeyO,
//     KeyP,
//     KeyQ,
//     KeyR,
//     KeyS,
//     KeyT,
//     KeyU,
//     KeyV,
//     KeyW,
//     KeyX,
//     KeyY,
//     KeyZ,
//     Digit0,
//     Digit1,
//     Digit2,
//     Digit3,
//     Digit4,
//     Digit5,
//     Digit6,
//     Digit7,
//     Digit8,
//     Digit9,
//     ShiftLeft,
//     ShiftRight,
//     ControlLeft,
//     ControlRight,
//     AltLeft,
//     AltRight,
//     MetaLeft,
//     MetaRight,
//     F1,
//     F2,
//     F3,
//     F4,
//     F5,
//     F6,
//     F7,
//     F8,
//     F9,
//     F10,
//     F11,
//     F12,
//     Space,
//     Enter,
//     Escape,
//     Tab,
//     Backspace,
//     Delete,
//     ArrowUp,
//     ArrowDown,
//     ArrowLeft,
//     ArrowRight,
//     Numpad0,
//     Numpad1,
//     Numpad2,
//     Numpad3,
//     Numpad4,
//     Numpad5,
//     Numpad6,
//     Numpad7,
//     Numpad8,
//     Numpad9,
//     NumpadAdd,
//     NumpadSubtract,
//     NumpadMultiply,
//     NumpadDivide,
//     NumpadEnter,
//     NumpadDecimal,
//     CapsLock,
//     NumLock,
//     ScrollLock,
//     PrintScreen,
//     Pause,
//     Insert,
//     Home,
//     End,
//     PageUp,
//     PageDown,
//     ContextMenu,
//     Power,
//     Help,
// }
//
// impl Key {
//     pub fn as_str(&self) -> &'static str {
//         match self {
//             Key::KeyA => "KeyA",
//             Key::KeyB => "KeyB",
//             Key::KeyC => "KeyC",
//             Key::KeyD => "KeyD",
//             Key::KeyE => "KeyE",
//             Key::KeyF => "KeyF",
//             Key::KeyG => "KeyG",
//             Key::KeyH => "KeyH",
//             Key::KeyI => "KeyI",
//             Key::KeyJ => "KeyJ",
//             Key::KeyK => "KeyK",
//             Key::KeyL => "KeyL",
//             Key::KeyM => "KeyM",
//             Key::KeyN => "KeyN",
//             Key::KeyO => "KeyO",
//             Key::KeyP => "KeyP",
//             Key::KeyQ => "KeyQ",
//             Key::KeyR => "KeyR",
//             Key::KeyS => "KeyS",
//             Key::KeyT => "KeyT",
//             Key::KeyU => "KeyU",
//             Key::KeyV => "KeyV",
//             Key::KeyW => "KeyW",
//             Key::KeyX => "KeyX",
//             Key::KeyY => "KeyY",
//             Key::KeyZ => "KeyZ",
//             Key::Digit0 => "Digit0",
//             Key::Digit1 => "Digit1",
//             Key::Digit2 => "Digit2",
//             Key::Digit3 => "Digit3",
//             Key::Digit4 => "Digit4",
//             Key::Digit5 => "Digit5",
//             Key::Digit6 => "Digit6",
//             Key::Digit7 => "Digit7",
//             Key::Digit8 => "Digit8",
//             Key::Digit9 => "Digit9",
//             Key::ShiftLeft => "ShiftLeft",
//             Key::ShiftRight => "ShiftRight",
//             Key::ControlLeft => "ControlLeft",
//             Key::ControlRight => "ControlRight",
//             Key::AltLeft => "AltLeft",
//             Key::AltRight => "AltRight",
//             Key::MetaLeft => "MetaLeft",
//             Key::MetaRight => "MetaRight",
//             Key::F1 => "F1",
//             Key::F2 => "F2",
//             Key::F3 => "F3",
//             Key::F4 => "F4",
//             Key::F5 => "F5",
//             Key::F6 => "F6",
//             Key::F7 => "F7",
//             Key::F8 => "F8",
//             Key::F9 => "F9",
//             Key::F10 => "F10",
//             Key::F11 => "F11",
//             Key::F12 => "F12",
//             Key::Space => "Space",
//             Key::Enter => "Enter",
//             Key::Escape => "Escape",
//             Key::Tab => "Tab",
//             Key::Backspace => "Backspace",
//             Key::Delete => "Delete",
//             Key::ArrowUp => "ArrowUp",
//             Key::ArrowDown => "ArrowDown",
//             Key::ArrowLeft => "ArrowLeft",
//             Key::ArrowRight => "ArrowRight",
//             Key::Numpad0 => "Numpad0",
//             Key::Numpad1 => "Numpad1",
//             Key::Numpad2 => "Numpad2",
//             Key::Numpad3 => "Numpad3",
//             Key::Numpad4 => "Numpad4",
//             Key::Numpad5 => "Numpad5",
//             Key::Numpad6 => "Numpad6",
//             Key::Numpad7 => "Numpad7",
//             Key::Numpad8 => "Numpad8",
//             Key::Numpad9 => "Numpad9",
//             Key::NumpadAdd => "NumpadAdd",
//             Key::NumpadSubtract => "NumpadSubtract",
//             Key::NumpadMultiply => "NumpadMultiply",
//             Key::NumpadDivide => "NumpadDivide",
//             Key::NumpadEnter => "NumpadEnter",
//             Key::NumpadDecimal => "NumpadDecimal",
//             Key::CapsLock => "CapsLock",
//             Key::NumLock => "NumLock",
//             Key::ScrollLock => "ScrollLock",
//             Key::PrintScreen => "PrintScreen",
//             Key::Pause => "Pause",
//             Key::Insert => "Insert",
//             Key::Home => "Home",
//             Key::End => "End",
//             Key::PageUp => "PageUp",
//             Key::PageDown => "PageDown",
//             Key::ContextMenu => "ContextMenu",
//             Key::Power => "Power",
//             Key::Help => "Help",
//         }
//     }
// }
//
// impl TryFrom<String> for Key {
//     type Error = Error;
//
//     fn try_from(value: String) -> Result<Self, Self::Error> {
//         value.as_str().try_into()
//     }
// }
//
// impl TryFrom<&String> for Key {
//     type Error = Error;
//
//     fn try_from(value: &String) -> Result<Self, Self::Error> {
//         value.as_str().try_into()
//     }
// }
//
// impl TryFrom<&str> for Key {
//     type Error = Error;
//
//     fn try_from(value: &str) -> Result<Self, Self::Error> {
//         match value {
//             "KeyA" => Ok(Key::KeyA),
//             "KeyB" => Ok(Key::KeyB),
//             "KeyC" => Ok(Key::KeyC),
//             "KeyD" => Ok(Key::KeyD),
//             "KeyE" => Ok(Key::KeyE),
//             "KeyF" => Ok(Key::KeyF),
//             "KeyG" => Ok(Key::KeyG),
//             "KeyH" => Ok(Key::KeyH),
//             "KeyI" => Ok(Key::KeyI),
//             "KeyJ" => Ok(Key::KeyJ),
//             "KeyK" => Ok(Key::KeyK),
//             "KeyL" => Ok(Key::KeyL),
//             "KeyM" => Ok(Key::KeyM),
//             "KeyN" => Ok(Key::KeyN),
//             "KeyO" => Ok(Key::KeyO),
//             "KeyP" => Ok(Key::KeyP),
//             "KeyQ" => Ok(Key::KeyQ),
//             "KeyR" => Ok(Key::KeyR),
//             "KeyS" => Ok(Key::KeyS),
//             "KeyT" => Ok(Key::KeyT),
//             "KeyU" => Ok(Key::KeyU),
//             "KeyV" => Ok(Key::KeyV),
//             "KeyW" => Ok(Key::KeyW),
//             "KeyX" => Ok(Key::KeyX),
//             "KeyY" => Ok(Key::KeyY),
//             "KeyZ" => Ok(Key::KeyZ),
//             "Digit0" => Ok(Key::Digit0),
//             "Digit1" => Ok(Key::Digit1),
//             "Digit2" => Ok(Key::Digit2),
//             "Digit3" => Ok(Key::Digit3),
//             "Digit4" => Ok(Key::Digit4),
//             "Digit5" => Ok(Key::Digit5),
//             "Digit6" => Ok(Key::Digit6),
//             "Digit7" => Ok(Key::Digit7),
//             "Digit8" => Ok(Key::Digit8),
//             "Digit9" => Ok(Key::Digit9),
//             "ShiftLeft" => Ok(Key::ShiftLeft),
//             "ShiftRight" => Ok(Key::ShiftRight),
//             "ControlLeft" => Ok(Key::ControlLeft),
//             "ControlRight" => Ok(Key::ControlRight),
//             "AltLeft" => Ok(Key::AltLeft),
//             "AltRight" => Ok(Key::AltRight),
//             "MetaLeft" => Ok(Key::MetaLeft),
//             "MetaRight" => Ok(Key::MetaRight),
//             "F1" => Ok(Key::F1),
//             "F2" => Ok(Key::F2),
//             "F3" => Ok(Key::F3),
//             "F4" => Ok(Key::F4),
//             "F5" => Ok(Key::F5),
//             "F6" => Ok(Key::F6),
//             "F7" => Ok(Key::F7),
//             "F8" => Ok(Key::F8),
//             "F9" => Ok(Key::F9),
//             "F10" => Ok(Key::F10),
//             "F11" => Ok(Key::F11),
//             "F12" => Ok(Key::F12),
//             "Space" => Ok(Key::Space),
//             "Enter" => Ok(Key::Enter),
//             "Escape" => Ok(Key::Escape),
//             "Tab" => Ok(Key::Tab),
//             "Backspace" => Ok(Key::Backspace),
//             "Delete" => Ok(Key::Delete),
//             "ArrowUp" => Ok(Key::ArrowUp),
//             "ArrowDown" => Ok(Key::ArrowDown),
//             "ArrowLeft" => Ok(Key::ArrowLeft),
//             "ArrowRight" => Ok(Key::ArrowRight),
//             "Numpad0" => Ok(Key::Numpad0),
//             "Numpad1" => Ok(Key::Numpad1),
//             "Numpad2" => Ok(Key::Numpad2),
//             "Numpad3" => Ok(Key::Numpad3),
//             "Numpad4" => Ok(Key::Numpad4),
//             "Numpad5" => Ok(Key::Numpad5),
//             "Numpad6" => Ok(Key::Numpad6),
//             "Numpad7" => Ok(Key::Numpad7),
//             "Numpad8" => Ok(Key::Numpad8),
//             "Numpad9" => Ok(Key::Numpad9),
//             "NumpadAdd" => Ok(Key::NumpadAdd),
//             "NumpadSubtract" => Ok(Key::NumpadSubtract),
//             "NumpadMultiply" => Ok(Key::NumpadMultiply),
//             "NumpadDivide" => Ok(Key::NumpadDivide),
//             "NumpadEnter" => Ok(Key::NumpadEnter),
//             "NumpadDecimal" => Ok(Key::NumpadDecimal),
//             "CapsLock" => Ok(Key::CapsLock),
//             "NumLock" => Ok(Key::NumLock),
//             "ScrollLock" => Ok(Key::ScrollLock),
//             "PrintScreen" => Ok(Key::PrintScreen),
//             "Pause" => Ok(Key::Pause),
//             "Insert" => Ok(Key::Insert),
//             "Home" => Ok(Key::Home),
//             "End" => Ok(Key::End),
//             "PageUp" => Ok(Key::PageUp),
//             "PageDown" => Ok(Key::PageDown),
//             "ContextMenu" => Ok(Key::ContextMenu),
//             "Power" => Ok(Key::Power),
//             "Help" => Ok(Key::Help),
//             other => Err(Error::Other(Box::new(anyhow::anyhow!(
//                 "Unrecognized key code: {other}. Please report this error in our GitHub repository's issue tracker."
//             )))),
//         }
//     }
// }
//
// impl Display for Key {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         f.write_str(self.as_str())
//     }
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VirtualKeyCode(u16);

impl TryFrom<VirtualKeyCode> for Code {
    type Error = Error;

    fn try_from(value: VirtualKeyCode) -> Result<Self, Self::Error> {
        use Code::*;
        use windows::Win32::UI::Input::KeyboardAndMouse::*;
        match VIRTUAL_KEY(value.0) {
            // Letters (A-Z: 0x41-0x5A)
            VK_A => Ok(KeyA),
            VK_B => Ok(KeyB),
            VK_C => Ok(KeyC),
            VK_D => Ok(KeyD),
            VK_E => Ok(KeyE),
            VK_F => Ok(KeyF),
            VK_G => Ok(KeyG),
            VK_H => Ok(KeyH),
            VK_I => Ok(KeyI),
            VK_J => Ok(KeyJ),
            VK_K => Ok(KeyK),
            VK_L => Ok(KeyL),
            VK_M => Ok(KeyM),
            VK_N => Ok(KeyN),
            VK_O => Ok(KeyO),
            VK_P => Ok(KeyP),
            VK_Q => Ok(KeyQ),
            VK_R => Ok(KeyR),
            VK_S => Ok(KeyS),
            VK_T => Ok(KeyT),
            VK_U => Ok(KeyU),
            VK_V => Ok(KeyV),
            VK_W => Ok(KeyW),
            VK_X => Ok(KeyX),
            VK_Y => Ok(KeyY),
            VK_Z => Ok(KeyZ),

            // Digits (0-9: 0x30-0x39)
            VK_0 => Ok(Digit0),
            VK_1 => Ok(Digit1),
            VK_2 => Ok(Digit2),
            VK_3 => Ok(Digit3),
            VK_4 => Ok(Digit4),
            VK_5 => Ok(Digit5),
            VK_6 => Ok(Digit6),
            VK_7 => Ok(Digit7),
            VK_8 => Ok(Digit8),
            VK_9 => Ok(Digit9),

            // Modifiers
            VK_SHIFT => Ok(ShiftLeft),       // VK_SHIFT
            VK_LSHIFT => Ok(ShiftLeft),      // VK_LSHIFT
            VK_RSHIFT => Ok(ShiftRight),     // VK_RSHIFT
            VK_CONTROL => Ok(ControlLeft),   // VK_CONTROL
            VK_LCONTROL => Ok(ControlLeft),  // VK_LCONTROL
            VK_RCONTROL => Ok(ControlRight), // VK_RCONTROL
            VK_MENU => Ok(AltLeft),          // VK_MENU
            VK_LMENU => Ok(AltLeft),         // VK_LMENU
            VK_RMENU => Ok(AltRight),        // VK_RMENU
            VK_LWIN => Ok(MetaLeft),         // VK_LWIN
            VK_RWIN => Ok(MetaRight),        // VK_RWIN

            // Function keys (F1-F24: 0x70-0x87)
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

            // Special keys
            VK_SPACE => Ok(Space),      // VK_SPACE
            VK_RETURN => Ok(Enter),     // VK_RETURN
            VK_ESCAPE => Ok(Escape),    // VK_ESCAPE
            VK_TAB => Ok(Tab),          // VK_TAB
            VK_BACK => Ok(Backspace),   // VK_BACK
            VK_DELETE => Ok(Delete),    // VK_DELETE
            VK_LEFT => Ok(ArrowLeft),   // VK_LEFT
            VK_UP => Ok(ArrowUp),       // VK_UP
            VK_RIGHT => Ok(ArrowRight), // VK_RIGHT
            VK_DOWN => Ok(ArrowDown),   // VK_DOWN

            // Numpad
            VK_NUMPAD0 => Ok(Numpad0),
            VK_NUMPAD1 => Ok(Numpad1),
            VK_NUMPAD2 => Ok(Numpad2),
            VK_NUMPAD3 => Ok(Numpad3),
            VK_NUMPAD4 => Ok(Numpad4),
            VK_NUMPAD5 => Ok(Numpad5),
            VK_NUMPAD6 => Ok(Numpad6),
            VK_NUMPAD7 => Ok(Numpad7),
            VK_NUMPAD8 => Ok(Numpad8),
            VK_NUMPAD9 => Ok(Numpad9),
            VK_ADD => Ok(NumpadAdd),
            VK_SUBTRACT => Ok(NumpadSubtract),
            VK_MULTIPLY => Ok(NumpadMultiply),
            VK_DIVIDE => Ok(NumpadDivide),

            // Additional keys
            VK_CAPITAL => Ok(CapsLock),
            VK_NUMLOCK => Ok(NumLock),
            VK_SCROLL => Ok(ScrollLock),
            VK_PRINT => Ok(PrintScreen),
            VK_SNAPSHOT => Ok(PrintScreen),
            VK_PAUSE => Ok(Pause),
            VK_INSERT => Ok(Insert),
            VK_PRIOR => Ok(PageUp),
            VK_NEXT => Ok(PageDown),
            VK_END => Ok(End),
            VK_HOME => Ok(Home),
            VK_APPS => Ok(ContextMenu),

            other => Err(Error::Other(Box::new(anyhow::anyhow!(
                "Unrecognized virtual key code: {}. Please report this error in our GitHub repository's issue tracker.",
                other.0
            )))),
        }
    }
}
