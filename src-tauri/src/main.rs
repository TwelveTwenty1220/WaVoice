#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod commands;
mod config;
mod hotkeys;
mod library;
mod state;

use parking_lot::RwLock;
use std::sync::Arc;

use config::{default_data_dir, Config};
use hotkeys::HotkeyService;
use library::Library;
use state::AppState;

fn main() {
    tracing_subscriber::fmt::init();
    let data_dir = default_data_dir();
    std::fs::create_dir_all(&data_dir).ok();
    let config = Config::load(&data_dir.join("config.json"));
    let library = Arc::new(RwLock::new(Library::load(&data_dir.join("library.json"))));
    let hotkeys = Arc::new(HotkeyService::new().expect("hotkey service init"));
    let app_state = AppState::new(config, library, hotkeys.clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(app_state)
        .setup(move |_app| {
            let hk = hotkeys.clone();
            std::thread::spawn(move || loop {
                hk.poll();
                std::thread::sleep(std::time::Duration::from_millis(50));
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_devices,
            commands::get_config,
            commands::save_config,
            commands::start_engine,
            commands::stop_engine,
            commands::engine_status,
            commands::play_track,
            commands::stop_all,
            commands::add_library_file,
            commands::remove_library_file,
            commands::list_library,
            commands::set_hotkey,
            commands::get_meters,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
