use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};

use parking_lot::Mutex;

use crate::config::AppConfig;

pub struct RuntimeState {
    pub config: Mutex<AppConfig>,
    pub active: AtomicBool,
    pub shutdown: AtomicBool,
    pub left_physical_down: AtomicBool,
    pub right_physical_down: AtomicBool,
    pub last_left_click_ms: AtomicU64,
    pub last_right_click_ms: AtomicU64,
    pub pending_left_releases: AtomicI32,
    pub pending_right_releases: AtomicI32,
    pub capture_mode: AtomicBool,
    pub captured_keys: Mutex<Vec<String>>,
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
            last_left_click_ms: AtomicU64::new(0),
            last_right_click_ms: AtomicU64::new(0),
            pending_left_releases: AtomicI32::new(0),
            pending_right_releases: AtomicI32::new(0),
            capture_mode: AtomicBool::new(false),
            captured_keys: Mutex::new(Vec::new()),
        }
    }

    pub fn config_snapshot(&self) -> AppConfig {
        self.config.lock().clone()
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
}
