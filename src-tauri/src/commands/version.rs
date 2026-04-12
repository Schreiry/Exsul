use crate::db::Database;
use crate::events::types::VersionInfo;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub fn get_version_info(app: AppHandle, db: State<'_, Database>) -> Result<VersionInfo, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let db_schema_version = crate::db::queries::get_schema_version(&conn);

    Ok(VersionInfo {
        app_version: app.package_info().version.to_string(),
        db_schema_version,
        min_compatible_version: "0.0.5".to_string(),
    })
}
