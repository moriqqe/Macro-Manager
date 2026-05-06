//! Import `.amc` (and related) macro payloads into [`MacroDefinition`].
//!
//! Supported:
//! - **JSON** (same shape as `import_macro_json`): UTF-8 or UTF-16 (BOM).
//! - **Line DSL** (UTF-8 / UTF-16 BOM): one command per line, `#` / `//` comments.
//!   - `DELAY <ms>` | `D <ms>` | `WAIT <ms>` | `SLEEP <ms>`
//!   - `MOUSE DOWN <L|LEFT|LMB|R|RIGHT|RMB|M|MIDDLE|MMB>` — same for `UP`
//!   - `LMB DOWN` / `LMB UP`, `RMB DOWN`, …
//!   - Short: `LD` `LU` `RD` `RU` `MD` `MU`
//!   - `KEY DOWN <vk>` | `KEY UP <vk>` (`0x10` or decimal); `KD` / `KU` shortcuts.

use std::path::Path;

use crate::types::{MacroDefinition, MacroStep};

pub fn decode_amc_bytes(bytes: &[u8]) -> Result<String, String> {
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return String::from_utf8(bytes[3..].to_vec()).map_err(|e| format!("utf-8 (bom): {e}"));
    }
    if bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] == 0xfe {
        return utf16_to_string(&bytes[2..], u16::from_le_bytes).map_err(|e| format!("utf-16le: {e}"));
    }
    if bytes.len() >= 2 && bytes[0] == 0xfe && bytes[1] == 0xff {
        return utf16_to_string(&bytes[2..], u16::from_be_bytes).map_err(|e| format!("utf-16be: {e}"));
    }
    String::from_utf8(bytes.to_vec()).map_err(|e| format!("utf-8: {e}"))
}

fn utf16_to_string(bytes: &[u8], from_bytes: fn([u8; 2]) -> u16) -> Result<String, String> {
    if bytes.len() % 2 != 0 {
        return Err("utf-16: odd byte length".into());
    }
    let u16s: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| from_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16(&u16s).map_err(|e| format!("invalid utf-16: {e}"))
}

fn slug_from_file_stem(file_name: &str) -> String {
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imported");
    let mut out: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else if c == '_' || c == '-' {
                '_'
            } else {
                '_'
            }
        })
        .collect();
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out = out.trim_matches('_').to_string();
    if out.is_empty() {
        "imported_macro".into()
    } else {
        out
    }
}

fn display_name_from_file(file_name: &str) -> String {
    Path::new(file_name)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(file_name)
        .to_string()
}

/// Parse file bytes into a macro. `file_name` is used for id / display name fallbacks.
pub fn parse_amc_bytes(bytes: &[u8], file_name: &str) -> Result<MacroDefinition, String> {
    let text = decode_amc_bytes(bytes)?;
    parse_amc_text(&text, file_name)
}

pub fn parse_amc_text(text: &str, file_name: &str) -> Result<MacroDefinition, String> {
    let stem_id = slug_from_file_stem(file_name);
    let disp = display_name_from_file(file_name);
    let t = text.trim_start();
    if t.starts_with('{') {
        let mut m: MacroDefinition =
            serde_json::from_str(t).map_err(|e| format!("amc json: {e}"))?;
        if m.id.trim().is_empty() {
            m.id = stem_id.clone();
        }
        if m.name.trim().is_empty() {
            m.name = disp.clone();
        }
        if m.steps.is_empty() {
            return Err("amc json: steps array is empty".into());
        }
        return Ok(m);
    }
    let steps = parse_line_dsl(text)?;
    if steps.is_empty() {
        return Err(
            "amc: no steps found. Use JSON (same as app import) or line commands:\n\
             DELAY 32 | MOUSE DOWN LEFT | MOUSE UP LEFT | KEY DOWN 0x46 | … (see source amc_import.rs)"
                .into(),
        );
    }
    Ok(MacroDefinition {
        id: stem_id,
        name: disp,
        version: 1,
        steps,
    })
}

