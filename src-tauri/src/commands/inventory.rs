use crate::db::Database;
use crate::events::{store, types::*};
use crate::sync::hlc::HybridLogicalClock;
use serde_json::json;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

#[tauri::command]
pub fn add_item(
    db: State<'_, Database>,
    hlc: State<'_, HybridLogicalClock>,
    payload: CreateItemPayload,
) -> Result<String, String> {
    let item_id = uuid::Uuid::new_v4().to_string();

    let data = json!({
        "name": payload.name,
        "category": payload.category.clone().unwrap_or_else(|| "uncategorized".to_string()),
        "category_id": payload.category_id,
        "price": payload.price,
        "production_cost": payload.production_cost.unwrap_or(0.0),
        "initial_stock": payload.initial_stock.unwrap_or(0),
    });

    store::append_event(&db, &hlc, &item_id, "item", "ItemCreated", data.clone())?;

    if let Err(e) = store::append_audit_log(&db, "local", "ItemCreated", data) {
        log::warn!("audit log write failed: {}", e);
    }

    Ok(item_id)
}

#[tauri::command]
pub fn update_item(
    db: State<'_, Database>,
    hlc: State<'_, HybridLogicalClock>,
    payload: UpdateItemPayload,
) -> Result<(), String> {
    // card_color is UI metadata — write directly, not via events
    if let Some(ref color) = payload.card_color {
        let conn = db.conn.lock().map_err(|e| e.to_string())?;
        crate::db::queries::update_item_card_color(
            &conn,
            &payload.item_id,
            if color.is_empty() { None } else { Some(color.as_str()) },
        )?;
        drop(conn);
    }

    // Only emit event if there are business-relevant changes
    if payload.name.is_some()
        || payload.category.is_some()
        || payload.category_id.is_some()
        || payload.production_cost.is_some()
    {
        let data = json!({
            "name": payload.name,
            "category": payload.category,
            "category_id": payload.category_id,
            "production_cost": payload.production_cost,
        });

        store::append_event(&db, &hlc, &payload.item_id, "item", "ItemUpdated", data.clone())?;

        if let Err(e) = store::append_audit_log(&db, "local", "ItemUpdated", data) {
            log::warn!("audit log write failed: {}", e);
        }
    }

    Ok(())
}

#[tauri::command]
pub fn get_items(db: State<'_, Database>) -> Result<Vec<Item>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_items(&conn)
}

#[tauri::command]
pub fn get_item(db: State<'_, Database>, item_id: String) -> Result<Option<Item>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_item_by_id(&conn, &item_id)
}

#[tauri::command]
pub fn record_sale(
    db: State<'_, Database>,
    hlc: State<'_, HybridLogicalClock>,
    payload: RecordSalePayload,
) -> Result<(), String> {
    let data = json!({
        "quantity": payload.quantity,
        "sale_price": payload.sale_price,
    });

    store::append_event(&db, &hlc, &payload.item_id, "item", "SaleRecorded", data.clone())?;

    if let Err(e) = store::append_audit_log(&db, "local", "SaleRecorded", data) {
        log::warn!("audit log write failed: {}", e);
    }

    Ok(())
}

#[tauri::command]
pub fn adjust_stock(
    db: State<'_, Database>,
    hlc: State<'_, HybridLogicalClock>,
    payload: AdjustStockPayload,
) -> Result<(), String> {
    let data = json!({
        "delta": payload.delta,
    });

    store::append_event(&db, &hlc, &payload.item_id, "item", "StockAdjusted", data.clone())?;

    if let Err(e) = store::append_audit_log(&db, "local", "StockAdjusted", data) {
        log::warn!("audit log write failed: {}", e);
    }

    Ok(())
}

#[tauri::command]
pub fn change_price(
    db: State<'_, Database>,
    hlc: State<'_, HybridLogicalClock>,
    payload: ChangePricePayload,
) -> Result<(), String> {
    let data = json!({
        "new_price": payload.new_price,
    });

    store::append_event(&db, &hlc, &payload.item_id, "item", "PriceChanged", data.clone())?;

    if let Err(e) = store::append_audit_log(&db, "local", "PriceChanged", data) {
        log::warn!("audit log write failed: {}", e);
    }

    Ok(())
}

/// Delete an item directly from the projection (local-only, not event-sourced).
/// Also removes dependent order_items to avoid FK violations.
#[tauri::command]
pub fn delete_item(db: State<'_, Database>, item_id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::delete_item(&conn, &item_id)
}

/// Delete ALL items and their dependent records. Returns deleted count.
#[tauri::command]
pub fn delete_all_items(db: State<'_, Database>) -> Result<usize, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::delete_all_items(&conn)
}

/// Duplicate an item (copy with new UUID, reset sold_count & revenue).
#[tauri::command]
pub fn duplicate_item(db: State<'_, Database>, item_id: String) -> Result<String, String> {
    let new_id = Uuid::new_v4().to_string();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::duplicate_item(&conn, &item_id, &new_id)?;
    Ok(new_id)
}

/// Saves a base64-encoded image to disk then records the path on the item.
/// File I/O happens BEFORE locking the DB mutex.
#[tauri::command]
pub async fn save_item_image(
    handle: AppHandle,
    db: State<'_, Database>,
    item_id: String,
    base64_data: String,
) -> Result<String, String> {
    // Enforce 5 MB limit on base64 string (≈6.7 MB decoded)
    if base64_data.len() > 7 * 1024 * 1024 {
        return Err("Image exceeds 5 MB limit".to_string());
    }

    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let bytes = STANDARD
        .decode(&base64_data)
        .map_err(|e| format!("base64 decode error: {}", e))?;

    let app_dir = handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let images_dir = app_dir.join("images");
    tokio::fs::create_dir_all(&images_dir)
        .await
        .map_err(|e| e.to_string())?;

    let file_name = format!("{}.img", item_id);
    let file_path = images_dir.join(&file_name);
    tokio::fs::write(&file_path, &bytes)
        .await
        .map_err(|e| e.to_string())?;

    let relative_path = format!("images/{}", file_name);

    // Lock DB only for the UPDATE — after file write is done
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::set_item_image_path(&conn, &item_id, &relative_path)?;
    drop(conn);

    if let Err(e) = store::append_audit_log(
        &db,
        "local",
        "ItemImageSaved",
        json!({ "item_id": item_id, "path": relative_path }),
    ) {
        log::warn!("audit log write failed: {}", e);
    }

    Ok(relative_path)
}
