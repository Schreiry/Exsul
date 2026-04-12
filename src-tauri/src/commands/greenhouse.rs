use crate::db::Database;
use crate::events::types::HarvestLogEntry;
use tauri::State;

/// Save a photo for a flower sort.
/// Copies the source file to app_data_dir/flower_photos/{sort_id}.{ext}
/// Updates the photo_path in the DB and returns the relative path.
#[tauri::command]
pub fn save_flower_photo(
    app: tauri::AppHandle,
    db: State<'_, Database>,
    sort_id: String,
    source_path: String,
) -> Result<String, String> {
    use std::fs;
    use std::path::Path;
    use tauri::Manager;

    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let photos_dir = app_dir.join("flower_photos");
    fs::create_dir_all(&photos_dir).map_err(|e| e.to_string())?;

    let src = Path::new(&source_path);
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg");
    let filename = format!("{}.{}", sort_id, ext);
    let dest = photos_dir.join(&filename);

    fs::copy(src, &dest).map_err(|e| format!("Failed to copy photo: {}", e))?;

    let relative_path = format!("flower_photos/{}", filename);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::set_flower_photo_path(&conn, &sort_id, &relative_path)?;

    Ok(relative_path)
}

/// Log a raw stock movement for a flower sort.
/// delta > 0 = stems added, delta < 0 = stems removed.
/// Also adjusts raw_stock on flower_sorts accordingly.
#[tauri::command]
pub fn log_greenhouse_harvest(
    db: State<'_, Database>,
    sort_id: String,
    delta: i32,
    reason: String,
    note: Option<String>,
) -> Result<(), String> {
    let id = uuid::Uuid::new_v4().to_string();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Validate reason
    match reason.as_str() {
        "manual" | "packaged" | "correction" => {}
        other => return Err(format!("Invalid reason: '{}'. Use manual|packaged|correction", other)),
    }

    // Insert harvest log entry (also updates total_harvested for positive delta)
    crate::db::queries::insert_harvest_log(
        &conn,
        &id,
        &sort_id,
        delta,
        &reason,
        note.as_deref(),
    )?;

    // Adjust raw_stock (clamped to 0 via MAX)
    conn.execute(
        "UPDATE flower_sorts SET
             raw_stock  = MAX(0, raw_stock + ?2),
             updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
         WHERE id = ?1",
        rusqlite::params![sort_id, delta],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Retrieve harvest log entries, optionally filtered by sort.
#[tauri::command]
pub fn get_harvest_log(
    db: State<'_, Database>,
    sort_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<HarvestLogEntry>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::get_harvest_log(&conn, sort_id.as_deref(), limit)
}
