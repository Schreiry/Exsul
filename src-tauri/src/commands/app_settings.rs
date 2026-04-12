use crate::db::Database;
use crate::events::types::AppSetting;
use tauri::State;

/// Get the value of a single setting by key.
#[tauri::command]
pub fn get_setting(db: State<'_, Database>, key: String) -> Result<Option<String>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_setting(&conn, &key)
}

/// Set (upsert) a setting value.
#[tauri::command]
pub fn set_setting(db: State<'_, Database>, key: String, value: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::set_setting(&conn, &key, &value)
}

/// Get all settings at once (for app initialization).
#[tauri::command]
pub fn get_all_settings(db: State<'_, Database>) -> Result<Vec<AppSetting>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_all_settings(&conn)
}
