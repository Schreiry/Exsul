use crate::db::Database;
use crate::events::{store, types::*};
use crate::sync::hlc::HybridLogicalClock;
use serde_json::json;
use tauri::State;

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
