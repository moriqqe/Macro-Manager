use std::collections::HashSet;
use std::sync::{Arc, Mutex, RwLock};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Deserialize;
use tauri::State;

use crate::config_manager;
use crate::hotkey_util;
use crate::types::{
    AppConfig, ExecutionMode, HotkeySpec, LoadResponse, MacroDefinition, MacroStep, UiConfig, UiGame,
    UiMacroStep, UiWeapon,
};

pub struct AppState {
    pub config: Arc<RwLock<AppConfig>>,
    pub engine: crate::macro_engine::MacroEngine,
    pub armed: Arc<Mutex<Option<(String, String)>>>,
}

impl AppState {
    pub fn new(cfg: AppConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(cfg)),
            engine: crate::macro_engine::MacroEngine::new(),
            armed: Arc::new(Mutex::new(None)),
        }
    }
}

fn armed_weapon_for_active(cfg: &AppConfig, armed: &Mutex<Option<(String, String)>>) -> Option<String> {
    let lock = armed.lock().ok()?;
    let (ref gid, ref wid) = lock.as_ref()?;
    if gid == &cfg.active_game {
        Some(wid.clone())
    } else {
        None
    }
}

fn validate_config(cfg: &AppConfig) -> Result<(), String> {
    for gp in &cfg.game_profiles {
        let mut seen = HashSet::new();
        for b in &gp.bindings {
            if !b.enabled {
                continue;
            }
            let Some(h) = &b.hotkey else { continue };
            let k = (h.modifiers, h.vk);
            if !seen.insert(k) {
                return Err(format!(
                    "Duplicate hotkey ({}, {}) for game {}",
                    h.modifiers, h.vk, gp.id
                ));
            }
        }
    }
    let fb = HotkeySpec {
        modifiers: 0,
        vk: cfg.fire_button_vk,
    };
    let fire_label = hotkey_util::format_hotkey(&fb);
    for gp in &cfg.game_profiles {
        for b in &gp.bindings {
            if !b.enabled {
                continue;
            }
            let Some(h) = &b.hotkey else { continue };
            if h.modifiers == 0 && h.vk == cfg.fire_button_vk {
                return Err(format!(
                    "Weapon `{}` profile hotkey matches primary fire ({}). Hold CTRL/ALT/SHIFT with the profile key, or pick another primary fire button.",
                    b.weapon_id, fire_label
                ));
            }
        }
    }
    Ok(())
}

fn game_code(id: &str) -> String {
    match id {
        "pubg" => "PUBG".into(),
        "rust" => "RUST".into(),
        "cs2" => "CS2".into(),
        _ => id.to_uppercase(),
    }
}

fn macro_preview(m: &MacroDefinition) -> Vec<UiMacroStep> {
    let mut t = 0u64;
    let mut out = Vec::new();
    for step in &m.steps {
        match step {
            MacroStep::Delay { ms } => {
                out.push(UiMacroStep {
                    t,
                    kind: "delay".into(),
                    action: "WAIT".into(),
                    value: format!("{ms}ms"),
                });
                t = t.saturating_add(*ms);
            }
            MacroStep::MouseDown { button } => {
                out.push(UiMacroStep {
                    t,
                    kind: "mouse".into(),
                    action: "DOWN".into(),
                    value: button.to_uppercase(),
                });
            }
            MacroStep::MouseUp { button } => {
                out.push(UiMacroStep {
                    t,
                    kind: "mouse".into(),
                    action: "UP".into(),
                    value: button.to_uppercase(),
                });
            }
            MacroStep::MouseMoveRel { dx, dy } => {
                out.push(UiMacroStep {
                    t,
                    kind: "mouse".into(),
                    action: "MOVE".into(),
                    value: format!("{dx},{dy}"),
                });
            }
            MacroStep::KeyDown { vk } => {
                out.push(UiMacroStep {
                    t,
                    kind: "key".into(),
                    action: "DOWN".into(),
                    value: format!("VK_{vk:X}"),
                });
            }
            MacroStep::KeyUp { vk } => {
                out.push(UiMacroStep {
                    t,
                    kind: "key".into(),
                    action: "UP".into(),
                    value: format!("VK_{vk:X}"),
                });
            }
        }
    }
    out
}

