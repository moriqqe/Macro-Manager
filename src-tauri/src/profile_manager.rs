use crate::types::{
    AppConfig, ExecutionMode, GameProfile, MacroDefinition, MacroStep, WeaponBinding, WeaponTemplate,
};

/// Syncs `icon_url` from the built-in defaults for each weapon (by game + id).
/// Returns `true` if any value changed (e.g. old HTTPS URLs → bundled `assets/weapons/...`).
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
                Some(s) => s.starts_with("http://") || s.starts_with("https://"),
            };
            if replace && w.icon_url.as_ref() != Some(bundled) {
                w.icon_url = Some(bundled.clone());
                changed = true;
            }
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

fn game_pubg() -> GameProfile {
    // Bundled icons (sources: pubg.wiki.gg UI weapon icons)
    let weapons = vec![
        tpl_icon("m416", "M416", "AR", "5.56", 720, 0.62, Some("assets/weapons/pubg/m416.png")),
        tpl_icon("akm", "AKM", "AR", "7.62", 600, 0.78, Some("assets/weapons/pubg/akm.png")),
        tpl_icon(
            "scarl",
            "SCAR-L",
            "AR",
            "5.56",
            625,
            0.55,
            Some("assets/weapons/pubg/scarl.png"),
        ),
        tpl_icon(
            "beryl",
            "Beryl M762",
            "AR",
            "7.62",
            700,
            0.85,
            Some("assets/weapons/pubg/beryl.png"),
        ),
        tpl_icon(
            "ump45",
            "UMP45",
            "SMG",
            ".45",
            670,
            0.41,
            Some("assets/weapons/pubg/ump45.png"),
        ),
        tpl_icon(
            "vector",
            "Vector",
            "SMG",
            ".45",
            1100,
            0.38,
            Some("assets/weapons/pubg/vector.png"),
        ),
        tpl_icon(
            "mini14",
            "Mini-14",
            "DMR",
            "5.56",
            250,
            0.28,
            Some("assets/weapons/pubg/mini14.png"),
        ),
        tpl_icon(
            "kar98",
            "Kar98K",
            "SR",
            "7.62",
            40,
            0.22,
            Some("assets/weapons/pubg/kar98.png"),
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
        bind(
            "ump45",
            Some("sample-recoil"),
            Some((0, 0x47)),
            ExecutionMode::Tap,
            true,
        ),
        bind("vector", None, None, ExecutionMode::Hold, false),
        bind("mini14", None, None, ExecutionMode::Tap, false),
        bind("kar98", None, None, ExecutionMode::Tap, false),
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
    // Bundled icons (sources: RustLabs items180)
    let weapons = vec![
        tpl_icon(
            "ak47",
            "AK-47",
            "AR",
            "5.56",
            600,
            0.82,
            Some("assets/weapons/rust/ak47.png"),
        ),
        tpl_icon(
            "lr300",
            "LR-300",
            "AR",
            "5.56",
            600,
            0.58,
            Some("assets/weapons/rust/lr300.png"),
        ),
        tpl_icon(
            "mp5a4",
            "MP5A4",
            "SMG",
            "9mm",
            800,
            0.44,
            Some("assets/weapons/rust/mp5a4.png"),
        ),
        tpl_icon(
            "thompson",
            "Thompson",
            "SMG",
            ".45",
            600,
            0.52,
            Some("assets/weapons/rust/thompson.png"),
        ),
        tpl_icon(
            "custom",
            "Custom SMG",
            "SMG",
            "9mm",
            750,
            0.48,
            Some("assets/weapons/rust/custom.png"),
        ),
        tpl_icon(
            "m249",
            "M249",
            "LMG",
            "5.56",
            700,
            0.88,
            Some("assets/weapons/rust/m249.png"),
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
            Some("assets/weapons/cs2/ak47cs.svg"),
        ),
        tpl_icon(
            "m4a4",
            "M4A4",
            "Rifle",
            "5.56",
            666,
            0.51,
            Some("assets/weapons/cs2/m4a4.svg"),
        ),
        tpl_icon(
            "m4a1s",
            "M4A1-S",
            "Rifle",
            "5.56",
            600,
            0.42,
            Some("assets/weapons/cs2/m4a1s.svg"),
        ),
        tpl_icon(
            "famas",
            "FAMAS",
            "Rifle",
            "5.56",
            666,
            0.46,
            Some("assets/weapons/cs2/famas.svg"),
        ),
        tpl_icon(
            "galil",
            "Galil AR",
            "Rifle",
            "5.56",
            666,
            0.61,
            Some("assets/weapons/cs2/galil.svg"),
        ),
        tpl_icon(
            "mp9",
            "MP9",
            "SMG",
            "9mm",
            857,
            0.32,
            Some("assets/weapons/cs2/mp9.svg"),
        ),
        tpl_icon(
            "ump45cs",
            "UMP-45",
            "SMG",
            ".45",
            666,
            0.36,
            Some("assets/weapons/cs2/ump45cs.svg"),
        ),
        tpl_icon(
            "awp",
            "AWP",
            "Sniper",
            ".338",
            41,
            0.20,
            Some("assets/weapons/cs2/awp.svg"),
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
        bind("ump45cs", None, None, ExecutionMode::Hold, false),
        bind("awp", None, None, ExecutionMode::Tap, false),
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
    icon_url: Option<&str>,
) -> WeaponTemplate {
    WeaponTemplate {
        id: id.into(),
        name: name.into(),
        class: class.into(),
        caliber: caliber.into(),
        rpm: Some(rpm),
        recoil: Some(recoil),
        icon_url: icon_url.map(String::from),
    }
}
