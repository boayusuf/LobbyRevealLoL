// Several LCU model fields are deserialized for completeness / future phases
// even when not yet read; don't warn about those.
#![allow(dead_code)]

mod commands;
mod engine;
mod lcu;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .setup(|app| {
            // Spawn the LCU polling engine for the lifetime of the app.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(engine::run(handle));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::set_settings,
            commands::get_settings,
            commands::dodge_now,
            commands::get_champions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
