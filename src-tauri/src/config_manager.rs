use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

use crate::profile_manager;
use crate::types::AppConfig;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("MacrosManager")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

pub fn load_or_default() -> Result<AppConfig, ConfigError> {
    let path = config_path();
    if !path.exists() {
        let cfg = profile_manager::default_config();
        save(&cfg)?;
        return Ok(cfg);
    }
    let mut f = fs::File::open(&path)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    let mut cfg: AppConfig = serde_json::from_str(&s)?;
    let mut need_save = profile_manager::apply_default_weapon_icons(&mut cfg);
    if profile_manager::sync_weapons_to_defaults(&mut cfg) {
        need_save = true;
    }
    if need_save {
        let _ = save(&cfg);
    }
    Ok(cfg)
}

pub fn save(cfg: &AppConfig) -> Result<(), ConfigError> {
    let dir = config_dir();
    fs::create_dir_all(&dir)?;
    let path = config_path();
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(cfg)?;
    let mut f = fs::File::create(&tmp)?;
    f.write_all(json.as_bytes())?;
    f.sync_all()?;
    drop(f);
    fs::rename(&tmp, &path)?;
    Ok(())
}
