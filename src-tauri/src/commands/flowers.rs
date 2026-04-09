use crate::db::Database;
use crate::events::types::{
    CreatePackAssignmentPayload, FlowerConstants, FlowerSort, CreateFlowerSortPayload,
    PackAssignment, PackageResult, UpdateFlowerSortPayload,
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

// ── ERP: Packaging ───────────────────────────────────────────

/// Consume raw stems from a flower sort and produce packs.
/// Returns the updated stock values so the frontend can optimistically update.
#[tauri::command]
pub fn package_flowers(
    db: State<'_, Database>,
    sort_id: String,
    pack_count: i32,
) -> Result<PackageResult, String> {
    if pack_count <= 0 {
        return Err("pack_count must be > 0".to_string());
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Determine effective flowers_per_pack: use per-sort override or global constant
    let override_fpp: Option<i32> = conn
        .query_row(
            "SELECT flowers_per_pack_override FROM flower_sorts WHERE id = ?1",
            [&sort_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let flowers_per_pack: i32 = match override_fpp {
        Some(v) if v > 0 => v,
        _ => {
            let global: f64 = conn
                .query_row(
                    "SELECT value FROM flower_constants WHERE key = 'flowers_per_pack'",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(10.0);
            global.round() as i32
        }
    };

    if flowers_per_pack <= 0 {
        return Err("flowers_per_pack must be > 0".to_string());
    }

    let log_id = Uuid::new_v4().to_string();
    crate::db::queries::package_flowers(&conn, &log_id, &sort_id, pack_count, flowers_per_pack)
}

// ── Pack Assignments (Task 9) ─────────────────────────────────

#[tauri::command]
pub fn create_pack_assignment(
    db: State<'_, Database>,
    payload: CreatePackAssignmentPayload,
) -> Result<String, String> {
    if payload.pack_count <= 0 {
        return Err("pack_count must be > 0".to_string());
    }
    if payload.stems_per_pack <= 0 {
        return Err("stems_per_pack must be > 0".to_string());
    }
    let id = Uuid::new_v4().to_string();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::insert_pack_assignment(&conn, &id, &payload)?;
    Ok(id)
}

#[tauri::command]
pub fn get_pack_assignments(
    db: State<'_, Database>,
    order_id: Option<String>,
) -> Result<Vec<PackAssignment>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_pack_assignments(&conn, order_id.as_deref())
}

#[tauri::command]
pub fn update_pack_status(
    db: State<'_, Database>,
    id: String,
    status: String,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::update_pack_status(&conn, &id, &status)
}
