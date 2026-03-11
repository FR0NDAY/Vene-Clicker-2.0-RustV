use std::fs;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::{egui, App};
use egui::{Color32, RichText};

use crate::config::{save_config, AppConfig};
use crate::hotkey;
use crate::keybind::keybind_display;
use crate::runtime::RuntimeState;
use crate::win;

pub struct VeneApp {
    state: Arc<RuntimeState>,
    config_path: PathBuf,
    timer_resolution_enabled: bool,
    save_notice_until: Option<Instant>,
    save_on_drop: bool,
}

impl VeneApp {
    pub fn new(
        state: Arc<RuntimeState>,
        config_path: PathBuf,
        timer_resolution_enabled: bool,
    ) -> Self {
        Self {
            state,
            config_path,
            timer_resolution_enabled,
            save_notice_until: None,
            save_on_drop: true,
        }
    }

    fn persist_config(&mut self) {
        let cfg = self.state.config_snapshot();
        if let Err(err) = save_config(&self.config_path, &cfg, self.state.is_active()) {
            eprintln!("[Vene] Failed to save config: {err}");
            return;
        }
        self.save_notice_until = Some(Instant::now() + Duration::from_secs(2));
    }
}

impl App for VeneApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(33));

        let mut cfg: AppConfig = self.state.config_snapshot();
        let mut changed = false;
        let is_active = self.state.is_active();
        let mut capture_mode = self.state.capture_mode.load(Ordering::SeqCst);

        if capture_mode {
            if let Some(new_keybind) = capture_keybind_from_ui(ctx) {
                self.state.update_config(|shared_cfg| {
                    shared_cfg.keybinds = new_keybind;
                });
                hotkey::request_hotkey_reload();
                self.state.capture_mode.store(false, Ordering::SeqCst);
                capture_mode = false;
                cfg = self.state.config_snapshot();
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading(RichText::new("VeneClicker").strong());
                let indicator = if is_active {
                    RichText::new("ACTIVE").color(Color32::LIGHT_GREEN).strong()
                } else {
                    RichText::new("INACTIVE").color(Color32::LIGHT_RED).strong()
                };
                ui.label(indicator);
            });

            ui.add_space(6.0);
            if capture_mode {
                ui.label(RichText::new("Press your key combo and release a key...").italics());
            }

            ui.horizontal(|ui| {
                let keybind_label = format!("Toggle Key: {}", keybind_display(&cfg.keybinds));
                if ui
                    .add_enabled(!capture_mode, egui::Button::new(keybind_label))
                    .clicked()
                {
                    self.state.begin_keybind_capture();
                }

                if ui.button("Save").clicked() {
                    self.persist_config();
                }
            });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            changed |= ui
                .checkbox(&mut cfg.only_in_minecraft, "Only in Minecraft window")
                .changed();
            changed |= ui
                .checkbox(&mut cfg.cps_drops_enabled, "Enable CPS drops")
                .changed();
            changed |= ui
                .checkbox(&mut cfg.right_click_enabled, "Enable right clicker")
                .changed();

            ui.add_space(10.0);
            ui.label(RichText::new("Left Click CPS").strong());
            changed |= ui
                .add(egui::Slider::new(&mut cfg.min_cps, 5..=25).text("Min"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(&mut cfg.max_cps, 5..=25).text("Max"))
                .changed();

            ui.add_space(6.0);
            ui.label(RichText::new("Right Click CPS").strong());
            changed |= ui
                .add(egui::Slider::new(&mut cfg.min_right_cps, 5..=25).text("Min"))
                .changed();
            changed |= ui
                .add(egui::Slider::new(&mut cfg.max_right_cps, 5..=25).text("Max"))
                .changed();

            cfg.sanitize();

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Reset Defaults").clicked() {
                    cfg = AppConfig::default();
                    changed = true;
                }

                if ui
                    .add(egui::Button::new(
                        RichText::new("Self-destruct").color(Color32::LIGHT_RED),
                    ))
                    .clicked()
                {
                    self.save_on_drop = false;
                    let _ = fs::remove_file(&self.config_path);
                    self.state.shutdown.store(true, Ordering::SeqCst);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            if let Some(until) = self.save_notice_until {
                if Instant::now() < until {
                    ui.label(
                        RichText::new("Configuration saved to config.txt")
                            .color(Color32::LIGHT_GREEN),
                    );
                }
            }
        });

        if changed {
            self.state.update_config(|shared_cfg| {
                *shared_cfg = cfg;
            });
        }
    }
}

