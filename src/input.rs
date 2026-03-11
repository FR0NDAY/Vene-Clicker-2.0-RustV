use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use parking_lot::Mutex;
use rdev::{listen, Event, EventType};

use crate::runtime::RuntimeState;
const TOGGLE_DEBOUNCE_MS: u64 = 80;

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

    if state.hotkey_registered.load(Ordering::SeqCst) {
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
        state.toggle_active_debounced(TOGGLE_DEBOUNCE_MS);
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
