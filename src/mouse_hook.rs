use std::io;
use std::sync::mpsc;
use std::sync::atomic::Ordering;
use std::sync::{Arc, OnceLock};
use std::thread;

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx,
    LLMHF_INJECTED, MSLLHOOKSTRUCT, MSG, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_QUIT, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

use crate::runtime::RuntimeState;

static MOUSE_STATE: OnceLock<Arc<RuntimeState>> = OnceLock::new();

pub struct MouseHookHandle {
    thread_id: u32,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl Drop for MouseHookHandle {
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

pub fn spawn_mouse_hook_thread(state: Arc<RuntimeState>) -> Option<MouseHookHandle> {
    let _ = MOUSE_STATE.set(state);
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);

    let join_handle = thread::spawn(move || {
        let thread_id = unsafe { GetCurrentThreadId() };
        let hook = unsafe {
            SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), std::ptr::null_mut(), 0)
        };
        if hook.is_null() {
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
        }

        unsafe {
            let _ = unhook(hook);
        }
    });

    match ready_rx.recv() {
        Ok(Ok(thread_id)) => Some(MouseHookHandle {
            thread_id,
            join_handle: Some(join_handle),
        }),
        Ok(Err(err)) => {
            eprintln!("[Vene] Mouse hook failed: {err}");
            None
        }
        Err(_) => None,
    }
}

unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let mouse = *(lparam as *const MSLLHOOKSTRUCT);
        if (mouse.flags & LLMHF_INJECTED) == 0 {
            if let Some(state) = MOUSE_STATE.get() {
                match wparam as u32 {
                    WM_LBUTTONDOWN => {
                        state.left_physical_down.store(true, Ordering::SeqCst);
                    }
                    WM_LBUTTONUP => {
                        state.left_physical_down.store(false, Ordering::SeqCst);
                    }
                    WM_RBUTTONDOWN => {
                        state.right_physical_down.store(true, Ordering::SeqCst);
                    }
                    WM_RBUTTONUP => {
                        state.right_physical_down.store(false, Ordering::SeqCst);
                    }
                    _ => {}
                }
            }
        }
    }

    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

unsafe fn unhook(hook: *mut std::ffi::c_void) -> i32 {
    UnhookWindowsHookEx(hook)
}
