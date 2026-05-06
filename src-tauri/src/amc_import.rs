//! Import `.amc` (and related) macro payloads into [`MacroDefinition`].
//!
//! Supported:
//! - **JSON** (same shape as `import_macro_json`): UTF-8 or UTF-16 (BOM).
//! - **F1DN-style XML** (UTF-8 / UTF-16 BOM): `<Root><DefaultMacro><KeyDown><Syntax>…</Syntax>` with
//!   `Delay`, `MoveR`, `LeftDown` / `LeftUp`, etc.
//! - **Line DSL** (UTF-8 / UTF-16 BOM): one command per line, `#` / `//` comments.
//!   - `DELAY <ms>` | `D <ms>` | `WAIT <ms>` | `SLEEP <ms>`
//!   - `MOVER <dx> <dy>` — relative mouse move (same as `MoveR` in device exports)
//!   - `MOUSE DOWN <L|LEFT|LMB|R|RIGHT|RMB|M|MIDDLE|MMB>` — same for `UP`
//!   - `LEFTDOWN` / `LEFTUP`, `RIGHTDOWN` / `RIGHTUP`, `MIDDLEDOWN` / `MIDDLEUP` (F1DN-style)
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
    if let Some(syntax) = extract_f1dn_xml_macro_syntax(t) {
        let steps = parse_f1dn_syntax(&syntax)?;
        if steps.is_empty() {
            return Err("amc xml: <Syntax> produced no steps".into());
        }
        let name = extract_xml_description(t)
            .as_ref()
            .map(|s| first_non_empty_line(s))
            .filter(|s| !s.is_empty())
            .unwrap_or(disp.clone());
        return Ok(MacroDefinition {
            id: stem_id,
            name,
            version: 1,
            steps,
        });
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

fn find_ci(haystack: &str, needle: &str) -> Option<usize> {
    let hb = haystack.as_bytes();
    let nb = needle.as_bytes();
    if nb.is_empty() || hb.len() < nb.len() {
        return None;
    }
    for i in 0..=hb.len() - nb.len() {
        if hb[i..i + nb.len()]
            .iter()
            .zip(nb.iter())
            .all(|(a, b)| a.to_ascii_lowercase() == b.to_ascii_lowercase())
        {
            return Some(i);
        }
    }
    None
}

fn extract_syntax_inner_block(block: &str) -> Option<&str> {
    let open = "<Syntax>";
    let close = "</Syntax>";
    let o = find_ci(block, open)?;
    let inner = &block[o + open.len()..];
    let c = find_ci(inner, close)?;
    Some(inner[..c].trim())
}

fn extract_longest_syntax_from_keydown_blocks(xml: &str) -> Option<&str> {
    let mut search = xml;
    let mut best: Option<&str> = None;
    let open_tag = "<KeyDown>";
    let close_tag = "</KeyDown>";
    while let Some(kd_pos) = find_ci(search, open_tag) {
        let inner_start = kd_pos + open_tag.len();
        let tail = &search[inner_start..];
        let Some(close_rel) = find_ci(tail, close_tag) else {
            break;
        };
        let block = &tail[..close_rel];
        if let Some(syn) = extract_syntax_inner_block(block) {
            let t = syn.trim();
            if !t.is_empty() && best.map(|b| b.len() < t.len()).unwrap_or(true) {
                best = Some(t);
            }
        }
        search = &tail[close_rel + close_tag.len()..];
    }
    best
}

fn extract_f1dn_xml_macro_syntax(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    if !lower.contains("<syntax>")
        || (!lower.contains("<root>") && !lower.contains("<defaultmacro>"))
    {
        return None;
    }
    let body = extract_longest_syntax_from_keydown_blocks(text)?;
    let t = body.trim();
    if t.is_empty() {
        return None;
    }
    Some(t.to_string())
}

fn extract_xml_description(xml: &str) -> Option<String> {
    let open_tag = "<Description>";
    let close_tag = "</Description>";
    let mut search = xml;
    let mut best: Option<String> = None;
    while let Some(p) = find_ci(search, open_tag) {
        let inner_start = p + open_tag.len();
        let tail = &search[inner_start..];
        let Some(c) = find_ci(tail, close_tag) else {
            break;
        };
        let inner = tail[..c].trim();
        if !inner.is_empty() && best.as_ref().map(|b| b.len() < inner.len()).unwrap_or(true) {
            best = Some(inner.to_string());
        }
        search = &tail[c + close_tag.len()..];
    }
    best
}

fn first_non_empty_line(s: &str) -> String {
    s.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("")
        .to_string()
}

fn parse_i32(s: &str) -> Result<i32, String> {
    let t = s.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        i32::from_str_radix(hex, 16).map_err(|e| format!("{e}"))
    } else {
        t.parse::<i32>().map_err(|e| format!("{e}"))
    }
}

