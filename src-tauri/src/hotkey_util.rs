//! Parse / format hotkeys for storage + hook matching.

use crate::types::HotkeySpec;

#[cfg(windows)]
pub fn async_modifiers() -> u32 {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    let mut m = 0u32;
    if unsafe { GetAsyncKeyState(0x12i32) } as u16 & 0x8000 != 0 {
        m |= 1;
    } // VK_MENU (ALT)
    if unsafe { GetAsyncKeyState(0x11i32) } as u16 & 0x8000 != 0 {
        m |= 2;
    } // VK_CONTROL
    if unsafe { GetAsyncKeyState(0x10i32) } as u16 & 0x8000 != 0 {
        m |= 4;
    } // VK_SHIFT
    m
}

#[cfg(not(windows))]
pub fn async_modifiers() -> u32 {
    0
}

pub fn format_hotkey(spec: &HotkeySpec) -> String {
    let mut parts: Vec<String> = Vec::new();
    if spec.modifiers & 2 != 0 {
        parts.push("CTRL".into());
    }
    if spec.modifiers & 4 != 0 {
        parts.push("SHIFT".into());
    }
    if spec.modifiers & 1 != 0 {
        parts.push("ALT".into());
    }
    parts.push(vk_label(spec.vk));
    parts.join(" + ")
}

fn vk_label(vk: u32) -> String {
    match vk {
        0x05 => "MOUSE 4".into(),
        0x06 => "MOUSE 5".into(),
        0x01 => "LMB".into(),
        0x02 => "RMB".into(),
        0x04 => "MMB".into(),
        0x20 => "SPACE".into(),
        0x08 | 0x09 => "BACK".into(),
        0x1B => "ESC".into(),
        0x2D => "INS".into(),
        0x2E => "DEL".into(),
        n if (0x30..=0x39).contains(&n) => {
            char::from_u32(n).map(|c| c.to_string()).unwrap_or_else(|| format!("0x{n:X}"))
        }
        n if (0x41..=0x5A).contains(&n) => {
            char::from_u32(n).map(|c| c.to_string()).unwrap_or_else(|| format!("0x{n:X}"))
        }
        vk @ 0x70..=0x7B => format!("F{}", vk - 0x6F),
        _ => format!("0x{vk:X}"),
    }
}

/// Accepts UI labels like `"MOUSE 4"`, `"CTRL + F"`, `"f"`.
pub fn parse_hotkey_label(s: &str) -> Result<HotkeySpec, String> {
    let t = s.trim();
    if t.is_empty() || t == "—" || t.eq_ignore_ascii_case("unbound") {
        return Err("empty".into());
    }
    let upper = t.to_uppercase();

    let (mods, rest) = parse_modifiers(&upper);

    let rest = rest.trim();
    if rest.is_empty() {
        return Err("missing key".into());
    }

    let vk = if rest == "MOUSE 4" || rest == "MOUSE4" {
        0x05
    } else if rest == "MOUSE 5" || rest == "MOUSE5" {
        0x06
    } else if rest == "LMB" || rest == "LEFT CLICK" {
        0x01
    } else if rest == "RMB" || rest == "RIGHT CLICK" {
        0x02
    } else if rest == "MMB" || rest == "MIDDLE CLICK" {
        0x04
    } else if rest == "SPACE" || rest == " " {
        0x20
    } else if rest.starts_with('F') && rest.len() > 1 {
        let n: u32 = rest[1..]
            .parse()
            .map_err(|_| "bad F-key".to_string())?;
        if !(1..=24).contains(&n) {
            return Err("F-key out of range".into());
        }
        0x6F + n
    } else if rest.len() == 1 {
        let c = rest.chars().next().unwrap();
        let v = if c.is_ascii_alphabetic() {
            c.to_ascii_uppercase() as u32
        } else if c.is_ascii_digit() {
            c as u32
        } else {
            return Err("unsupported key".into());
        };
        v
    } else {
        return Err("unsupported hotkey token".into());
    };

    Ok(HotkeySpec { modifiers: mods, vk })
}

fn parse_modifiers(upper: &str) -> (u32, String) {
    let mut m = 0u32;
    let mut parts: Vec<&str> = upper.split('+').map(str::trim).filter(|x| !x.is_empty()).collect();
    let mut rest_parts = Vec::new();
    for p in parts.drain(..) {
        match p {
            "CTRL" | "CONTROL" => m |= 2,
            "SHIFT" => m |= 4,
            "ALT" => m |= 1,
            other => rest_parts.push(other),
        }
    }
    (m, rest_parts.join(" + "))
}
