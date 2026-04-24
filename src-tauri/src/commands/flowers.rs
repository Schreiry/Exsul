use crate::db::Database;
use crate::events::types::{
    CreatePackAssignmentPayload, FlowerConstants, FlowerSort, CreateFlowerSortPayload,
    OrderWaitingForSort, PackAssignment, PackageResult, PackagingLogEntry, UpdateFlowerSortPayload,
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
///
/// When `order_id` is supplied, the resulting `packaging_log` row is
/// immediately linked to that order so the warehouse ↔ orders chain is
/// queryable in both directions. Callers that just package free stock
/// (no order yet) simply pass `None` and the column stays NULL.
#[tauri::command]
pub fn package_flowers(
    db: State<'_, Database>,
    sort_id: String,
    pack_count: i32,
    order_id: Option<String>,
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
    crate::db::queries::package_flowers(
        &conn,
        &log_id,
        &sort_id,
        pack_count,
        flowers_per_pack,
        order_id.as_deref(),
    )
}

// ── Packaging Log ────────────────────────────────────────────

#[tauri::command]
pub fn get_packaging_log(
    db: State<'_, Database>,
    limit: Option<i64>,
) -> Result<Vec<PackagingLogEntry>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_packaging_log(&conn, limit)
}

#[tauri::command]
pub fn get_packaging_log_by_sort(
    db: State<'_, Database>,
    sort_id: String,
    limit: Option<i64>,
) -> Result<Vec<PackagingLogEntry>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_packaging_log_by_sort(&conn, &sort_id, limit)
}

/// Packaging entries linked to a specific order. Used by the order print
/// pipeline as the authoritative source of sort/pack/stems when `order_items`
/// is missing or incomplete (legacy orders, or orders created before
/// packaging→order linking landed).
#[tauri::command]
pub fn get_packaging_log_by_order(
    db: State<'_, Database>,
    order_id: String,
) -> Result<Vec<PackagingLogEntry>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_packaging_log_by_order(&conn, &order_id)
}

/// Active orders (`pending` or `in_progress`) that have line items referencing
/// the given sort, annotated with reserved-vs-ordered pack counts so the
/// greenhouse UI can surface reservations and shortages on the sort card.
#[tauri::command]
pub fn get_orders_waiting_for_sort(
    db: State<'_, Database>,
    sort_id: String,
) -> Result<Vec<OrderWaitingForSort>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_orders_waiting_for_sort(&conn, &sort_id)
}

/// Delete a packaging_log entry and roll back the stock movement
/// (pkg_stock -= pack_count, raw_stock += stems_used). Records the inverse
/// move in `greenhouse_harvest_log` as a correction. Fails if pkg_stock
/// would go negative (i.e. some of those packs have already shipped).
#[tauri::command]
pub fn delete_packaging_entry(db: State<'_, Database>, id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::delete_packaging_entry(&conn, &id)
}

/// Delete every packaging_log entry, rolling back each stock movement.
/// Aborts atomically if any single rollback would underflow pkg_stock.
#[tauri::command]
pub fn delete_all_packaging(db: State<'_, Database>) -> Result<i64, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::delete_all_packaging(&conn)
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

/// Remove a pack-assignment row. pkg_stock stays unchanged: this only
/// releases the reservation — the physical packs remain on the warehouse.
#[tauri::command]
pub fn delete_pack_assignment(db: State<'_, Database>, id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::delete_pack_assignment(&conn, &id)
}
