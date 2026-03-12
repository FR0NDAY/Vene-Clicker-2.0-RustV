use windows_sys::Win32::System::SystemInformation::GetTickCount64;
use windows_sys::Win32::System::Threading::{
    GetCurrentThread, OpenProcess, QueryFullProcessImageNameW, SetThreadPriority,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, THREAD_PRIORITY_HIGHEST,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    mouse_event, GetAsyncKeyState, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, VK_LBUTTON, VK_RBUTTON,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId,
};
use windows_sys::Win32::Foundation::CloseHandle;

pub fn now_millis() -> u64 {
    unsafe { GetTickCount64() }
}

pub fn raise_clicker_thread_priority() {
    unsafe {
        let handle = GetCurrentThread();
        let ok = SetThreadPriority(handle, THREAD_PRIORITY_HIGHEST);
        if ok == 0 {
            eprintln!("[Vene] Failed to raise clicker thread priority.");
        }
    }
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

pub fn is_minecraft_foreground() -> bool {
    match foreground_process_name() {
        Some(name) => is_minecraft_process_name(&name),
        None => false,
    }
}

pub fn is_minecraft_process_name(name: &str) -> bool {
    let name = name.trim().to_ascii_lowercase();
    name == "javaw.exe" || name == "minecraft.windows.exe"
}

pub fn foreground_process_name() -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }

        let mut pid: u32 = 0;
        let _ = GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return None;
        }

        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }

        let mut buffer = vec![0u16; 1024];
        let mut size = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            buffer.as_mut_ptr(),
            &mut size,
        );
        let _ = CloseHandle(handle);

        if ok == 0 || size == 0 {
            return None;
        }

        let full = String::from_utf16_lossy(&buffer[..size as usize]);
        let name = full
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or(full.as_str())
            .to_string();
        Some(name)
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

pub fn is_left_button_down() -> bool {
    unsafe { (GetAsyncKeyState(VK_LBUTTON as i32) & (0x8000u16 as i16)) != 0 }
}

pub fn is_right_button_down() -> bool {
    unsafe { (GetAsyncKeyState(VK_RBUTTON as i32) & (0x8000u16 as i16)) != 0 }
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
