//! Hotkey parsing and matching utilities

use eframe::egui::{self, Key};
use global_hotkey::hotkey::{Code, Modifiers};

/// Check if a hotkey string matches the current egui input state
/// Returns true if the hotkey is pressed
pub fn check_egui_hotkey(input: &egui::InputState, hotkey_str: &str) -> bool {
    let hotkey_lower = hotkey_str.to_lowercase();
    let parts: Vec<&str> = hotkey_lower.split('+').collect();
    if parts.is_empty() {
        return false;
    }

    let key_part = match parts.last() {
        Some(k) => *k,
        None => return false,
    };

    // Check modifiers
    let mut need_cmd = false;
    let mut need_ctrl = false;
    let mut need_alt = false;
    let mut need_shift = false;

    for part in &parts[..parts.len() - 1] {
        match *part {
            "cmd" | "command" | "super" | "win" => need_cmd = true,
            "ctrl" | "control" => need_ctrl = true,
            "alt" | "option" => need_alt = true,
            "shift" => need_shift = true,
            _ => {}
        }
    }

    // Check if modifiers match
    // On Windows/Linux, egui sets command == ctrl (both refer to Ctrl key)
    // So we treat "cmd" and "ctrl" as requesting the same logical modifier
    // If either cmd or ctrl is requested, we check if either is pressed
    let cmd_or_ctrl_required = need_cmd || need_ctrl;
    let cmd_or_ctrl_pressed = input.modifiers.command || input.modifiers.ctrl;

    let mods_match = if cmd_or_ctrl_required {
        // User wants cmd or ctrl - check if either is pressed, and no extra modifiers
        cmd_or_ctrl_pressed
            && input.modifiers.alt == need_alt
            && input.modifiers.shift == need_shift
    } else {
        // User doesn't want cmd/ctrl - ensure neither is pressed
        !cmd_or_ctrl_pressed
            && input.modifiers.alt == need_alt
            && input.modifiers.shift == need_shift
    };

    if !mods_match {
        return false;
    }

    // Map key string to egui Key
    let key = match key_part {
        "a" => Key::A,
        "b" => Key::B,
        "c" => Key::C,
        "d" => Key::D,
        "e" => Key::E,
        "f" => Key::F,
        "g" => Key::G,
        "h" => Key::H,
        "i" => Key::I,
        "j" => Key::J,
        "k" => Key::K,
        "l" => Key::L,
        "m" => Key::M,
        "n" => Key::N,
        "o" => Key::O,
        "p" => Key::P,
        "q" => Key::Q,
        "r" => Key::R,
        "s" => Key::S,
        "t" => Key::T,
        "u" => Key::U,
        "v" => Key::V,
        "w" => Key::W,
        "x" => Key::X,
        "y" => Key::Y,
        "z" => Key::Z,
        "0" => Key::Num0,
        "1" => Key::Num1,
        "2" => Key::Num2,
        "3" => Key::Num3,
        "4" => Key::Num4,
        "5" => Key::Num5,
        "6" => Key::Num6,
        "7" => Key::Num7,
        "8" => Key::Num8,
        "9" => Key::Num9,
        "space" => Key::Space,
        "enter" | "return" => Key::Enter,
        "escape" | "esc" => Key::Escape,
        "tab" => Key::Tab,
        "backspace" => Key::Backspace,
        "delete" => Key::Delete,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        _ => return false,
    };

    input.key_pressed(key)
}

/// Parse a hotkey string like "cmd+shift+v" into Modifiers and Code
/// Returns None if the string is invalid
pub fn parse_hotkey_string(hotkey_str: &str) -> Option<(Modifiers, Code)> {
    let hotkey_lower = hotkey_str.to_lowercase();
    let parts: Vec<&str> = hotkey_lower.split('+').collect();
    if parts.is_empty() {
        return None;
    }

    let mut modifiers = Modifiers::empty();
    let key_part = parts.last()?;

    for part in &parts[..parts.len() - 1] {
        match *part {
            "cmd" | "command" | "super" | "win" => modifiers |= Modifiers::SUPER,
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            _ => {}
        }
    }

    let code = match *key_part {
        "a" => Code::KeyA,
        "b" => Code::KeyB,
        "c" => Code::KeyC,
        "d" => Code::KeyD,
        "e" => Code::KeyE,
        "f" => Code::KeyF,
        "g" => Code::KeyG,
        "h" => Code::KeyH,
        "i" => Code::KeyI,
        "j" => Code::KeyJ,
        "k" => Code::KeyK,
        "l" => Code::KeyL,
        "m" => Code::KeyM,
        "n" => Code::KeyN,
        "o" => Code::KeyO,
        "p" => Code::KeyP,
        "q" => Code::KeyQ,
        "r" => Code::KeyR,
        "s" => Code::KeyS,
        "t" => Code::KeyT,
        "u" => Code::KeyU,
        "v" => Code::KeyV,
        "w" => Code::KeyW,
        "x" => Code::KeyX,
        "y" => Code::KeyY,
        "z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "space" => Code::Space,
        "enter" | "return" => Code::Enter,
        "escape" | "esc" => Code::Escape,
        "tab" => Code::Tab,
        "backspace" => Code::Backspace,
        "delete" => Code::Delete,
        "f1" => Code::F1,
        "f2" => Code::F2,
        "f3" => Code::F3,
        "f4" => Code::F4,
        "f5" => Code::F5,
        "f6" => Code::F6,
        "f7" => Code::F7,
        "f8" => Code::F8,
        "f9" => Code::F9,
        "f10" => Code::F10,
        "f11" => Code::F11,
        "f12" => Code::F12,
        _ => return None,
    };

    Some((modifiers, code))
}
