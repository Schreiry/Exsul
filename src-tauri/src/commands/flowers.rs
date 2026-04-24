use crate::db::Database;
use crate::events::types::{
    AddOrderItemPayload, CreateFlowerSortPayload, CreateOrderPayload, CreatePackAssignmentPayload,
    FlowerConstants, FlowerSort, OrderWaitingForSort, PackAssignment, PackageResult,
    PackageWithOrderPayload, PackageWithOrderResult, PackagingLogEntry, UpdateFlowerSortPayload,
};
use rusqlite::params;
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

/// Atomic variant of `package_flowers` that also creates (or skips) the
/// associated order, order_item and pack_assignment in a single SQLite
/// transaction. Closes the cascade where a mid-chain failure used to leave
/// `packaging_log` written but `order_items` missing — observed as "FK
/// constraint failed", total_amount = 0, and empty "Linked packs" in the UI.
///
/// Behaviour:
///   - `customer_name` empty → pure packaging, no order created (same result
///     shape, `order_id = None`).
///   - `customer_name` set  → creates order + extended fields + packaging +
///     order_item + pack_assignment + recalculates total. All-or-nothing.
#[tauri::command]
pub fn package_flowers_with_order(
    db: State<'_, Database>,
    payload: PackageWithOrderPayload,
) -> Result<PackageWithOrderResult, String> {
    if payload.pack_count <= 0 {
        return Err("pack_count must be > 0".to_string());
    }
    if payload.sort_id.trim().is_empty() {
        return Err("sort_id is required".to_string());
    }

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Resolve effective flowers_per_pack up front (same logic as package_flowers)
    // so we can use it both for packaging and for the order_item.
    let override_fpp: Option<i32> = conn
        .query_row(
            "SELECT flowers_per_pack_override FROM flower_sorts WHERE id = ?1",
            [&payload.sort_id],
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

    let has_customer = payload
        .customer_name
        .as_ref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    let order_id_opt: Option<String> = if has_customer {
        Some(Uuid::new_v4().to_string())
    } else {
        None
    };
    let log_id = Uuid::new_v4().to_string();

    conn.execute("BEGIN IMMEDIATE", [])
        .map_err(|e| e.to_string())?;

    let work: Result<PackageWithOrderResult, String> = (|| {
        // 1. Order + extended details (only when customer data was supplied).
        if let Some(ref order_id) = order_id_opt {
            let customer_name = payload
                .customer_name
                .as_ref()
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            let create_payload = CreateOrderPayload {
                customer_name,
                customer_email: payload.customer_email.clone(),
                customer_phone: payload.customer_phone.clone(),
                deadline: payload.deadline.clone(),
                notes: payload.notes.clone(),
                card_color: payload.card_color.clone(),
                contact_id: payload.contact_id.clone(),
                contact_location_id: payload.contact_location_id.clone(),
            };
            crate::db::queries::insert_order(&conn, order_id, &create_payload)?;

            crate::db::queries::update_order_extended(
                &conn,
                order_id,
                None,
                payload.delivery_address.as_deref(),
                None,
                Some(payload.pack_count),
            )?;
        }

        // 2. Packaging (stock movement + packaging_log row linked to the order).
        let pkg_result = crate::db::queries::package_flowers(
            &conn,
            &log_id,
            &payload.sort_id,
            payload.pack_count,
            flowers_per_pack,
            order_id_opt.as_deref(),
        )?;

        // 3. order_item + pack_assignment when an order was created.
        if let Some(ref order_id) = order_id_opt {
            let oi_id = Uuid::new_v4().to_string();
            let oi_payload = AddOrderItemPayload {
                order_id: order_id.clone(),
                item_id: payload.sort_id.clone(),
                quantity: payload.pack_count,
                unit_price: payload.price_per_pack,
                specifications: None,
                pack_count: Some(payload.pack_count),
                stems_per_pack: Some(flowers_per_pack),
                sort_id: Some(payload.sort_id.clone()),
            };
            // insert_order_item also seeds the items-shadow row and recomputes
            // order.total_amount, so no extra plumbing is needed here.
            crate::db::queries::insert_order_item(&conn, &oi_id, &oi_payload)?;

            let pa_id = Uuid::new_v4().to_string();
            let pa_payload = CreatePackAssignmentPayload {
                sort_id: payload.sort_id.clone(),
                order_id: Some(order_id.clone()),
                pack_count: payload.pack_count,
                stems_per_pack: flowers_per_pack,
                note: None,
            };
            crate::db::queries::insert_pack_assignment(&conn, &pa_id, &pa_payload)?;
        }

        Ok(PackageWithOrderResult {
            order_id: order_id_opt.clone(),
            packaging_log_id: log_id.clone(),
            new_raw_stock: pkg_result.new_raw_stock,
            new_pkg_stock: pkg_result.new_pkg_stock,
            stems_used: pkg_result.stems_used,
            packs_created: pkg_result.packs_created,
        })
    })();

    match work {
        Ok(result) => {
            conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
            Ok(result)
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", params![]);
            Err(e)
        }
    }
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
