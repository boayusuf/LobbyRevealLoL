//! Tauri commands exposed to the Svelte frontend.

use crate::lcu::{champions, models::*};
use crate::state::AppState;
use std::sync::atomic::Ordering;
use tauri::State;

#[tauri::command]
pub fn set_settings(state: State<AppState>, settings: Settings) {
    *state.settings.lock().unwrap() = settings;
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

/// Request an immediate dodge; consumed by the engine on its next tick.
#[tauri::command]
pub fn dodge_now(state: State<AppState>) {
    state.dodge_requested.store(true, Ordering::SeqCst);
}

/// Champion id<->name list for the auto-pick / auto-ban selectors.
#[tauri::command]
pub async fn get_champions() -> Result<Vec<Champion>, String> {
    champions::fetch().await.map_err(|e| e.to_string())
}
