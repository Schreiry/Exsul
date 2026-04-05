use crate::db::Database;
use crate::events::types::{EventRecord, PriceRecord};
use tauri::State;

#[tauri::command]
pub fn get_events(
    db: State<'_, Database>,
    since: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<EventRecord>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_events(&conn, since.as_deref(), limit)
}

#[tauri::command]
pub fn get_price_history(
    db: State<'_, Database>,
    item_id: String,
) -> Result<Vec<PriceRecord>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_price_history(&conn, &item_id)
}
