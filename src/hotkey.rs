use std::io;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetMessageW, PostThreadMessageW, MSG, WM_HOTKEY, WM_QUIT,
};

use crate::runtime::RuntimeState;

const HOTKEY_ID: i32 = 1;
const TOGGLE_DEBOUNCE_MS: u64 = 80;

pub struct HotkeyHandle {
    thread_id: u32,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl Drop for HotkeyHandle {
    fn drop(&mut self) {
        if self.thread_id != 0 {
            unsafe {
                let _ = PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
            }
        }
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

pub fn spawn_hotkey_thread(state: Arc<RuntimeState>) -> Option<HotkeyHandle> {
    let (mods, vk) = parse_hotkey(&state.config_snapshot().keybinds)?;
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);

    let join_handle = thread::spawn(move || {
        let thread_id = unsafe { GetCurrentThreadId() };
        let ok = unsafe { RegisterHotKey(std::ptr::null_mut(), HOTKEY_ID, mods, vk) };
        if ok == 0 {
            let _ = ready_tx.send(Err(io::Error::last_os_error()));
            return;
        }

        let _ = ready_tx.send(Ok(thread_id));

        let mut msg: MSG = unsafe { std::mem::zeroed() };
        loop {
            let result = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
            if result <= 0 {
                break;
            }
            if msg.message == WM_HOTKEY {
                state.toggle_active_debounced(TOGGLE_DEBOUNCE_MS);
            }
        }

        unsafe {
            let _ = UnregisterHotKey(std::ptr::null_mut(), HOTKEY_ID);
        }
    });

    match ready_rx.recv() {
        Ok(Ok(thread_id)) => Some(HotkeyHandle {
            thread_id,
            join_handle: Some(join_handle),
        }),
        Ok(Err(err)) => {
            eprintln!("[Vene] RegisterHotKey failed: {err}");
            None
        }
        Err(_) => None,
    }
}

fn parse_hotkey(tokens: &[String]) -> Option<(u32, u32)> {
    let mut modifiers = 0u32;
    let mut key_vk: Option<u32> = None;

    for token in tokens {
        match token.as_str() {
            "ControlLeft" | "ControlRight" => modifiers |= MOD_CONTROL,
            "ShiftLeft" | "ShiftRight" => modifiers |= MOD_SHIFT,
            "Alt" | "AltGr" => modifiers |= MOD_ALT,
            _ => {
                let vk = token_to_vk(token)?;
                if key_vk.is_some() {
                    return None;
                }
                key_vk = Some(vk);
            }
        }
    }

    let vk = key_vk?;
    modifiers |= MOD_NOREPEAT;
    Some((modifiers, vk))
}

fn token_to_vk(token: &str) -> Option<u32> {
    if let Some(rest) = token.strip_prefix("Key") {
        let ch = rest.chars().next()?;
        let upper = ch.to_ascii_uppercase();
        if ('A'..='Z').contains(&upper) {
            return Some(upper as u32);
        }
        if ('0'..='9').contains(&ch) {
            return Some(ch as u32);
        }
    }

    if let Some(rest) = token.strip_prefix("Num") {
        let ch = rest.chars().next()?;
        if ('0'..='9').contains(&ch) {
            return Some(ch as u32);
        }
    }

    if let Some(rest) = token.strip_prefix("F") {
        if let Ok(num) = rest.parse::<u32>() {
            if (1..=12).contains(&num) {
                return Some(0x70 + (num - 1));
            }
        }
    }

    match token {
        "Space" => Some(0x20),
        "Tab" => Some(0x09),
        "Backspace" => Some(0x08),
        "Return" => Some(0x0D),
        "Escape" => Some(0x1B),
        "CapsLock" => Some(0x14),
        "NumLock" => Some(0x90),
        "ScrollLock" => Some(0x91),
        "PrintScreen" => Some(0x2C),
        "Pause" => Some(0x13),
        "Menu" => Some(0x5D),
        "MetaLeft" => Some(0x5B),
        "MetaRight" => Some(0x5C),
        "Command" => Some(0x5B),
        "Super" => Some(0x5B),
        "Win" => Some(0x5B),
        "Insert" => Some(0x2D),
        "Delete" => Some(0x2E),
        "Home" => Some(0x24),
        "End" => Some(0x23),
        "PageUp" => Some(0x21),
        "PageDown" => Some(0x22),
        "UpArrow" => Some(0x26),
        "DownArrow" => Some(0x28),
        "LeftArrow" => Some(0x25),
        "RightArrow" => Some(0x27),
        "Minus" => Some(0xBD),
        "Equal" => Some(0xBB),
        "Comma" => Some(0xBC),
        "Period" => Some(0xBE),
        "Slash" => Some(0xBF),
        "BackSlash" => Some(0xDC),
        "Apostrophe" => Some(0xDE),
        "Quote" => Some(0xDE),
        "Semicolon" => Some(0xBA),
        "Grave" => Some(0xC0),
        "Backquote" => Some(0xC0),
        "BracketLeft" => Some(0xDB),
        "BracketRight" => Some(0xDD),
        _ => None,
    }
}
