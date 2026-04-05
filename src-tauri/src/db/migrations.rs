use rusqlite::Connection;

struct Migration {
    version: i32,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_schema",
        sql: include_str!("../../migrations/001_initial_schema.sql"),
    },
    Migration {
        version: 2,
        name: "projection_triggers",
        sql: include_str!("../../migrations/002_projection_triggers.sql"),
    },
    Migration {
        version: 3,
        name: "extensions",
        sql: include_str!("../../migrations/003_extensions.sql"),
    },
];

pub fn run(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure schema_migrations table exists (it's created in migration 001,
    // but we need it to check which migrations have run)
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version  INTEGER PRIMARY KEY,
            name     TEXT    NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
        );",
    )?;

    for migration in MIGRATIONS {
        let already_applied: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM schema_migrations WHERE version = ?1",
                [migration.version],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !already_applied {
            log::info!(
                "Applying migration {}: {}",
                migration.version,
                migration.name
            );

            // Split by semicolons and execute each statement
            // (SQLite's execute_batch handles this, but triggers need careful handling)
            conn.execute_batch(migration.sql)?;

            conn.execute(
                "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
                rusqlite::params![migration.version, migration.name],
            )?;
        }
    }

    log::info!("All migrations applied");
    Ok(())
}
