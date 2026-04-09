use crate::db::Database;
use crate::events::types::{TrustedNode, AddTrustedNodePayload};
use tauri::State;

#[tauri::command]
pub fn get_app_preset(db: State<'_, Database>) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let preset = crate::db::queries::get_local_config(&conn, "app_preset")?
        .unwrap_or_else(|| "balanced".to_string());
    Ok(preset)
}

#[tauri::command]
pub fn set_app_preset(db: State<'_, Database>, preset: String) -> Result<(), String> {
    // Validate
    let _ = match preset.as_str() {
        "flowers" | "ochokochi" | "balanced" => &preset,
        other => return Err(format!("Invalid preset: {}", other)),
    };
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::set_local_config(&conn, "app_preset", &preset)?;
    // Sync flower sorts → categories when switching to flowers preset
    if preset == "flowers" {
        if let Err(e) = crate::db::queries::sync_flower_sorts_to_categories(&conn) {
            log::warn!("sort→category sync failed: {}", e);
        }
    }
    Ok(())
}

// ── Trusted Nodes ────────────────────────────────────────────

#[tauri::command]
pub fn get_trusted_nodes(db: State<'_, Database>) -> Result<Vec<TrustedNode>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_trusted_nodes(&conn)
}

#[tauri::command]
pub fn add_trusted_node(
    db: State<'_, Database>,
    payload: AddTrustedNodePayload,
) -> Result<(), String> {
    if payload.node_id.trim().is_empty() {
        return Err("node_id cannot be empty".to_string());
    }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::insert_trusted_node(&conn, &payload)
}

#[tauri::command]
pub fn remove_trusted_node(db: State<'_, Database>, node_id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::delete_trusted_node(&conn, &node_id)
}
