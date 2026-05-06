mod amc_import;
mod commands;
mod config_manager;
mod embedded_weapon_icons;
mod executor;
mod hotkey_util;
mod input_listener;
mod macro_engine;
mod profile_manager;
mod types;

use std::sync::Arc;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cfg = config_manager::load_or_default().expect("failed to load config");
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .manage(commands::AppState::new(cfg))
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::save_config,
            commands::import_macro_json,
            commands::import_macro_amc,
            commands::set_weapon_hotkey,
            commands::clear_weapon_hotkey,
            commands::set_weapon_mode,
            commands::set_active_game,
            commands::set_fire_button,
            commands::set_master_enabled,
        ])
        .setup(|app| {
            #[cfg(windows)]
            input_listener::init_app_handle(app.handle().clone());
            let st = app.state::<commands::AppState>();
            let rt = Arc::new(input_listener::InputRuntime {
                config: st.config.clone(),
                engine: st.engine.clone(),
                armed: st.armed.clone(),
            });
            input_listener::spawn(rt);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
