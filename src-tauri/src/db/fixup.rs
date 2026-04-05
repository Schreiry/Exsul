use rusqlite::Connection;

/// One-time idempotent migration: populates the `categories` table from
/// existing `items.category` strings and backfills `items.category_id`.
/// Safe to run on every startup — INSERT OR IGNORE + idempotent UPDATE.
pub fn migrate_categories(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    // Collect all distinct category strings from existing items
    let mut stmt = conn.prepare(
        "SELECT DISTINCT category FROM items WHERE category IS NOT NULL AND category != ''",
    )?;

    let category_names: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    for name in &category_names {
        let id = uuid::Uuid::new_v4().to_string();
        // INSERT OR IGNORE keyed on UNIQUE(name) — safe to call multiple times
        conn.execute(
            "INSERT OR IGNORE INTO categories (id, name) VALUES (?1, ?2)",
            rusqlite::params![id, name],
        )?;
    }

    // Backfill category_id for items that still have NULL category_id
    conn.execute_batch(
        "UPDATE items
         SET category_id = (SELECT id FROM categories WHERE categories.name = items.category)
         WHERE category_id IS NULL AND category IS NOT NULL AND category != '';",
    )?;

    if !category_names.is_empty() {
        log::info!(
            "Category fixup: processed {} distinct categories",
            category_names.len()
        );
    }

    Ok(())
}
