use crate::db::Database;
use crate::events::{store, types::*};
use serde_json::json;
use tauri::State;

#[tauri::command]
pub fn create_category(
    db: State<'_, Database>,
    payload: CreateCategoryPayload,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::insert_category(&conn, &id, &payload)?;
    drop(conn);

    if let Err(e) = store::append_audit_log(
        &db,
        "local",
        "CategoryCreated",
        json!({ "id": id, "name": payload.name }),
    ) {
        log::warn!("audit log write failed: {}", e);
    }

    Ok(id)
}

#[tauri::command]
pub fn get_categories(db: State<'_, Database>) -> Result<Vec<Category>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_categories(&conn)
}

#[tauri::command]
pub fn update_category(
    db: State<'_, Database>,
    payload: UpdateCategoryPayload,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::update_category(&conn, &payload)?;
    drop(conn);

    if let Err(e) = store::append_audit_log(
        &db,
        "local",
        "CategoryUpdated",
        json!({ "id": payload.id }),
    ) {
        log::warn!("audit log write failed: {}", e);
    }

    Ok(())
}

#[tauri::command]
pub fn delete_category(db: State<'_, Database>, id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::delete_category(&conn, &id)?;
    drop(conn);

    if let Err(e) = store::append_audit_log(
        &db,
        "local",
        "CategoryDeleted",
        json!({ "id": id }),
    ) {
        log::warn!("audit log write failed: {}", e);
    }

    Ok(())
}
