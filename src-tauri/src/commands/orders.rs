use crate::db::Database;
use crate::events::{store, types::*};
use crate::sync::hlc::HybridLogicalClock;
use serde_json::json;
use tauri::State;

#[tauri::command]
pub fn get_order_items(
    db: State<'_, Database>,
    order_id: String,
) -> Result<Vec<OrderItem>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_order_items(&conn, &order_id)
}

#[tauri::command]
pub fn create_order(
    db: State<'_, Database>,
    payload: CreateOrderPayload,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::insert_order(&conn, &id, &payload)?;
    drop(conn);

    if let Err(e) = store::append_audit_log(
        &db,
        "local",
        "OrderCreated",
        json!({ "id": id, "customer_name": payload.customer_name }),
    ) {
        log::warn!("audit log write failed: {}", e);
    }

    Ok(id)
}

#[tauri::command]
pub fn update_order_status(
    db: State<'_, Database>,
    hlc: State<'_, HybridLogicalClock>,
    order_id: String,
    status: String,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::update_order_status(&conn, &order_id, &status)?;
    drop(conn);

    // Append CRDT-syncable event for order status changes
    store::append_event(
        &db,
        &hlc,
        &order_id,
        "order",
        "OrderStatusChanged",
        json!({ "status": status }),
    )?;

    if let Err(e) = store::append_audit_log(
        &db,
        "local",
        "OrderStatusChanged",
        json!({ "order_id": order_id, "status": status }),
    ) {
        log::warn!("audit log write failed: {}", e);
    }

    Ok(())
}

#[tauri::command]
pub fn get_orders(
    db: State<'_, Database>,
    status_filter: Option<String>,
) -> Result<Vec<Order>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_orders(&conn, status_filter.as_deref())
}

#[tauri::command]
pub fn get_order(db: State<'_, Database>, order_id: String) -> Result<Option<Order>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_order(&conn, &order_id)
}

#[tauri::command]
pub fn add_order_item(
    db: State<'_, Database>,
    payload: AddOrderItemPayload,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::insert_order_item(&conn, &id, &payload)?;
    drop(conn);

    if let Err(e) = store::append_audit_log(
        &db,
        "local",
        "OrderItemAdded",
        json!({ "order_id": payload.order_id, "item_id": payload.item_id, "quantity": payload.quantity }),
    ) {
        log::warn!("audit log write failed: {}", e);
    }

    Ok(id)
}

/// Update extended flower-mode fields on an order.
#[tauri::command]
pub fn update_order_extended(
    db: State<'_, Database>,
    order_id: String,
    customer_company: Option<String>,
    delivery_address: Option<String>,
    delivery_notes: Option<String>,
    pack_count_ordered: Option<i32>,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::update_order_extended(
        &conn,
        &order_id,
        customer_company.as_deref(),
        delivery_address.as_deref(),
        delivery_notes.as_deref(),
        pack_count_ordered,
    )
}

/// Mark a deadline as confirmed (user acknowledged overdue status).
#[tauri::command]
pub fn confirm_order_deadline(
    db: State<'_, Database>,
    order_id: String,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::confirm_order_deadline(&conn, &order_id)
}

/// Get orders whose deadline has passed and have not been confirmed yet.
/// Used on startup to show the overdue notification modal.
#[tauri::command]
pub fn get_overdue_unconfirmed_orders(
    db: State<'_, Database>,
) -> Result<Vec<Order>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_overdue_unconfirmed_orders(&conn)
}

/// Calculate pack shortages across all active orders.
/// Returns only orders where ordered_packs > available pkg_stock.
#[tauri::command]
pub fn check_order_shortages(
    db: State<'_, Database>,
) -> Result<Vec<OrderShortage>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::check_order_shortages(&conn)
}
