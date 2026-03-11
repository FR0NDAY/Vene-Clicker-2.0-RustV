use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

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

    // Intentionally no benchmark-only helpers.
}