fn parse_delay_ms_tokens(rest: &[String]) -> Result<u64, String> {
    let ms = rest
        .first()
        .ok_or_else(|| "delay: missing ms".to_string())?;
    parse_u64(ms)
}

fn parse_f1dn_syntax(text: &str) -> Result<Vec<MacroStep>, String> {
    let mut steps = Vec::new();
    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let tokens: Vec<String> = line.split_whitespace().map(|s| s.to_string()).collect();
        if tokens.is_empty() {
            continue;
        }
        let t0 = tokens[0].to_ascii_uppercase();
        let rest = &tokens[1..];

        match t0.as_str() {
            "DELAY" | "D" | "WAIT" | "SLEEP" => {
                let n = parse_delay_ms_tokens(rest).map_err(|e| err_line(lineno, &e))?;
                steps.push(MacroStep::Delay { ms: n });
            }
            "LEFTDOWN" => steps.push(MacroStep::MouseDown {
                button: "left".into(),
            }),
            "LEFTUP" => steps.push(MacroStep::MouseUp {
                button: "left".into(),
            }),
            "RIGHTDOWN" => steps.push(MacroStep::MouseDown {
                button: "right".into(),
            }),
            "RIGHTUP" => steps.push(MacroStep::MouseUp {
                button: "right".into(),
            }),
            "MIDDLEDOWN" => steps.push(MacroStep::MouseDown {
                button: "middle".into(),
            }),
            "MIDDLEUP" => steps.push(MacroStep::MouseUp {
                button: "middle".into(),
            }),
            "MOVER" => {
                let dx = rest
                    .first()
                    .ok_or_else(|| err_line(lineno, "MoveR: missing dx"))?;
                let dy = rest
                    .get(1)
                    .ok_or_else(|| err_line(lineno, "MoveR: missing dy"))?;
                let dx = parse_i32(dx).map_err(|e| err_line(lineno, &e))?;
                let dy = parse_i32(dy).map_err(|e| err_line(lineno, &e))?;
                steps.push(MacroStep::MouseMoveRel { dx, dy });
            }
            _ => {
                return Err(err_line(
                    lineno,
                    &format!("unknown F1DN command '{}'", tokens[0]),
                ));
            }
        }
    }
    Ok(steps)
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
            "MOVER" | "MOVEREL" => {
                let dx = rest
                    .first()
                    .ok_or_else(|| err_line(lineno, "MoveR: missing dx"))?;
                let dy = rest
                    .get(1)
                    .ok_or_else(|| err_line(lineno, "MoveR: missing dy"))?;
                let dx = parse_i32(dx).map_err(|e| err_line(lineno, &e))?;
                let dy = parse_i32(dy).map_err(|e| err_line(lineno, &e))?;
                steps.push(MacroStep::MouseMoveRel { dx, dy });
            }
            "LEFTDOWN" => steps.push(MacroStep::MouseDown {
                button: "left".into(),
            }),
            "LEFTUP" => steps.push(MacroStep::MouseUp {
                button: "left".into(),
            }),
            "RIGHTDOWN" => steps.push(MacroStep::MouseDown {
                button: "right".into(),
            }),
            "RIGHTUP" => steps.push(MacroStep::MouseUp {
                button: "right".into(),
            }),
            "MIDDLEDOWN" => steps.push(MacroStep::MouseDown {
                button: "middle".into(),
            }),
            "MIDDLEUP" => steps.push(MacroStep::MouseUp {
                button: "middle".into(),
            }),
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
    fn f1dn_xml_recognizes_move_and_delay() {
        let xml = r#"<Root><DefaultMacro>
<KeyDown><Syntax></Syntax></KeyDown>
<KeyDown><Syntax>LeftDown 1
Delay 10 ms
MoveR 0 2
LeftUp 1
</Syntax></KeyDown>
</DefaultMacro></Root>"#;
        let m = parse_amc_text(xml, "t.amc").unwrap();
        assert!(m.steps.iter().any(|s| matches!(s, MacroStep::MouseMoveRel { dx: 0, dy: 2 })));
        assert!(m.steps.iter().any(|s| matches!(s, MacroStep::Delay { ms: 10 })));
    }

    #[test]
    fn json_in_amc() {
        let j = r#"{"id":"x","name":"n","version":1,"steps":[{"type":"delay","ms":5}]}"#;
        let m = parse_amc_text(j, "f.amc").unwrap();
        assert_eq!(m.id, "x");
        assert_eq!(m.steps.len(), 1);
    }
}
