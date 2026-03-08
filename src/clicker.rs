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
    let debug = std::env::var_os("VENE_DEBUG").is_some();
    let mut rng = rand::thread_rng();
    let mut fatigue_ticks = 0_i32;
    let mut fatigue_intensity = 1.0_f64;
    let mut total_latency_ms = 0.0_f64;
    let mut click_count = 0_usize;

    loop {
        if state.shutdown.load(Ordering::SeqCst) {
            break;
        }

        let cfg = state.config_snapshot();
        let active = state.active.load(Ordering::SeqCst);
        let pressed = match button {
            MouseButton::Left => state.left_physical_down.load(Ordering::SeqCst),
            MouseButton::Right => state.right_physical_down.load(Ordering::SeqCst),
        };
        let running = match button {
            MouseButton::Left => active && pressed,
            MouseButton::Right => active && pressed && cfg.right_click_enabled,
        };

        if !running {
            fatigue_ticks = 0;
            precise_sleep(Duration::from_millis(10), &state);
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

        let spread = (max_cps - min_cps) as f64;
        let target_cps = (min_cps as f64) + rng.gen_range(0.0..=spread);
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

        let press_start = Instant::now();
        match button {
            MouseButton::Left => {
                state
                    .last_left_click_ms
                    .store(win::now_millis(), Ordering::SeqCst);
                win::left_press();
            }
            MouseButton::Right => {
                state
                    .last_right_click_ms
                    .store(win::now_millis(), Ordering::SeqCst);
                win::right_press();
            }
        }
        total_latency_ms += press_start.elapsed().as_secs_f64() * 1000.0;
        click_count += 1;
        if click_count >= 40 {
            if debug {
                println!(
                    "[Vene] {:?} Avg Latency: {:.4} ms | Target CPS: {:.1}{}",
                    button,
                    total_latency_ms / click_count as f64,
                    target_cps,
                    if fatigue_ticks > 0 { " (Fatigued)" } else { "" }
                );
            }
            total_latency_ms = 0.0;
            click_count = 0;
        }

        if cfg.jitter_intensity > 0 {
            apply_jitter(cfg.jitter_intensity, &mut rng);
        }

        let hold_fraction = 0.15 + rng.gen_range(0.0..=0.15);
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

fn apply_jitter(intensity: u32, rng: &mut rand::rngs::ThreadRng) {
    let range = (intensity / 20 + 1) as i32;
    let dx = rng.gen_range(-range..=range);
    let dy = rng.gen_range(-range..=range);
    if dx != 0 || dy != 0 {
        win::move_relative(dx, dy);
    }
}

fn precise_sleep(duration: Duration, state: &RuntimeState) {
    if duration.is_zero() {
        return;
    }
    let deadline = Instant::now() + duration;
    loop {
        if state.shutdown.load(Ordering::SeqCst) {
            return;
        }

        let now = Instant::now();
        if now >= deadline {
            return;
        }

        let remaining = deadline - now;
        if remaining > Duration::from_millis(2) {
            thread::sleep(Duration::from_millis(1));
            continue;
        }

        if remaining > Duration::from_micros(100) {
            thread::sleep(Duration::from_micros(100));
            continue;
        }

        std::hint::spin_loop();
    }
}
