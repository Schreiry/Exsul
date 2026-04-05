use std::path::PathBuf;
use tauri::AppHandle;

#[tauri::command]
pub async fn export_backup(handle: AppHandle) -> Result<String, String> {
    let path = crate::sync::backup::create_encrypted_backup(&handle).await?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn import_backup(handle: AppHandle, path: String) -> Result<(), String> {
    crate::sync::backup::restore_from_backup(&handle, PathBuf::from(path)).await
}
