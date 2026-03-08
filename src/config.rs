use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use crate::keybind::{normalize_token, DEFAULT_KEYBIND};

const MIN_CPS: u32 = 5;
const MAX_CPS: u32 = 25;
const MIN_JITTER: u32 = 0;
const MAX_JITTER: u32 = 100;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub min_cps: u32,
    pub max_cps: u32,
    pub min_right_cps: u32,
    pub max_right_cps: u32,
    pub right_click_enabled: bool,
    pub cps_drops_enabled: bool,
    pub only_in_minecraft: bool,
    pub jitter_intensity: u32,
    pub keybinds: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            min_cps: 9,
            max_cps: 12,
            min_right_cps: 10,
            max_right_cps: 14,
            right_click_enabled: false,
            cps_drops_enabled: true,
            only_in_minecraft: false,
            jitter_intensity: 0,
            keybinds: vec![DEFAULT_KEYBIND.to_owned()],
        }
    }
}

impl AppConfig {
    pub fn sanitize(&mut self) {
        self.min_cps = self.min_cps.clamp(MIN_CPS, MAX_CPS);
        self.max_cps = self.max_cps.clamp(MIN_CPS, MAX_CPS);
        if self.min_cps > self.max_cps {
            std::mem::swap(&mut self.min_cps, &mut self.max_cps);
        }

        self.min_right_cps = self.min_right_cps.clamp(MIN_CPS, MAX_CPS);
        self.max_right_cps = self.max_right_cps.clamp(MIN_CPS, MAX_CPS);
        if self.min_right_cps > self.max_right_cps {
            std::mem::swap(&mut self.min_right_cps, &mut self.max_right_cps);
        }

        self.jitter_intensity = self.jitter_intensity.clamp(MIN_JITTER, MAX_JITTER);

        if self.keybinds.is_empty() {
            self.keybinds.push(DEFAULT_KEYBIND.to_owned());
        }
    }
}

pub fn load_config(path: &Path) -> AppConfig {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return AppConfig::default(),
    };

    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_owned(), value.trim().to_owned());
        }
    }

    let mut cfg = AppConfig {
        min_cps: parse_u32(map.get("minCps"), 9),
        max_cps: parse_u32(map.get("maxCps"), 12),
        min_right_cps: parse_u32(map.get("minRightCps"), 10),
        max_right_cps: parse_u32(map.get("maxRightCps"), 14),
        right_click_enabled: parse_bool(map.get("rightClickEnabled"), false),
        cps_drops_enabled: parse_bool(map.get("cpsDropsEnabled"), true),
        only_in_minecraft: parse_bool(map.get("onlyInMinecraft"), false),
        jitter_intensity: parse_u32(map.get("jitterIntensity"), 0),
        keybinds: parse_keybinds(map.get("keybinds").map(String::as_str).unwrap_or("")),
    };
    cfg.sanitize();
    cfg
}

pub fn save_config(path: &Path, cfg: &AppConfig, active: bool) -> io::Result<()> {
    let mut cfg = cfg.clone();
    cfg.sanitize();

    let keybinds = cfg.keybinds.join(":");
    let mut output = String::new();
    output.push_str("# Vene Clicker Configuration\n");
    output.push_str(&format!("keybinds={keybinds}\n"));
    output.push_str(&format!("minCps={}\n", cfg.min_cps));
    output.push_str(&format!("maxCps={}\n", cfg.max_cps));
    output.push_str(&format!("minRightCps={}\n", cfg.min_right_cps));
    output.push_str(&format!("maxRightCps={}\n", cfg.max_right_cps));
    output.push_str(&format!("rightClickEnabled={}\n", cfg.right_click_enabled));
    output.push_str(&format!("cpsDropsEnabled={}\n", cfg.cps_drops_enabled));
    output.push_str(&format!("onlyInMinecraft={}\n", cfg.only_in_minecraft));
    output.push_str(&format!("jitterIntensity={}\n", cfg.jitter_intensity));
    output.push_str(&format!("enabled={active}\n"));

    fs::write(path, output)
}

fn parse_keybinds(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    for token in raw.split(':') {
        if let Some(normalized) = normalize_token(token) {
            if !out.contains(&normalized) {
                out.push(normalized);
            }
        }
    }

    if out.is_empty() {
        out.push(DEFAULT_KEYBIND.to_owned());
    }
    out
}

fn parse_u32(raw: Option<&String>, default: u32) -> u32 {
    raw.and_then(|v| v.parse::<u32>().ok()).unwrap_or(default)
}

fn parse_bool(raw: Option<&String>, default: bool) -> bool {
    raw.and_then(|v| v.parse::<bool>().ok()).unwrap_or(default)
}
