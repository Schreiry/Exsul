use std::path::PathBuf;
use tauri::AppHandle;

#[tauri::command]
pub async fn export_backup(handle: AppHandle) -> Result<String, String> {
    let backup_path = crate::sync::backup::create_encrypted_backup(&handle).await?;

    // Open OS file explorer with the backup selected
    #[cfg(target_os = "windows")]
    {
        let path_str = backup_path.to_string_lossy().to_string();
        std::process::Command::new("explorer.exe")
            .args(["/select,", &path_str])
            .spawn()
            .ok();
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &backup_path.to_string_lossy().as_ref()])
            .spawn()
            .ok();
    }
    #[cfg(target_os = "linux")]
    {
        let parent = backup_path.parent().unwrap_or(&backup_path);
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .ok();
    }

    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn import_backup(handle: AppHandle, path: String) -> Result<(), String> {
    crate::sync::backup::restore_from_backup(&handle, PathBuf::from(path)).await
}

/// Import backup from raw bytes (received from frontend file picker).
#[tauri::command]
pub async fn import_backup_data(handle: AppHandle, data: Vec<u8>) -> Result<(), String> {
    crate::sync::backup::restore_from_backup_data(&handle, data).await
}
