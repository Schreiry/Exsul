use crate::db::Database;
use crate::events::types::{
    FlowerConstants, FlowerSort, CreateFlowerSortPayload, UpdateFlowerSortPayload,
};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn get_flower_sorts(db: State<'_, Database>) -> Result<Vec<FlowerSort>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_flower_sorts(&conn)
}

#[tauri::command]
pub fn create_flower_sort(
    db: State<'_, Database>,
    payload: CreateFlowerSortPayload,
) -> Result<String, String> {
    if payload.name.trim().is_empty() {
        return Err("name cannot be empty".to_string());
    }
    let id = Uuid::new_v4().to_string();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::insert_flower_sort(&conn, &id, &payload)?;
    Ok(id)
}

#[tauri::command]
pub fn update_flower_sort(
    db: State<'_, Database>,
    payload: UpdateFlowerSortPayload,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::update_flower_sort(&conn, &payload)
}

#[tauri::command]
pub fn delete_flower_sort(db: State<'_, Database>, id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::delete_flower_sort(&conn, &id)
}

#[tauri::command]
pub fn adjust_flower_stock(
    db: State<'_, Database>,
    id: String,
    raw_delta: i32,
    pkg_delta: i32,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::adjust_flower_stock(&conn, &id, raw_delta, pkg_delta)
}

// ── Constants ────────────────────────────────────────────────

#[tauri::command]
pub fn get_flower_constants(db: State<'_, Database>) -> Result<FlowerConstants, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_flower_constants(&conn)
}

#[tauri::command]
pub fn set_flower_constants(
    db: State<'_, Database>,
    constants: FlowerConstants,
) -> Result<(), String> {
    if constants.flowers_per_pack <= 0.0 {
        return Err("flowers_per_pack must be > 0".to_string());
    }
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::set_flower_constants(&conn, &constants)
}