fn parse_line_dsl(text: &str) -> Result<Vec<MacroStep>, String> {
    let mut steps = Vec::new();
    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let tokens: Vec<String> = line
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        if tokens.is_empty() {
            continue;
        }
        let t0 = tokens[0].to_ascii_uppercase();
        let rest = &tokens[1..];

        match t0.as_str() {
            "DELAY" | "D" | "WAIT" | "SLEEP" => {
                let ms = rest
                    .first()
                    .ok_or_else(|| err_line(lineno, "delay: missing ms"))?;
                let n = parse_u64(ms).map_err(|e| err_line(lineno, &e))?;
                steps.push(MacroStep::Delay { ms: n });
            }
            "MOUSE" => {
                let (dir, btn) = parse_mouse_line(rest).map_err(|e| err_line(lineno, &e))?;
                steps.push(match dir {
                    MouseDir::Down => MacroStep::MouseDown { button: btn },
                    MouseDir::Up => MacroStep::MouseUp { button: btn },
                });
            }
            "KEY" => {
                let (dir, vk) = parse_key_line(rest).map_err(|e| err_line(lineno, &e))?;
                steps.push(match dir {
                    KeyDir::Down => MacroStep::KeyDown { vk },
                    KeyDir::Up => MacroStep::KeyUp { vk },
                });
            }
            "LMB" | "RMB" | "MMB" => {
                let btn = match t0.as_str() {
                    "LMB" => "left".to_string(),
                    "RMB" => "right".to_string(),
                    _ => "middle".to_string(),
                };
                let dir = parse_mouse_dir_down_up(rest).map_err(|e| err_line(lineno, &e))?;
                steps.push(match dir {
                    MouseDir::Down => MacroStep::MouseDown {
                        button: btn.clone(),
                    },
                    MouseDir::Up => MacroStep::MouseUp { button: btn },
                });
            }
            "LD" => steps.push(MacroStep::MouseDown {
                button: "left".into(),
            }),
            "LU" => steps.push(MacroStep::MouseUp { button: "left".into() }),
            "RD" => steps.push(MacroStep::MouseDown {
                button: "right".into(),
            }),
            "RU" => steps.push(MacroStep::MouseUp {
                button: "right".into(),
            }),
            "MD" => steps.push(MacroStep::MouseDown {
                button: "middle".into(),
            }),
            "MU" => steps.push(MacroStep::MouseUp {
                button: "middle".into(),
            }),
            "KD" => {
                let vk = rest
                    .first()
                    .ok_or_else(|| err_line(lineno, "kd: missing vk"))?;
                let vk = parse_vk(vk).map_err(|e| err_line(lineno, &e))?;
                steps.push(MacroStep::KeyDown { vk });
            }
            "KU" => {
                let vk = rest
                    .first()
                    .ok_or_else(|| err_line(lineno, "ku: missing vk"))?;
                let vk = parse_vk(vk).map_err(|e| err_line(lineno, &e))?;
                steps.push(MacroStep::KeyUp { vk });
            }
            _ => {
                return Err(err_line(
                    lineno,
                    &format!("unknown command '{}'", tokens[0]),
                ));
            }
        }
    }
    Ok(steps)
}

fn err_line(lineno: usize, msg: &str) -> String {
    format!("amc line {}: {msg}", lineno + 1)
}

#[derive(Clone, Copy)]
enum MouseDir {
    Down,
    Up,
}

#[derive(Clone, Copy)]
enum KeyDir {
    Down,
    Up,
}

fn parse_mouse_line(tokens: &[String]) -> Result<(MouseDir, String), String> {
    let d = tokens
        .first()
        .ok_or_else(|| "mouse: missing DOWN/UP".to_string())?
        .to_ascii_uppercase();
    let dir = match d.as_str() {
        "DOWN" | "D" => MouseDir::Down,
        "UP" | "U" => MouseDir::Up,
        _ => return Err("mouse: expected DOWN or UP".into()),
    };
    let btn_word = tokens
        .get(1)
        .ok_or_else(|| "mouse: missing button".to_string())?;
    let btn = normalize_mouse_btn(btn_word)?;
    Ok((dir, btn))
}

fn parse_mouse_dir_down_up(tokens: &[String]) -> Result<MouseDir, String> {
    let d = tokens
        .first()
        .ok_or_else(|| "mouse btn: missing DOWN/UP".to_string())?
        .to_ascii_uppercase();
    match d.as_str() {
        "DOWN" | "D" => Ok(MouseDir::Down),
        "UP" | "U" => Ok(MouseDir::Up),
        _ => Err("expected DOWN or UP after mouse button".into()),
    }
}

fn normalize_mouse_btn(s: &str) -> Result<String, String> {
    match s.to_ascii_uppercase().as_str() {
        "L" | "LEFT" | "LMB" | "LEFTBUTTON" => Ok("left".into()),
        "R" | "RIGHT" | "RMB" | "RIGHTBUTTON" => Ok("right".into()),
        "M" | "MIDDLE" | "MMB" | "MIDDLEBUTTON" => Ok("middle".into()),
        _ => Err(format!("unknown mouse button '{s}'")),
    }
}

fn parse_key_line(tokens: &[String]) -> Result<(KeyDir, u32), String> {
    let d = tokens
        .first()
        .ok_or_else(|| "key: missing DOWN/UP".to_string())?
        .to_ascii_uppercase();
    let dir = match d.as_str() {
        "DOWN" | "D" => KeyDir::Down,
        "UP" | "U" => KeyDir::Up,
        _ => return Err("key: expected DOWN or UP".into()),
    };
    let vk = tokens
        .get(1)
        .ok_or_else(|| "key: missing vk".to_string())?;
    let vk = parse_vk(vk)?;
    Ok((dir, vk))
}

fn parse_vk(s: &str) -> Result<u32, String> {
    let t = s.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).map_err(|e| format!("vk hex: {e}"))
    } else {
        t.parse::<u32>().map_err(|e| format!("vk: {e}"))
    }
}

fn parse_u64(s: &str) -> Result<u64, String> {
    let t = s.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).map_err(|e| format!("{e}"))
    } else {
        t.parse::<u64>().map_err(|e| format!("{e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_dsl_basic() {
        let text = "DELAY 10\nLD\nLU\nKD 0x05\n";
        let m = parse_amc_text(text, "test.amc").unwrap();
        assert_eq!(m.id, "test");
        assert_eq!(m.steps.len(), 4);
    }

    #[test]
    fn json_in_amc() {
        let j = r#"{"id":"x","name":"n","version":1,"steps":[{"type":"delay","ms":5}]}"#;
        let m = parse_amc_text(j, "f.amc").unwrap();
        assert_eq!(m.id, "x");
        assert_eq!(m.steps.len(), 1);
    }
}
