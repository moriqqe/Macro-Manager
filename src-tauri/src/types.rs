use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    Tap,
    #[default]
    Hold,
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct HotkeySpec {
    /// `MOD_*` bitmask (ALT=1, CONTROL=2, SHIFT=4).
    pub modifiers: u32,
    pub vk: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaponBinding {
    pub weapon_id: String,
    pub macro_id: Option<String>,
    #[serde(default)]
    pub hotkey: Option<HotkeySpec>,
    #[serde(default)]
    pub mode: ExecutionMode,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaponTemplate {
    pub id: String,
    pub name: String,
    pub class: String,
    pub caliber: String,
    #[serde(default)]
    pub rpm: Option<u32>,
    #[serde(default)]
    pub recoil: Option<f32>,
    /// HTTPS (e.g. RustLabs) or relative path `assets/...` for bundled SVG/PNG.
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameProfile {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub subtitle: String,
    #[serde(default)]
    pub profile_label: String,
    pub weapons: Vec<WeaponTemplate>,
    pub bindings: Vec<WeaponBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MacroStep {
    MouseDown { button: String },
    MouseUp { button: String },
    /// Relative mouse movement (same units as Win32 `SendInput` / `MOUSEEVENTF_MOVE`).
    MouseMoveRel { dx: i32, dy: i32 },
    KeyDown { vk: u32 },
    KeyUp { vk: u32 },
    Delay { ms: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroDefinition {
    pub id: String,
    pub name: String,
    #[serde(default = "default_macro_version")]
    pub version: u32,
    pub steps: Vec<MacroStep>,
}

fn default_macro_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default = "default_schema")]
    pub schema_version: u32,
    #[serde(default = "default_master")]
    pub master_enabled: bool,
    #[serde(default = "default_game")]
    pub active_game: String,
    /// VK for primary fire (default LMB `0x01`). Macro Hold/Toggle/Tap applies to this button only.
    #[serde(default = "default_fire_button_vk")]
    pub fire_button_vk: u32,
    #[serde(default)]
    pub jitter_ms: Option<(u64, u64)>,
    #[serde(default)]
    pub macros: Vec<MacroDefinition>,
    #[serde(default)]
    pub game_profiles: Vec<GameProfile>,
}

fn default_schema() -> u32 {
    1
}

fn default_master() -> bool {
    true
}

fn default_game() -> String {
    "pubg".to_string()
}

fn default_fire_button_vk() -> u32 {
    0x01
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiConfig {
    pub master_enabled: bool,
    pub active_game: String,
    pub fire_button_vk: u32,
    pub fire_button_label: String,
    #[serde(default)]
    pub armed_weapon_id: Option<String>,
    pub jitter_ms: Option<(u64, u64)>,
    pub games: Vec<UiGame>,
    pub macros: Vec<MacroDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiGame {
    pub id: String,
    pub code: String,
    pub name: String,
    pub sub: String,
    pub profile: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    pub weapons: Vec<UiWeapon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiWeapon {
    pub id: String,
    pub name: String,
    pub class: String,
    pub caliber: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpm: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recoil: Option<f32>,
    pub bound: bool,
    /// Selected weapon profile for the active game (macro runs on primary fire while set).
    #[serde(default)]
    pub armed: bool,
    pub hotkey: String,
    pub mode: ExecutionMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macro_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub macro_preview: Option<Vec<UiMacroStep>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiMacroStep {
    pub t: u64,
    pub kind: String,
    pub action: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadResponse {
    pub ui: UiConfig,
    pub config: AppConfig,
}

pub enum RunMode {
    Once,
    LoopUntilCancel,
}
