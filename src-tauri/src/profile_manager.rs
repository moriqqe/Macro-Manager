use std::collections::HashSet;

use crate::types::{
    AppConfig, ExecutionMode, GameProfile, MacroDefinition, MacroStep, WeaponBinding, WeaponTemplate,
};

/// Syncs `icon_url` from the built-in defaults for each weapon (by game + id).
/// Returns `true` if any value changed (e.g. old `assets/` paths → embedded `data:` URLs).
pub fn apply_default_weapon_icons(cfg: &mut AppConfig) -> bool {
    let default = default_config();
    let mut changed = false;
    for gp in &mut cfg.game_profiles {
        let Some(def_gp) = default.game_profiles.iter().find(|g| g.id == gp.id) else {
            continue;
        };
        for w in &mut gp.weapons {
            let Some(def_w) = def_gp.weapons.iter().find(|x| x.id == w.id) else {
                continue;
            };
            let Some(ref bundled) = def_w.icon_url else {
                continue;
            };
            let replace = match &w.icon_url {
                None => true,
                Some(s) => {
                    s.starts_with("http://")
                        || s.starts_with("https://")
                        || s.starts_with("assets/")
                }
            };
            if replace && w.icon_url.as_ref() != Some(bundled) {
                w.icon_url = Some(bundled.clone());
                changed = true;
            }
        }
    }
    changed
}

/// Drops weapons and bindings that no longer exist in built-in defaults (e.g. removed guns).
/// Reorders remaining weapons to match default profile order.
pub fn sync_weapons_to_defaults(cfg: &mut AppConfig) -> bool {
    let default = default_config();
    let mut changed = false;
    for gp in &mut cfg.game_profiles {
        let Some(def_gp) = default.game_profiles.iter().find(|g| g.id == gp.id) else {
            continue;
        };
        let allowed: HashSet<_> = def_gp.weapons.iter().map(|w| w.id.as_str()).collect();
        let before_w = gp.weapons.len();
        gp.weapons.retain(|w| allowed.contains(w.id.as_str()));
        if gp.weapons.len() != before_w {
            changed = true;
        }
        let order: Vec<_> = def_gp.weapons.iter().map(|w| w.id.clone()).collect();
        gp.weapons.sort_by_key(|w| {
            order
                .iter()
                .position(|id| id == &w.id)
                .unwrap_or(usize::MAX)
        });
        let before_b = gp.bindings.len();
        gp.bindings
            .retain(|b| allowed.contains(b.weapon_id.as_str()));
        if gp.bindings.len() != before_b {
            changed = true;
        }
    }
    changed
}

pub fn default_config() -> AppConfig {
    let sample_macro = MacroDefinition {
        id: "sample-recoil".into(),
        name: "Sample pattern".into(),
        version: 1,
        steps: vec![
            MacroStep::MouseDown {
                button: "left".into(),
            },
            MacroStep::Delay { ms: 32 },
            MacroStep::MouseUp {
                button: "left".into(),
            },
        ],
    };

    AppConfig {
        schema_version: 1,
        master_enabled: true,
        active_game: "pubg".into(),
        jitter_ms: Some((0, 3)),
        macros: vec![sample_macro],
        game_profiles: vec![game_pubg(), game_rust(), game_cs2()],
    }
}

fn bind(
    weapon_id: &str,
    macro_id: Option<&str>,
    hotkey: Option<(u32, u32)>,
    mode: ExecutionMode,
    enabled: bool,
) -> WeaponBinding {
    WeaponBinding {
        weapon_id: weapon_id.into(),
        macro_id: macro_id.map(String::from),
        hotkey: hotkey.map(|(m, vk)| crate::types::HotkeySpec {
            modifiers: m,
            vk,
        }),
        mode,
        enabled,
    }
}

fn wicon(game: &str, weapon_id: &str) -> Option<String> {
    crate::embedded_weapon_icons::weapon_icon_data_url(game, weapon_id).map(str::to_string)
}

fn game_pubg() -> GameProfile {
    let weapons = vec![
        tpl_icon("m416", "M416", "AR", "5.56", 720, 0.62, wicon("pubg", "m416")),
        tpl_icon("akm", "AKM", "AR", "7.62", 600, 0.78, wicon("pubg", "akm")),
        tpl_icon(
            "scarl",
            "SCAR-L",
            "AR",
            "5.56",
            625,
            0.55,
            wicon("pubg", "scarl"),
        ),
        tpl_icon(
            "beryl",
            "Beryl M762",
            "AR",
            "7.62",
            700,
            0.85,
            wicon("pubg", "beryl"),
        ),
        tpl_icon(
            "vector",
            "Vector",
            "SMG",
            ".45",
            1100,
            0.38,
            wicon("pubg", "vector"),
        ),
        tpl_icon(
            "mini14",
            "Mini-14",
            "DMR",
            "5.56",
            250,
            0.28,
            wicon("pubg", "mini14"),
        ),
    ];
    let bindings = vec![
        bind(
            "m416",
            Some("sample-recoil"),
            Some((0, 0x05)),
            ExecutionMode::Hold,
            true,
        ), // mouse 4
        bind(
            "akm",
            Some("sample-recoil"),
            Some((0, 0x06)),
            ExecutionMode::Hold,
            true,
        ), // mouse 5
        bind(
            "scarl",
            Some("sample-recoil"),
            Some((0, 0x46)),
            ExecutionMode::Toggle,
            true,
        ), // F
        bind("beryl", None, None, ExecutionMode::Hold, false),
        bind("vector", None, None, ExecutionMode::Hold, false),
        bind("mini14", None, None, ExecutionMode::Tap, false),
    ];
    GameProfile {
        id: "pubg".into(),
        display_name: "PUBG: Battlegrounds".into(),
        subtitle: "Battle royale · 100 players".into(),
        profile_label: "PROFILE 03".into(),
        weapons,
        bindings,
    }
}

