use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Condvar, Mutex as StdMutex};
use std::time::Duration;

use parking_lot::Mutex;

use crate::config::AppConfig;
use crate::win;

#[derive(Clone, Copy, Debug)]
pub struct ClickerConfigSnapshot {
    pub min_cps: u32,
    pub max_cps: u32,
    pub min_right_cps: u32,
    pub max_right_cps: u32,
    pub right_click_enabled: bool,
    pub cps_drops_enabled: bool,
    pub only_in_minecraft: bool,
}

pub struct RuntimeState {
    pub config: Mutex<AppConfig>,
    pub active: AtomicBool,
    pub shutdown: AtomicBool,
    pub left_physical_down: AtomicBool,
    pub right_physical_down: AtomicBool,
    wake_seq: AtomicU64,
    wake_lock: StdMutex<()>,
    wake_cv: Condvar,
    pub capture_mode: AtomicBool,
    pub captured_keys: Mutex<Vec<String>>,
    pub last_toggle_ms: AtomicU64,
    pub hotkey_registered: AtomicBool,
    pub mouse_hook_registered: AtomicBool,
}

impl RuntimeState {
    pub fn new(mut config: AppConfig) -> Self {
        config.sanitize();
        Self {
            config: Mutex::new(config),
            active: AtomicBool::new(false),
            shutdown: AtomicBool::new(false),
            left_physical_down: AtomicBool::new(false),
            right_physical_down: AtomicBool::new(false),
            wake_seq: AtomicU64::new(0),
            wake_lock: StdMutex::new(()),
            wake_cv: Condvar::new(),
            capture_mode: AtomicBool::new(false),
            captured_keys: Mutex::new(Vec::new()),
            last_toggle_ms: AtomicU64::new(0),
            hotkey_registered: AtomicBool::new(false),
            mouse_hook_registered: AtomicBool::new(false),
        }
    }

    pub fn config_snapshot(&self) -> AppConfig {
        self.config.lock().clone()
    }

    pub fn clicker_config_snapshot(&self) -> ClickerConfigSnapshot {
        let cfg = self.config.lock();
        ClickerConfigSnapshot {
            min_cps: cfg.min_cps,
            max_cps: cfg.max_cps,
            min_right_cps: cfg.min_right_cps,
            max_right_cps: cfg.max_right_cps,
            right_click_enabled: cfg.right_click_enabled,
            cps_drops_enabled: cfg.cps_drops_enabled,
            only_in_minecraft: cfg.only_in_minecraft,
        }
    }

    pub fn update_config<F>(&self, update: F)
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut cfg = self.config.lock();
        update(&mut cfg);
        cfg.sanitize();
        drop(cfg);
        self.notify_wakeup();
    }

    pub fn begin_keybind_capture(&self) {
        self.capture_mode.store(true, Ordering::SeqCst);
        self.captured_keys.lock().clear();
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    pub fn toggle_active(&self) -> bool {
        let next = !self.active.load(Ordering::SeqCst);
        self.active.store(next, Ordering::SeqCst);
        println!("[Vene] Clicker Active: {next}");
        self.notify_wakeup();
        next
    }

    pub fn toggle_active_debounced(&self, min_gap_ms: u64) -> bool {
        let now = win::now_millis();
        let mut last = self.last_toggle_ms.load(Ordering::SeqCst);
        loop {
            if now.saturating_sub(last) < min_gap_ms {
                return self.active.load(Ordering::SeqCst);
            }
            match self.last_toggle_ms.compare_exchange(
                last,
                now,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return self.toggle_active(),
                Err(updated) => last = updated,
            }
        }
    }

    pub fn notify_wakeup(&self) {
        self.wake_seq.fetch_add(1, Ordering::SeqCst);
        self.wake_cv.notify_all();
    }

    pub fn wake_seq(&self) -> u64 {
        self.wake_seq.load(Ordering::SeqCst)
    }

    pub fn wait_for_wakeup(&self, seq: u64, timeout: Duration) {
        let guard = self.wake_lock.lock().unwrap();
        let _ = self
            .wake_cv
            .wait_timeout_while(guard, timeout, |_| {
                self.wake_seq.load(Ordering::SeqCst) == seq
            });
    }

    // Intentionally no benchmark-only helpers.
}
