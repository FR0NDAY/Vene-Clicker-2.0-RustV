use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use parking_lot::Mutex;
use rdev::{listen, Button, Event, EventType};

use crate::runtime::RuntimeState;
use crate::win;

const SYNTHETIC_EVENT_WINDOW_MS: u64 = 20;

pub fn spawn_input_listener(state: Arc<RuntimeState>) -> JoinHandle<()> {
    thread::spawn(move || {
        let pressed_keys = Arc::new(Mutex::new(HashSet::<String>::new()));
        let state_for_listener = state.clone();
        let pressed_for_listener = pressed_keys.clone();

        if let Err(err) = listen(move |event| {
            handle_event(&state_for_listener, &pressed_for_listener, event);
        }) {
            eprintln!("[Vene] Global input hook failed: {err:?}");
        }
    })
}

fn handle_event(state: &RuntimeState, pressed_keys: &Mutex<HashSet<String>>, event: Event) {
    match event.event_type {
        EventType::KeyPress(key) => on_key_press(state, pressed_keys, format!("{key:?}")),
        EventType::KeyRelease(key) => on_key_release(state, pressed_keys, format!("{key:?}")),
        EventType::ButtonPress(Button::Left) => on_left_press(state),
        EventType::ButtonRelease(Button::Left) => on_left_release(state),
        EventType::ButtonPress(Button::Right) => on_right_press(state),
        EventType::ButtonRelease(Button::Right) => on_right_release(state),
        _ => {}
    }
}

fn on_key_press(state: &RuntimeState, pressed_keys: &Mutex<HashSet<String>>, key: String) {
    if state.capture_mode.load(Ordering::SeqCst) {
        let mut captured = state.captured_keys.lock();
        if !captured.contains(&key) {
            captured.push(key);
        }
        return;
    }

    let keybinds = state.config_snapshot().keybinds;
    if keybinds.is_empty() {
        return;
    }

    let mut pressed = pressed_keys.lock();
    if pressed.len() > 20 {
        pressed.clear();
    }

    let already_pressed = pressed.contains(&key) && keybinds.contains(&key);
    pressed.insert(key.clone());

    if keybinds.iter().all(|k| pressed.contains(k)) && keybinds.contains(&key) && !already_pressed {
        state.toggle_active();
    }
}

fn on_key_release(state: &RuntimeState, pressed_keys: &Mutex<HashSet<String>>, key: String) {
    if state.capture_mode.load(Ordering::SeqCst) {
        let mut captured = state.captured_keys.lock();
        let mut unique = Vec::<String>::new();
        for token in captured.drain(..) {
            if !unique.contains(&token) {
                unique.push(token);
            }
        }
        drop(captured);

        if !unique.is_empty() {
            state.update_config(|cfg| {
                cfg.keybinds = unique;
            });
        }
        state.capture_mode.store(false, Ordering::SeqCst);
        return;
    }

    pressed_keys.lock().remove(&key);
}

fn on_left_press(state: &RuntimeState) {
    if is_synthetic_event(state.last_left_click_ms.load(Ordering::SeqCst)) {
        state.pending_left_releases.fetch_add(1, Ordering::SeqCst);
        return;
    }

    if state.is_active() {
        state.left_physical_down.store(true, Ordering::SeqCst);
    }
}

fn on_left_release(state: &RuntimeState) {
    let pending = state.pending_left_releases.load(Ordering::SeqCst);
    if pending > 0 {
        state.pending_left_releases.fetch_sub(1, Ordering::SeqCst);
        return;
    }
    state.left_physical_down.store(false, Ordering::SeqCst);
}

fn on_right_press(state: &RuntimeState) {
    if is_synthetic_event(state.last_right_click_ms.load(Ordering::SeqCst)) {
        state.pending_right_releases.fetch_add(1, Ordering::SeqCst);
        return;
    }

    if state.is_active() && state.config_snapshot().right_click_enabled {
        state.right_physical_down.store(true, Ordering::SeqCst);
    }
}

fn on_right_release(state: &RuntimeState) {
    let pending = state.pending_right_releases.load(Ordering::SeqCst);
    if pending > 0 {
        state.pending_right_releases.fetch_sub(1, Ordering::SeqCst);
        return;
    }
    state.right_physical_down.store(false, Ordering::SeqCst);
}

fn is_synthetic_event(last_click_ms: u64) -> bool {
    win::now_millis().saturating_sub(last_click_ms) <= SYNTHETIC_EVENT_WINDOW_MS
}