fn game_rust() -> GameProfile {
    let weapons = vec![
        tpl_icon(
            "ak47",
            "AK-47",
            "AR",
            "5.56",
            600,
            0.82,
            wicon("rust", "ak47"),
        ),
        tpl_icon(
            "lr300",
            "LR-300",
            "AR",
            "5.56",
            600,
            0.58,
            wicon("rust", "lr300"),
        ),
        tpl_icon(
            "mp5a4",
            "MP5A4",
            "SMG",
            "9mm",
            800,
            0.44,
            wicon("rust", "mp5a4"),
        ),
        tpl_icon(
            "thompson",
            "Thompson",
            "SMG",
            ".45",
            600,
            0.52,
            wicon("rust", "thompson"),
        ),
        tpl_icon(
            "custom",
            "Custom SMG",
            "SMG",
            "9mm",
            750,
            0.48,
            wicon("rust", "custom"),
        ),
        tpl_icon(
            "m249",
            "M249",
            "LMG",
            "5.56",
            700,
            0.88,
            wicon("rust", "m249"),
        ),
    ];
    let bindings = vec![
        bind(
            "ak47",
            Some("sample-recoil"),
            Some((0, 0x05)),
            ExecutionMode::Hold,
            true,
        ),
        bind(
            "lr300",
            Some("sample-recoil"),
            Some((0, 0x06)),
            ExecutionMode::Hold,
            true,
        ),
        bind("mp5a4", None, None, ExecutionMode::Hold, false),
        bind("thompson", None, None, ExecutionMode::Hold, false),
        bind(
            "custom",
            Some("sample-recoil"),
            Some((0, 0x51)),
            ExecutionMode::Toggle,
            true,
        ), // Q
        bind("m249", None, None, ExecutionMode::Hold, false),
    ];
    GameProfile {
        id: "rust".into(),
        display_name: "Rust".into(),
        subtitle: "Survival · PvP".into(),
        profile_label: "PROFILE 01".into(),
        weapons,
        bindings,
    }
}

fn game_cs2() -> GameProfile {
    let weapons = vec![
        tpl_icon(
            "ak47cs",
            "AK-47",
            "Rifle",
            "7.62",
            600,
            0.74,
            wicon("cs2", "ak47cs"),
        ),
        tpl_icon(
            "m4a4",
            "M4A4",
            "Rifle",
            "5.56",
            666,
            0.51,
            wicon("cs2", "m4a4"),
        ),
        tpl_icon(
            "m4a1s",
            "M4A1-S",
            "Rifle",
            "5.56",
            600,
            0.42,
            wicon("cs2", "m4a1s"),
        ),
        tpl_icon(
            "famas",
            "FAMAS",
            "Rifle",
            "5.56",
            666,
            0.46,
            wicon("cs2", "famas"),
        ),
        tpl_icon(
            "galil",
            "Galil AR",
            "Rifle",
            "5.56",
            666,
            0.61,
            wicon("cs2", "galil"),
        ),
        tpl_icon(
            "mp9",
            "MP9",
            "SMG",
            "9mm",
            857,
            0.32,
            wicon("cs2", "mp9"),
        ),
    ];
    let bindings = vec![
        bind(
            "ak47cs",
            Some("sample-recoil"),
            Some((0, 0x05)),
            ExecutionMode::Hold,
            true,
        ),
        bind(
            "m4a4",
            Some("sample-recoil"),
            Some((0, 0x06)),
            ExecutionMode::Hold,
            true,
        ),
        bind(
            "m4a1s",
            Some("sample-recoil"),
            Some((0, 0x46)),
            ExecutionMode::Hold,
            true,
        ),
        bind("famas", None, None, ExecutionMode::Tap, false),
        bind("galil", None, None, ExecutionMode::Hold, false),
        bind(
            "mp9",
            Some("sample-recoil"),
            Some((0, 0x47)),
            ExecutionMode::Tap,
            true,
        ),
    ];
    GameProfile {
        id: "cs2".into(),
        display_name: "Counter-Strike 2".into(),
        subtitle: "Tactical FPS · Competitive".into(),
        profile_label: "PROFILE 07".into(),
        weapons,
        bindings,
    }
}

fn tpl(id: &str, name: &str, class: &str, caliber: &str, rpm: u32, recoil: f32) -> WeaponTemplate {
    tpl_icon(id, name, class, caliber, rpm, recoil, None)
}

fn tpl_icon(
    id: &str,
    name: &str,
    class: &str,
    caliber: &str,
    rpm: u32,
    recoil: f32,
    icon_url: Option<String>,
) -> WeaponTemplate {
    WeaponTemplate {
        id: id.into(),
        name: name.into(),
        class: class.into(),
        caliber: caliber.into(),
        rpm: Some(rpm),
        recoil: Some(recoil),
        icon_url,
    }
}
