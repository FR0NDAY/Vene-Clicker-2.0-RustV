use std::io;
use std::sync::mpsc;
use std::sync::{Arc, OnceLock};
use std::thread;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetMessageW, PostThreadMessageW, EVENT_SYSTEM_FOREGROUND, MSG, WM_QUIT, WINEVENT_OUTOFCONTEXT,
    WINEVENT_SKIPOWNPROCESS,
};

use crate::runtime::RuntimeState;
use crate::win;

static FOREGROUND_STATE: OnceLock<Arc<RuntimeState>> = OnceLock::new();

pub struct ForegroundHookHandle {
    thread_id: u32,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl Drop for ForegroundHookHandle {
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

pub fn spawn_foreground_hook_thread(state: Arc<RuntimeState>) -> Option<ForegroundHookHandle> {
    let _ = FOREGROUND_STATE.set(state);
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);

    let join_handle = thread::spawn(move || {
        let thread_id = unsafe { GetCurrentThreadId() };
        let hook = unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                std::ptr::null_mut(),
                Some(foreground_event_proc),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        };
        if hook.is_null() {
            let _ = ready_tx.send(Err(io::Error::last_os_error()));
            return;
        }

        update_foreground_state();
        let _ = ready_tx.send(Ok(thread_id));

        let mut msg: MSG = unsafe { std::mem::zeroed() };
        loop {
            let result = unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) };
            if result <= 0 {
                break;
            }
        }

        unsafe {
            let _ = UnhookWinEvent(hook);
        }
    });

    match ready_rx.recv() {
        Ok(Ok(thread_id)) => Some(ForegroundHookHandle {
            thread_id,
            join_handle: Some(join_handle),
        }),
        Ok(Err(err)) => {
            eprintln!("[Vene] Foreground hook failed: {err}");
            None
        }
        Err(_) => None,
    }
}

fn update_foreground_state() {
    if let Some(state) = FOREGROUND_STATE.get() {
        let is_minecraft = win::is_minecraft_foreground();
        state.set_minecraft_foreground(is_minecraft);
    }
}

unsafe extern "system" fn foreground_event_proc(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    update_foreground_state();
}
