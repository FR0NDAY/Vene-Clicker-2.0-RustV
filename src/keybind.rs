pub const DEFAULT_KEYBIND: &str = "KeyF";

pub fn normalize_token(raw: &str) -> Option<String> {
    let token = raw.trim();
    if token.is_empty() {
        return None;
    }

    if let Ok(legacy_code) = token.parse::<i32>() {
        return legacy_code_to_token(legacy_code).map(ToOwned::to_owned);
    }

    Some(token.to_owned())
}

pub fn keybind_display(keybinds: &[String]) -> String {
    if keybinds.is_empty() {
        return "None".to_owned();
    }
    keybinds
        .iter()
        .map(|token| display_token(token))
        .collect::<Vec<_>>()
        .join(" + ")
}

fn display_token(token: &str) -> String {
    if let Some(rest) = token.strip_prefix("Key") {
        return rest.to_owned();
    }
    if let Some(rest) = token.strip_prefix("Num") {
        return rest.to_owned();
    }
    match token {
        "ControlLeft" | "ControlRight" => "Ctrl".to_owned(),
        "ShiftLeft" | "ShiftRight" => "Shift".to_owned(),
        "Alt" | "AltGr" => "Alt".to_owned(),
        "Return" => "Enter".to_owned(),
        "Space" => "Space".to_owned(),
        _ => token.to_owned(),
    }
}

fn legacy_code_to_token(code: i32) -> Option<&'static str> {
    match code {
        2 => Some("Num1"),
        3 => Some("Num2"),
        4 => Some("Num3"),
        5 => Some("Num4"),
        6 => Some("Num5"),
        7 => Some("Num6"),
        8 => Some("Num7"),
        9 => Some("Num8"),
        10 => Some("Num9"),
        11 => Some("Num0"),
        16 => Some("KeyQ"),
        17 => Some("KeyW"),
        18 => Some("KeyE"),
        19 => Some("KeyR"),
        20 => Some("KeyT"),
        21 => Some("KeyY"),
        22 => Some("KeyU"),
        23 => Some("KeyI"),
        24 => Some("KeyO"),
        25 => Some("KeyP"),
        29 => Some("ControlLeft"),
        30 => Some("KeyA"),
        31 => Some("KeyS"),
        32 => Some("KeyD"),
        33 => Some("KeyF"),
        34 => Some("KeyG"),
        35 => Some("KeyH"),
        36 => Some("KeyJ"),
        37 => Some("KeyK"),
        38 => Some("KeyL"),
        42 => Some("ShiftLeft"),
        44 => Some("KeyZ"),
        45 => Some("KeyX"),
        46 => Some("KeyC"),
        47 => Some("KeyV"),
        48 => Some("KeyB"),
        49 => Some("KeyN"),
        50 => Some("KeyM"),
        54 => Some("ShiftRight"),
        56 => Some("Alt"),
        57 => Some("Space"),
        58 => Some("CapsLock"),
        59 => Some("F1"),
        60 => Some("F2"),
        61 => Some("F3"),
        62 => Some("F4"),
        63 => Some("F5"),
        64 => Some("F6"),
        65 => Some("F7"),
        66 => Some("F8"),
        67 => Some("F9"),
        68 => Some("F10"),
        87 => Some("F11"),
        88 => Some("F12"),
        _ => None,
    }
}
