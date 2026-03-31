use std::collections::HashMap;

use tauri::State;

use crate::state::TauriState;

#[tauri::command]
pub fn get_setting(state: State<'_, TauriState>, key: String) -> Result<Option<String>, String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    codeforge_persistence::queries::get_setting(db.conn(), &key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings_batch(
    state: State<'_, TauriState>,
    keys: Vec<String>,
) -> Result<HashMap<String, String>, String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    codeforge_persistence::queries::get_settings_batch(db.conn(), &keys)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_setting(
    state: State<'_, TauriState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    codeforge_persistence::queries::set_setting(db.conn(), &key, &value)
        .map_err(|e| e.to_string())
}
