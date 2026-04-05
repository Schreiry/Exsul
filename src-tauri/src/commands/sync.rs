use crate::db::Database;
use crate::events::store;
use crate::events::types::{EventRecord, SyncPeer};
use crate::sync::hlc::HybridLogicalClock;
use tauri::State;

#[tauri::command]
pub fn get_sync_state(db: State<'_, Database>) -> Result<Vec<SyncPeer>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_sync_state(&conn)
}

#[tauri::command]
pub fn merge_remote_events(
    db: State<'_, Database>,
    hlc: State<'_, HybridLogicalClock>,
    events: Vec<EventRecord>,
) -> Result<u32, String> {
    let mut merged = 0u32;
    for event in &events {
        if store::insert_remote_event(&db, &hlc, event)? {
            merged += 1;
        }
    }
    Ok(merged)
}

#[tauri::command]
pub fn get_node_id(db: State<'_, Database>) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_node_id(&conn)
}
