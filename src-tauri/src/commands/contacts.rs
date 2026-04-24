use crate::db::Database;
use crate::events::{store, types::*};
use crate::sync::hlc::HybridLogicalClock;
use serde_json::json;
use tauri::State;

// ============================================================
// Contacts — Phase E
// ============================================================
// Every mutating command writes BOTH an HLC-stamped event (for P2P
// replication) AND an audit log entry. Read commands do neither.
//
// Contacts are not gated by `$preset === 'flowers'` here on the Rust
// side — we rely on the frontend to only show the UI in flower mode.
// This keeps the backend preset-agnostic, which matches how inventory
// and other domains already behave.

#[tauri::command]
pub fn get_contacts(
    db: State<'_, Database>,
    search: Option<String>,
) -> Result<Vec<Contact>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::list_contacts(&conn, search.as_deref())
}

#[tauri::command]
pub fn get_contact(
    db: State<'_, Database>,
    contact_id: String,
) -> Result<Option<Contact>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_contact(&conn, &contact_id)
}

#[tauri::command]
pub fn create_contact(
    db: State<'_, Database>,
    hlc: State<'_, HybridLogicalClock>,
    payload: CreateContactPayload,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::insert_contact(&conn, &id, &payload)?;
    drop(conn);

    store::append_event(
        &db,
        &hlc,
        &id,
        "contact",
        "ContactCreated",
        json!({
            "id":   id,
            "name": payload.name,
            "phone": payload.phone,
            "email": payload.email,
        }),
    )?;

    if let Err(e) = store::append_audit_log(
        &db,
        "local",
        "ContactCreated",
        json!({ "id": id, "name": payload.name }),
    ) {
        log::warn!("audit log write failed: {}", e);
    }

    Ok(id)
}

#[tauri::command]
pub fn update_contact(
    db: State<'_, Database>,
    hlc: State<'_, HybridLogicalClock>,
    payload: UpdateContactPayload,
) -> Result<(), String> {
    let contact_id = payload.contact_id.clone();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::update_contact(&conn, &payload)?;
    drop(conn);

    let event_data = json!({
        "name":             payload.name,
        "surname":          payload.surname,
        "email":            payload.email,
        "phone":            payload.phone,
        "company":          payload.company,
        "notes":            payload.notes,
        "card_color":       payload.card_color,
        "clear_card_color": payload.clear_card_color,
    });

    store::append_event(
        &db,
        &hlc,
        &contact_id,
        "contact",
        "ContactUpdated",
        event_data.clone(),
    )?;

    if let Err(e) = store::append_audit_log(
        &db,
        "local",
        "ContactUpdated",
        json!({ "contact_id": contact_id, "fields": event_data }),
    ) {
        log::warn!("audit log write failed: {}", e);
    }

    Ok(())
}

#[tauri::command]
pub fn delete_contact(
    db: State<'_, Database>,
    hlc: State<'_, HybridLogicalClock>,
    contact_id: String,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::delete_contact(&conn, &contact_id)?;
    drop(conn);

    store::append_event(
        &db,
        &hlc,
        &contact_id,
        "contact",
        "ContactDeleted",
        json!({ "contact_id": contact_id }),
    )?;

    if let Err(e) = store::append_audit_log(
        &db,
        "local",
        "ContactDeleted",
        json!({ "contact_id": contact_id }),
    ) {
        log::warn!("audit log write failed: {}", e);
    }

    Ok(())
}

// ── Locations ─────────────────────────────────────────────────

#[tauri::command]
pub fn get_contact_locations(
    db: State<'_, Database>,
    contact_id: String,
) -> Result<Vec<ContactLocation>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::list_contact_locations(&conn, &contact_id)
}

#[tauri::command]
pub fn add_contact_location(
    db: State<'_, Database>,
    hlc: State<'_, HybridLogicalClock>,
    payload: CreateContactLocationPayload,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let contact_id = payload.contact_id.clone();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::insert_contact_location(&conn, &id, &payload)?;
    drop(conn);

    store::append_event(
        &db,
        &hlc,
        &contact_id,
        "contact",
        "ContactLocationAdded",
        json!({
            "id":         id,
            "contact_id": contact_id,
            "label":      payload.label,
            "address":    payload.address,
            "is_default": payload.is_default,
        }),
    )?;

    Ok(id)
}

#[tauri::command]
pub fn update_contact_location(
    db: State<'_, Database>,
    hlc: State<'_, HybridLogicalClock>,
    payload: UpdateContactLocationPayload,
) -> Result<(), String> {
    let location_id = payload.location_id.clone();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::update_contact_location(&conn, &payload)?;
    drop(conn);

    store::append_event(
        &db,
        &hlc,
        &location_id,
        "contact",
        "ContactLocationUpdated",
        json!({
            "location_id": location_id,
            "label":       payload.label,
            "address":     payload.address,
        }),
    )?;

    Ok(())
}

#[tauri::command]
pub fn delete_contact_location(
    db: State<'_, Database>,
    hlc: State<'_, HybridLogicalClock>,
    location_id: String,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::delete_contact_location(&conn, &location_id)?;
    drop(conn);

    store::append_event(
        &db,
        &hlc,
        &location_id,
        "contact",
        "ContactLocationDeleted",
        json!({ "location_id": location_id }),
    )?;

    Ok(())
}

#[tauri::command]
pub fn set_default_contact_location(
    db: State<'_, Database>,
    hlc: State<'_, HybridLogicalClock>,
    location_id: String,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::set_default_contact_location(&conn, &location_id)?;
    drop(conn);

    store::append_event(
        &db,
        &hlc,
        &location_id,
        "contact",
        "ContactLocationDefaulted",
        json!({ "location_id": location_id }),
    )?;

    Ok(())
}

// ── Orders attached to a contact ──────────────────────────────

#[tauri::command]
pub fn get_orders_for_contact(
    db: State<'_, Database>,
    contact_id: String,
) -> Result<Vec<Order>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_orders_for_contact(&conn, &contact_id)
}

// ── Photo upload (analogous to save_flower_photo) ─────────────

#[tauri::command]
pub fn save_contact_photo(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    contact_id: String,
    source_path: String,
) -> Result<String, String> {
    use std::fs;
    use std::path::Path;
    use tauri::Manager;

    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let photos_dir = app_dir.join("contact_photos");
    fs::create_dir_all(&photos_dir).map_err(|e| e.to_string())?;

    let src = Path::new(&source_path);
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("jpg");
    let filename = format!("{}.{}", contact_id, ext);
    let dest = photos_dir.join(&filename);
    fs::copy(src, &dest).map_err(|e| format!("Failed to copy photo: {}", e))?;

    let relative_path = format!("contact_photos/{}", filename);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::set_contact_photo_path(&conn, &contact_id, &relative_path)?;

    Ok(relative_path)
}

// ── Backfill helper (Phase E7) ────────────────────────────────

#[tauri::command]
pub fn backfill_contacts_from_orders(
    db: State<'_, Database>,
) -> Result<BackfillContactsResult, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let (created, linked) = crate::db::queries::backfill_contacts_from_orders(&conn)?;
    drop(conn);

    if let Err(e) = store::append_audit_log(
        &db,
        "local",
        "ContactsBackfilled",
        json!({ "created": created, "linked": linked }),
    ) {
        log::warn!("audit log write failed: {}", e);
    }

    Ok(BackfillContactsResult { created, linked })
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BackfillContactsResult {
    pub created: i64,
    pub linked: i64,
}
