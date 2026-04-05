pub mod fixup;
pub mod migrations;
pub mod queries;

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct Database {
    pub conn: Mutex<Connection>,
}

pub fn init(handle: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let app_dir = handle.path().app_data_dir()?;
    std::fs::create_dir_all(&app_dir)?;
    let db_path = app_dir.join("exsul.db");

    let conn = Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON;")?;

    migrations::run(&conn)?;
    fixup::migrate_categories(&conn)?;

    // Ensure a node_id exists
    let node_id: Option<String> = conn
        .query_row(
            "SELECT value FROM local_config WHERE key = 'node_id'",
            [],
            |row| row.get(0),
        )
        .ok();

    if node_id.is_none() {
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO local_config (key, value) VALUES ('node_id', ?1)",
            [&id],
        )?;
        log::info!("Generated node_id: {}", id);
    }

    handle.manage(Database {
        conn: Mutex::new(conn),
    });

    log::info!("Database initialized at {:?}", db_path);
    Ok(())
}

pub fn get_db_path(handle: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = handle.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(app_dir.join("exsul.db"))
}
