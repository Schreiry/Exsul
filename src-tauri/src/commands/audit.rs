use crate::db::Database;
use crate::events::types::{AuditLog, AuditLogFilter};
use tauri::State;

#[tauri::command]
pub fn get_audit_logs(
    db: State<'_, Database>,
    filter: Option<AuditLogFilter>,
) -> Result<Vec<AuditLog>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let f = filter.unwrap_or(AuditLogFilter {
        user_id: None,
        action: None,
        since: None,
        until: None,
        limit: Some(500),
    });
    crate::db::queries::get_audit_logs(&conn, &f)
}
