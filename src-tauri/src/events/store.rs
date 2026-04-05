use crate::db::Database;
use crate::events::types::EventRecord;
use crate::sync::hlc::HybridLogicalClock;
use rusqlite::params;

pub fn append_event(
    db: &Database,
    hlc: &HybridLogicalClock,
    aggregate_id: &str,
    aggregate_type: &str,
    event_type: &str,
    data: serde_json::Value,
) -> Result<EventRecord, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let node_id = hlc.node_id().to_string();
    let hlc_timestamp = hlc.now();

    let version = crate::db::queries::get_next_version(&conn, aggregate_id, &node_id)?;

    let data_str = serde_json::to_string(&data).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO events (aggregate_id, aggregate_type, event_type, data, hlc_timestamp, node_id, version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![aggregate_id, aggregate_type, event_type, data_str, hlc_timestamp, node_id, version],
    )
    .map_err(|e| e.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(EventRecord {
        id: Some(id),
        aggregate_id: aggregate_id.to_string(),
        aggregate_type: aggregate_type.to_string(),
        event_type: event_type.to_string(),
        data,
        hlc_timestamp,
        node_id,
        version,
        created_at: None,
    })
}

/// Appends a record to the audit_logs table. Non-fatal: errors are returned
/// but callers should log-and-continue rather than propagating.
pub fn append_audit_log(
    db: &Database,
    user_id: &str,
    action: &str,
    payload: serde_json::Value,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    crate::db::queries::insert_audit_log(&conn, user_id, action, &payload)
}

pub fn insert_remote_event(
    db: &Database,
    hlc: &HybridLogicalClock,
    event: &EventRecord,
) -> Result<bool, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Merge the remote HLC
    hlc.recv(&event.hlc_timestamp);

    let data_str = serde_json::to_string(&event.data).map_err(|e| e.to_string())?;

    // INSERT OR IGNORE: if the event already exists (same aggregate_id, node_id, version), skip it
    let rows = conn
        .execute(
            "INSERT OR IGNORE INTO events (aggregate_id, aggregate_type, event_type, data, hlc_timestamp, node_id, version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                event.aggregate_id,
                event.aggregate_type,
                event.event_type,
                data_str,
                event.hlc_timestamp,
                event.node_id,
                event.version
            ],
        )
        .map_err(|e| e.to_string())?;

    if rows > 0 {
        // Update sync_state for this peer
        conn.execute(
            "INSERT OR REPLACE INTO sync_state (peer_node_id, last_hlc, last_event_id, last_synced_at)
             VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%f', 'now'))",
            params![event.node_id, event.hlc_timestamp, conn.last_insert_rowid()],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(rows > 0)
}
