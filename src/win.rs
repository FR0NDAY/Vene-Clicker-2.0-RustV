use std::time::{SystemTime, UNIX_EPOCH};

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    mouse_event, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW,
};

pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn enable_high_resolution_timer() -> bool {
    unsafe { time_begin_period(1) == 0 }
}

pub fn disable_high_resolution_timer(enabled: bool) {
    if enabled {
        unsafe {
            let _ = time_end_period(1);
        }
    }
}

pub fn active_window_title() -> String {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return String::new();
        }

        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }

        let mut buffer = vec![0u16; (len + 1) as usize];
        let copied = GetWindowTextW(hwnd, buffer.as_mut_ptr(), len + 1);
        if copied <= 0 {
            return String::new();
        }

        String::from_utf16_lossy(&buffer[..copied as usize])
    }
}

pub fn left_press() {
    unsafe {
        mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0);
    }
}

pub fn left_release() {
    unsafe {
        mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0);
    }
}

pub fn right_press() {
    unsafe {
        mouse_event(MOUSEEVENTF_RIGHTDOWN, 0, 0, 0, 0);
    }
}

pub fn right_release() {
    unsafe {
        mouse_event(MOUSEEVENTF_RIGHTUP, 0, 0, 0, 0);
    }
}

pub fn move_relative(dx: i32, dy: i32) {
    unsafe {
        mouse_event(MOUSEEVENTF_MOVE, dx, dy, 0, 0);
    }
}

#[link(name = "winmm")]
extern "system" {
    fn timeBeginPeriod(uperiod: u32) -> u32;
    fn timeEndPeriod(uperiod: u32) -> u32;
}

unsafe fn time_begin_period(period: u32) -> u32 {
    timeBeginPeriod(period)
}

unsafe fn time_end_period(period: u32) -> u32 {
    timeEndPeriod(period)
}
