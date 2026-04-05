use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use rand::RngCore;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Derive a 256-bit key from a passphrase using Argon2id.
fn derive_key(passphrase: &[u8], salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    argon2::Argon2::default()
        .hash_password_into(passphrase, salt, &mut key)
        .expect("key derivation failed");
    key
}

/// Generate a random 16-byte salt encoded as hex.
fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

pub async fn create_encrypted_backup(handle: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let db_path = app_dir.join("exsul.db");
    let backup_dir = app_dir.join("backups");
    let backup_path = backup_dir.join(format!(
        "exsul-{}.bak",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    ));

    if !db_path.exists() {
        return Err("Database file not found".to_string());
    }

    let plaintext = tokio::fs::read(&db_path)
        .await
        .map_err(|e| e.to_string())?;

    // TODO: In production, derive passphrase from user input or stored config
    let passphrase = b"exsul-default-backup-key";
    let salt = generate_salt();
    let key = derive_key(passphrase, &salt);

    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("cipher init: {e}"))?;
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|e| format!("encryption failed: {e}"))?;

    // Format: salt(16B) || nonce(12B) || ciphertext
    tokio::fs::create_dir_all(&backup_dir)
        .await
        .map_err(|e| e.to_string())?;

    let mut output = Vec::with_capacity(16 + 12 + ciphertext.len());
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&ciphertext);

    tokio::fs::write(&backup_path, output)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("Backup created at {:?}", backup_path);
    Ok(backup_path)
}

pub async fn restore_from_backup(
    handle: &AppHandle,
    backup_path: PathBuf,
) -> Result<(), String> {
    let app_dir = handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let db_path = app_dir.join("exsul.db");

    let data = tokio::fs::read(&backup_path)
        .await
        .map_err(|e| e.to_string())?;

    if data.len() < 28 {
        return Err("Backup file too small".to_string());
    }

    let salt = &data[..16];
    let nonce_bytes = &data[16..28];
    let ciphertext = &data[28..];

    let passphrase = b"exsul-default-backup-key";
    let key = derive_key(passphrase, salt);

    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| format!("cipher init: {e}"))?;
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("decryption failed: {e}"))?;

    tokio::fs::write(&db_path, plaintext)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("Backup restored from {:?}", backup_path);
    Ok(())
}