fn build_ui(cfg: &AppConfig, armed: &Mutex<Option<(String, String)>>) -> UiConfig {
    let armed_wid = armed_weapon_for_active(cfg, armed);
    let fire_label = hotkey_util::format_hotkey(&HotkeySpec {
        modifiers: 0,
        vk: cfg.fire_button_vk,
    });
    let games: Vec<UiGame> = cfg
        .game_profiles
        .iter()
        .map(|gp| {
            let weapons: Vec<UiWeapon> = gp
                .weapons
                .iter()
                .map(|w| {
                    let binding = gp.bindings.iter().find(|b| b.weapon_id == w.id);
                    let hotkey_str = binding
                        .and_then(|b| b.hotkey.as_ref())
                        .map(hotkey_util::format_hotkey)
                        .unwrap_or_else(|| "—".into());
                    let bound = binding
                        .map(|b| b.enabled && b.hotkey.is_some())
                        .unwrap_or(false);
                    let is_armed =
                        armed_wid.as_ref().map(|aw| aw == &w.id).unwrap_or(false);
                    let mode = binding.map(|b| b.mode).unwrap_or_default();
                    let macro_id = binding.and_then(|b| b.macro_id.clone());
                    let preview = macro_id
                        .as_ref()
                        .and_then(|mid| cfg.macros.iter().find(|m| &m.id == mid))
                        .map(macro_preview);
                    UiWeapon {
                        id: w.id.clone(),
                        name: w.name.clone(),
                        class: w.class.clone(),
                        caliber: w.caliber.clone(),
                        rpm: w.rpm,
                        recoil: w.recoil,
                        bound,
                        armed: is_armed,
                        hotkey: hotkey_str,
                        mode,
                        macro_id,
                        macro_preview: preview,
                        icon_url: w.icon_url.clone(),
                    }
                })
                .collect();
            UiGame {
                id: gp.id.clone(),
                code: game_code(&gp.id),
                name: gp.display_name.clone(),
                sub: gp.subtitle.clone(),
                profile: gp.profile_label.clone(),
                logo_url: crate::embedded_weapon_icons::game_logo_data_url(&gp.id)
                    .map(|s| s.to_string()),
                weapons,
            }
        })
        .collect();

    UiConfig {
        master_enabled: cfg.master_enabled,
        active_game: cfg.active_game.clone(),
        fire_button_vk: cfg.fire_button_vk,
        fire_button_label: fire_label,
        armed_weapon_id: armed_wid,
        jitter_ms: cfg.jitter_ms,
        games,
        macros: cfg.macros.clone(),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub fn load_config(state: State<AppState>) -> Result<LoadResponse, String> {
    let cfg = state.config.read().map_err(|e| e.to_string())?;
    Ok(LoadResponse {
        ui: build_ui(&cfg, &state.armed),
        config: cfg.clone(),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn save_config(state: State<AppState>, config: AppConfig) -> Result<(), String> {
    validate_config(&config)?;
    {
        let mut w = state.config.write().map_err(|e| e.to_string())?;
        *w = config;
    }
    let snapshot = state.config.read().map_err(|e| e.to_string())?;
    config_manager::save(&snapshot).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignMacroPayload {
    pub game_id: String,
    pub weapon_id: String,
}

fn apply_imported_macro(
    state: &State<AppState>,
    imported: MacroDefinition,
    assign: Option<AssignMacroPayload>,
) -> Result<LoadResponse, String> {
    if imported.id.is_empty() {
        return Err("macro id required".into());
    }
    {
        let mut w = state.config.write().map_err(|e| e.to_string())?;
        w.macros.retain(|m| m.id != imported.id);
        w.macros.push(imported.clone());
        if let Some(a) = assign {
            let gp = w
                .game_profiles
                .iter_mut()
                .find(|g| g.id == a.game_id)
                .ok_or_else(|| "game not found".to_string())?;
            let binding = gp
                .bindings
                .iter_mut()
                .find(|b| b.weapon_id == a.weapon_id)
                .ok_or_else(|| "weapon not found".to_string())?;
            binding.macro_id = Some(imported.id.clone());
            binding.enabled = true;
            if binding.hotkey.is_none() {
                binding.hotkey = Some(crate::types::HotkeySpec {
                    modifiers: 0,
                    vk: 0x05,
                });
            }
        }
        validate_config(&w)?;
    }
    let cfg = state.config.read().map_err(|e| e.to_string())?;
    config_manager::save(&cfg).map_err(|e| e.to_string())?;
    Ok(LoadResponse {
        ui: build_ui(&cfg, &state.armed),
        config: cfg.clone(),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn import_macro_json(
    state: State<AppState>,
    json: String,
    assign: Option<AssignMacroPayload>,
) -> Result<LoadResponse, String> {
    let imported: MacroDefinition =
        serde_json::from_str(&json).map_err(|e| format!("invalid macro json: {e}"))?;
    apply_imported_macro(&state, imported, assign)
}

#[tauri::command(rename_all = "camelCase")]
pub fn import_macro_amc(
    state: State<AppState>,
    content_base64: String,
    file_name: String,
    assign: Option<AssignMacroPayload>,
) -> Result<LoadResponse, String> {
    let bytes = STANDARD
        .decode(content_base64.trim())
        .map_err(|e| format!("invalid base64: {e}"))?;
    let imported = crate::amc_import::parse_amc_bytes(&bytes, &file_name)?;
    apply_imported_macro(&state, imported, assign)
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_weapon_hotkey(
    state: State<AppState>,
    game_id: String,
    weapon_id: String,
    label: String,
) -> Result<LoadResponse, String> {
    let hk = hotkey_util::parse_hotkey_label(&label)?;
    {
        let mut w = state.config.write().map_err(|e| e.to_string())?;
        let gp = w
            .game_profiles
            .iter_mut()
            .find(|g| g.id == game_id)
            .ok_or_else(|| "game not found".to_string())?;
        let binding = gp
            .bindings
            .iter_mut()
            .find(|b| b.weapon_id == weapon_id)
            .ok_or_else(|| "weapon not found".to_string())?;
        binding.hotkey = Some(hk);
        binding.enabled = true;
        validate_config(&w)?;
    }
    let cfg = state.config.read().map_err(|e| e.to_string())?;
    config_manager::save(&cfg).map_err(|e| e.to_string())?;
    Ok(LoadResponse {
        ui: build_ui(&cfg, &state.armed),
        config: cfg.clone(),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn clear_weapon_hotkey(
    state: State<AppState>,
    game_id: String,
    weapon_id: String,
) -> Result<LoadResponse, String> {
    {
        let mut w = state.config.write().map_err(|e| e.to_string())?;
        let gp = w
            .game_profiles
            .iter_mut()
            .find(|g| g.id == game_id)
            .ok_or_else(|| "game not found".to_string())?;
        let binding = gp
            .bindings
            .iter_mut()
            .find(|b| b.weapon_id == weapon_id)
            .ok_or_else(|| "weapon not found".to_string())?;
        binding.hotkey = None;
        binding.enabled = false;
        validate_config(&w)?;
    }
    if let Ok(mut a) = state.armed.lock() {
        if let Some((_, wid)) = a.as_ref() {
            if wid == &weapon_id {
                *a = None;
            }
        }
    }
    let cfg = state.config.read().map_err(|e| e.to_string())?;
    config_manager::save(&cfg).map_err(|e| e.to_string())?;
    Ok(LoadResponse {
        ui: build_ui(&cfg, &state.armed),
        config: cfg.clone(),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_weapon_mode(
    state: State<AppState>,
    game_id: String,
    weapon_id: String,
    mode: ExecutionMode,
) -> Result<LoadResponse, String> {
    {
        let mut w = state.config.write().map_err(|e| e.to_string())?;
        let gp = w
            .game_profiles
            .iter_mut()
            .find(|g| g.id == game_id)
            .ok_or_else(|| "game not found".to_string())?;
        let binding = gp
            .bindings
            .iter_mut()
            .find(|b| b.weapon_id == weapon_id)
            .ok_or_else(|| "weapon not found".to_string())?;
        binding.mode = mode;
        validate_config(&w)?;
    }
    let cfg = state.config.read().map_err(|e| e.to_string())?;
    config_manager::save(&cfg).map_err(|e| e.to_string())?;
    Ok(LoadResponse {
        ui: build_ui(&cfg, &state.armed),
        config: cfg.clone(),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_active_game(state: State<AppState>, game_id: String) -> Result<LoadResponse, String> {
    {
        let mut w = state.config.write().map_err(|e| e.to_string())?;
        if !w.game_profiles.iter().any(|g| g.id == game_id) {
            return Err("game not found".into());
        }
        w.active_game = game_id;
        validate_config(&w)?;
    }
    if let Ok(mut a) = state.armed.lock() {
        *a = None;
    }
    let cfg = state.config.read().map_err(|e| e.to_string())?;
    config_manager::save(&cfg).map_err(|e| e.to_string())?;
    Ok(LoadResponse {
        ui: build_ui(&cfg, &state.armed),
        config: cfg.clone(),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_fire_button(state: State<AppState>, vk: u32) -> Result<LoadResponse, String> {
    let allowed: &[u32] = &[
        0x01, 0x02, 0x04, 0x05, 0x06, // mouse
        0x20, // space
        0x58, // X (some titles)
    ];
    if !allowed.contains(&vk) {
        return Err(
            "Unsupported primary fire key. Use LMB, RMB, MMB, Mouse 4/5, SPACE, or X.".into(),
        );
    }
    {
        let mut w = state.config.write().map_err(|e| e.to_string())?;
        w.fire_button_vk = vk;
        validate_config(&w)?;
    }
    let cfg = state.config.read().map_err(|e| e.to_string())?;
    config_manager::save(&cfg).map_err(|e| e.to_string())?;
    Ok(LoadResponse {
        ui: build_ui(&cfg, &state.armed),
        config: cfg.clone(),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn set_master_enabled(state: State<AppState>, on: bool) -> Result<LoadResponse, String> {
    {
        let mut w = state.config.write().map_err(|e| e.to_string())?;
        w.master_enabled = on;
        validate_config(&w)?;
    }
    if !on {
        state.engine.interrupt();
    }
    let cfg = state.config.read().map_err(|e| e.to_string())?;
    config_manager::save(&cfg).map_err(|e| e.to_string())?;
    Ok(LoadResponse {
        ui: build_ui(&cfg, &state.armed),
        config: cfg.clone(),
    })
}
