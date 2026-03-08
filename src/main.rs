#[cfg(target_os = "windows")]
mod clicker;
#[cfg(target_os = "windows")]
mod config;
#[cfg(target_os = "windows")]
mod input;
#[cfg(target_os = "windows")]
mod keybind;
#[cfg(target_os = "windows")]
mod runtime;
#[cfg(target_os = "windows")]
mod ui;
#[cfg(target_os = "windows")]
mod win;

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("VeneClicker Rust build currently supports Windows only.");
}

#[cfg(target_os = "windows")]
fn main() -> eframe::Result<()> {
    use std::path::PathBuf;
    use std::sync::Arc;

    let config_path = PathBuf::from("config.txt");
    let config = config::load_config(&config_path);
    let state = Arc::new(runtime::RuntimeState::new(config));

    let _left_worker = clicker::spawn_click_worker(state.clone(), clicker::MouseButton::Left);
    let _right_worker = clicker::spawn_click_worker(state.clone(), clicker::MouseButton::Right);
    let _input_listener = input::spawn_input_listener(state.clone());

    let timer_resolution_enabled = win::enable_high_resolution_timer();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("VeneClicker")
            .with_inner_size([340.0, 440.0])
            .with_resizable(false),
        ..Default::default()
    };

    let app = ui::VeneApp::new(state, config_path, timer_resolution_enabled);
    eframe::run_native(
        "VeneClicker",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
}
