//! Tauri commands exposed to the Svelte frontend.

use crate::lcu::{api, champions, models::*, Connection};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn set_settings(state: State<AppState>, settings: Settings) {
    *state.settings.lock().unwrap() = settings;
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

/// Dodge the current champ select immediately. Issues the LCU `quitV2` invoke,
/// which leaves champ select but keeps the client open. Returns an error string
/// the UI can surface so the user knows whether it actually fired.
#[tauri::command]
pub async fn dodge_now(state: State<'_, AppState>) -> Result<(), String> {
    let info = state.conn_info.lock().unwrap().clone();
    let (port, token) = info.ok_or("Not connected to the League client.")?;
    let conn = Connection::new(port, &token).map_err(|e| e.to_string())?;
    api::dodge(&conn)
        .await
        .map_err(|e| format!("League client rejected the dodge: {e}"))
}

/// Champion id<->name list for the auto-pick / auto-ban selectors.
#[tauri::command]
pub async fn get_champions() -> Result<Vec<Champion>, String> {
    champions::fetch().await.map_err(|e| e.to_string())
}