impl Drop for VeneApp {
    fn drop(&mut self) {
        if self.save_on_drop {
            let cfg = self.state.config_snapshot();
            if let Err(err) = save_config(&self.config_path, &cfg, self.state.is_active()) {
                eprintln!("[Vene] Failed to save config on shutdown: {err}");
            }
        }
        self.state.shutdown.store(true, Ordering::SeqCst);
        self.state.notify_wakeup();
        win::disable_high_resolution_timer(self.timer_resolution_enabled);
    }
}

fn capture_keybind_from_ui(ctx: &egui::Context) -> Option<Vec<String>> {
    ctx.input(|input| {
        let mut tokens = Vec::<String>::new();

        if input.modifiers.ctrl {
            tokens.push("ControlLeft".to_owned());
        }
        if input.modifiers.shift {
            tokens.push("ShiftLeft".to_owned());
        }
        if input.modifiers.alt {
            tokens.push("Alt".to_owned());
        }

        for event in &input.events {
            if let egui::Event::Key { key, pressed, .. } = event {
                if *pressed {
                    if let Some(token) = egui_key_to_rdev_token(*key) {
                        let token = token.to_owned();
                        if !tokens.contains(&token) {
                            tokens.push(token);
                        }
                    }
                }
            }
        }

        if tokens.is_empty() {
            None
        } else {
            Some(tokens)
        }
    })
}

fn egui_key_to_rdev_token(key: egui::Key) -> Option<&'static str> {
    use egui::Key;

    match key {
        Key::A => Some("KeyA"),
        Key::B => Some("KeyB"),
        Key::C => Some("KeyC"),
        Key::D => Some("KeyD"),
        Key::E => Some("KeyE"),
        Key::F => Some("KeyF"),
        Key::G => Some("KeyG"),
        Key::H => Some("KeyH"),
        Key::I => Some("KeyI"),
        Key::J => Some("KeyJ"),
        Key::K => Some("KeyK"),
        Key::L => Some("KeyL"),
        Key::M => Some("KeyM"),
        Key::N => Some("KeyN"),
        Key::O => Some("KeyO"),
        Key::P => Some("KeyP"),
        Key::Q => Some("KeyQ"),
        Key::R => Some("KeyR"),
        Key::S => Some("KeyS"),
        Key::T => Some("KeyT"),
        Key::U => Some("KeyU"),
        Key::V => Some("KeyV"),
        Key::W => Some("KeyW"),
        Key::X => Some("KeyX"),
        Key::Y => Some("KeyY"),
        Key::Z => Some("KeyZ"),
        Key::Num0 => Some("Num0"),
        Key::Num1 => Some("Num1"),
        Key::Num2 => Some("Num2"),
        Key::Num3 => Some("Num3"),
        Key::Num4 => Some("Num4"),
        Key::Num5 => Some("Num5"),
        Key::Num6 => Some("Num6"),
        Key::Num7 => Some("Num7"),
        Key::Num8 => Some("Num8"),
        Key::Num9 => Some("Num9"),
        Key::ArrowDown => Some("DownArrow"),
        Key::ArrowLeft => Some("LeftArrow"),
        Key::ArrowRight => Some("RightArrow"),
        Key::ArrowUp => Some("UpArrow"),
        Key::Escape => Some("Escape"),
        Key::Tab => Some("Tab"),
        Key::Backspace => Some("Backspace"),
        Key::Enter => Some("Return"),
        Key::Space => Some("Space"),
        Key::Insert => Some("Insert"),
        Key::Delete => Some("Delete"),
        Key::Home => Some("Home"),
        Key::End => Some("End"),
        Key::PageUp => Some("PageUp"),
        Key::PageDown => Some("PageDown"),
        Key::F1 => Some("F1"),
        Key::F2 => Some("F2"),
        Key::F3 => Some("F3"),
        Key::F4 => Some("F4"),
        Key::F5 => Some("F5"),
        Key::F6 => Some("F6"),
        Key::F7 => Some("F7"),
        Key::F8 => Some("F8"),
        Key::F9 => Some("F9"),
        Key::F10 => Some("F10"),
        Key::F11 => Some("F11"),
        Key::F12 => Some("F12"),
        _ => None,
    }
}
