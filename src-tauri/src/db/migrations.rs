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
    Migration {
        version: 4,
        name: "preset_and_flowers",
        sql: include_str!("../../migrations/004_preset_and_flowers.sql"),
    },
    Migration {
        version: 5,
        name: "flower_erp",
        sql: include_str!("../../migrations/005_flower_erp.sql"),
    },
    Migration {
        version: 6,
        name: "version_compat",
        sql: include_str!("../../migrations/006_version_compat.sql"),
    },
    Migration {
        version: 7,
        name: "category_sort_bridge",
        sql: include_str!("../../migrations/007_category_sort_bridge.sql"),
    },
    Migration {
        version: 8,
        name: "item_card_color",
        sql: include_str!("../../migrations/008_item_card_color.sql"),
    },
    Migration {
        version: 9,
        name: "packing_orders",
        sql: include_str!("../../migrations/009_packing_orders.sql"),
    },
    Migration {
        version: 10,
        name: "greenhouse_warehouse",
        sql: include_str!("../../migrations/010_greenhouse_warehouse.sql"),
    },
    Migration {
        version: 11,
        name: "orders_extended",
        sql: include_str!("../../migrations/011_orders_extended.sql"),
    },
    Migration {
        version: 12,
        name: "app_settings",
        sql: include_str!("../../migrations/012_app_settings.sql"),
    },
    Migration {
        version: 13,
        name: "data_reset",
        sql: include_str!("../../migrations/013_data_reset.sql"),
    },
];

/// Execute a migration's SQL idempotently:
/// - Wraps everything in a single transaction (atomic: all-or-nothing).
/// - Executes each SQL statement individually.
/// - "duplicate column name" and "already exists" errors are treated as
///   warnings rather than fatal — the column/table is already present, which
///   is fine when recovering from a previous partial run.
/// - Any other error rolls back the transaction and propagates.
fn apply_migration_safe(
    conn: &Connection,
    sql: &str,
    version: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute_batch("BEGIN IMMEDIATE;")?;

    for raw in sql.split(';') {
        let stmt = raw.trim();

        // Skip blanks and pure comment blocks
        if stmt.is_empty() || stmt.lines().all(|l| l.trim().is_empty() || l.trim().starts_with("--")) {
            continue;
        }

        let executable = format!("{};", stmt);
        if let Err(e) = conn.execute_batch(&executable) {
            let msg = e.to_string().to_lowercase();

            // Idempotent: skip if the object already exists
            if msg.contains("duplicate column name")
                || (msg.contains("already exists") && !msg.contains("syntax"))
            {
                log::warn!(
                    "Migration {}: skipping (already applied): {}",
                    version,
                    // Print only the first meaningful line to keep logs short
                    stmt.lines()
                        .find(|l| !l.trim().is_empty() && !l.trim().starts_with("--"))
                        .unwrap_or(stmt)
                        .trim()
                );
            } else {
                // Real error — roll back and surface it
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(Box::new(e));
            }
        }
    }

    conn.execute_batch("COMMIT;")?;
    Ok(())
}

pub fn run(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure schema_migrations table exists before we query it
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version    INTEGER PRIMARY KEY,
            name       TEXT    NOT NULL,
            applied_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
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

            apply_migration_safe(conn, migration.sql, migration.version)?;

            conn.execute(
                "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
                rusqlite::params![migration.version, migration.name],
            )?;
        }
    }

    log::info!("All migrations applied");
    Ok(())
}
