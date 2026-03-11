use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rand::Rng;

use crate::runtime::RuntimeState;
use crate::win;

#[derive(Clone, Copy, Debug)]
pub enum MouseButton {
    Left,
    Right,
}

pub fn spawn_click_worker(state: Arc<RuntimeState>, button: MouseButton) -> JoinHandle<()> {
    thread::spawn(move || run_worker(state, button))
}

fn run_worker(state: Arc<RuntimeState>, button: MouseButton) {
    let mut rng = rand::thread_rng();
    let mut fatigue_ticks = 0_i32;
    let mut fatigue_intensity = 1.0_f64;

    loop {
        if state.shutdown.load(Ordering::SeqCst) {
            break;
        }

        let cfg = state.clicker_config_snapshot();
        let active = state.active.load(Ordering::SeqCst);
        let pressed = if state.mouse_hook_registered.load(Ordering::SeqCst) {
            match button {
                MouseButton::Left => state.left_physical_down.load(Ordering::SeqCst),
                MouseButton::Right => state.right_physical_down.load(Ordering::SeqCst),
            }
        } else {
            match button {
                MouseButton::Left => win::is_left_button_down(),
                MouseButton::Right => win::is_right_button_down(),
            }
        };
        let running = match button {
            MouseButton::Left => active && pressed,
            MouseButton::Right => active && pressed && cfg.right_click_enabled,
        };

        if !running {
            fatigue_ticks = 0;
            let seq = state.wake_seq();
            state.wait_for_wakeup(seq, Duration::from_millis(100));
            continue;
        }

        let (mut min_cps, mut max_cps) = match button {
            MouseButton::Left => (cfg.min_cps, cfg.max_cps),
            MouseButton::Right => (cfg.min_right_cps, cfg.max_right_cps),
        };
        if min_cps > max_cps {
            std::mem::swap(&mut min_cps, &mut max_cps);
        }
        min_cps = min_cps.max(1);
        max_cps = max_cps.max(1);

        if cfg.cps_drops_enabled {
            if fatigue_ticks <= 0 {
                if rng.gen::<f64>() > 0.97 {
                    fatigue_ticks = rng.gen_range(3..=10);
                    fatigue_intensity = 0.6 + rng.gen_range(0.0..=0.25);
                } else {
                    fatigue_intensity = 1.0;
                }
            }

            if fatigue_ticks > 0 {
                min_cps = ((min_cps as f64) * fatigue_intensity).round() as u32;
                max_cps = ((max_cps as f64) * fatigue_intensity).round() as u32;
                min_cps = min_cps.max(1);
                max_cps = max_cps.max(1);
                if min_cps > max_cps {
                    std::mem::swap(&mut min_cps, &mut max_cps);
                }
                fatigue_ticks -= 1;
            }
        }

        let (target_cps, hold_fraction) = if !cfg.cps_drops_enabled && min_cps == max_cps {
            (min_cps as f64, 0.225)
        } else {
            let spread = (max_cps - min_cps) as f64;
            (
                (min_cps as f64) + rng.gen_range(0.0..=spread),
                0.15 + rng.gen_range(0.0..=0.15),
            )
        };
        let interval = Duration::from_secs_f64(1.0 / target_cps.max(1.0));
        let loop_start = Instant::now();

        if cfg.only_in_minecraft {
            let title = win::active_window_title().to_lowercase();
            let is_game_window = title.contains("minecraft")
                || title.contains("javaw")
                || title.contains("lunar")
                || title.contains("badlion")
                || title.contains("feather")
                || title.contains("cheatbreaker");
            if !is_game_window {
                precise_sleep(Duration::from_millis(50), &state);
                continue;
            }
        }

        match button {
            MouseButton::Left => {
                win::left_press();
            }
            MouseButton::Right => {
                win::right_press();
            }
        }
        precise_sleep(interval.mul_f64(hold_fraction), &state);

        match button {
            MouseButton::Left => win::left_release(),
            MouseButton::Right => win::right_release(),
        }

        let elapsed = loop_start.elapsed();
        if interval > elapsed {
            precise_sleep(interval - elapsed, &state);
        }
    }
}

fn precise_sleep(duration: Duration, state: &RuntimeState) {
    if duration.is_zero() {
        return;
    }
    let deadline = Instant::now() + duration;
    let spin_guard = Duration::from_micros(200);
    loop {
        if state.shutdown.load(Ordering::SeqCst) {
            return;
        }

        let now = Instant::now();
        if now >= deadline {
            return;
        }

        let remaining = deadline - now;
        if remaining > Duration::from_millis(3) {
            // Sleep most of the remaining interval, then finish with spin/yield.
            thread::sleep(remaining - Duration::from_millis(1));
            continue;
        }

        if remaining > spin_guard {
            thread::yield_now();
            continue;
        }

        while Instant::now() < deadline {
            if state.shutdown.load(Ordering::SeqCst) {
                return;
            }
            std::hint::spin_loop();
        }
        return;
    }
}
