use crate::events::types::{
    AddOrderItemPayload, AddTrustedNodePayload, AppSetting, AuditLog, AuditLogFilter, Category,
    Contact, ContactLocation, CreateCategoryPayload, CreateContactLocationPayload,
    CreateContactPayload, CreateFlowerSortPayload, CreatePackAssignmentPayload, CreateOrderPayload,
    EventRecord, FlowerConstants, FlowerSort, HarvestLogEntry, Item, Order, OrderItem,
    OrderShortage, OrderWaitingForSort, PackAssignment, PackageResult, PackagingLogEntry,
    PriceRecord, SyncPeer, TrustedNode, UpdateCategoryPayload, UpdateContactLocationPayload,
    UpdateContactPayload, UpdateFlowerSortPayload, UpdateOrderPayload,
};
use rusqlite::{params, Connection};

// ============================================================
// Items
// ============================================================

pub fn get_items(conn: &Connection) -> Result<Vec<Item>, String> {
    // Exclude shadow rows synthesized for flower_sorts (see migration 018 and
    // insert_flower_sort). Those exist only to satisfy the FK from
    // order_items.item_id → items(id) in flowers mode and must not appear in
    // the items-UI of other presets.
    let mut stmt = conn
        .prepare(
            "SELECT id, name, category, initial_price, current_price, production_cost,
                    current_stock, sold_count, revenue, created_at, updated_at,
                    category_id, image_path, card_color
             FROM items
             WHERE category != 'flower_sort'
             ORDER BY updated_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let items = stmt
        .query_map([], |row| {
            Ok(Item {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                initial_price: row.get(3)?,
                current_price: row.get(4)?,
                production_cost: row.get(5)?,
                current_stock: row.get(6)?,
                sold_count: row.get(7)?,
                revenue: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                category_id: row.get(11)?,
                image_path: row.get(12)?,
                card_color: row.get(13)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(items)
}

pub fn get_item_by_id(conn: &Connection, item_id: &str) -> Result<Option<Item>, String> {
    let result = conn.query_row(
        "SELECT id, name, category, initial_price, current_price, production_cost,
                current_stock, sold_count, revenue, created_at, updated_at,
                category_id, image_path, card_color
         FROM items WHERE id = ?1",
        [item_id],
        |row| {
            Ok(Item {
                id: row.get(0)?,
                name: row.get(1)?,
                category: row.get(2)?,
                initial_price: row.get(3)?,
                current_price: row.get(4)?,
                production_cost: row.get(5)?,
                current_stock: row.get(6)?,
                sold_count: row.get(7)?,
                revenue: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                category_id: row.get(11)?,
                image_path: row.get(12)?,
                card_color: row.get(13)?,
            })
        },
    );

    match result {
        Ok(item) => Ok(Some(item)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn set_item_image_path(conn: &Connection, item_id: &str, path: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE items SET image_path = ?1 WHERE id = ?2",
        params![path, item_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================
// Events / Price History / Sync
// ============================================================

pub fn get_events(
    conn: &Connection,
    since_hlc: Option<&str>,
    limit: Option<i64>,
) -> Result<Vec<EventRecord>, String> {
    let limit = limit.unwrap_or(1000);

    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match since_hlc {
        Some(hlc) => (
            "SELECT id, aggregate_id, aggregate_type, event_type, data, hlc_timestamp,
                    node_id, version, created_at
             FROM events WHERE hlc_timestamp > ?1 ORDER BY hlc_timestamp ASC LIMIT ?2"
                .to_string(),
            vec![
                Box::new(hlc.to_string()) as Box<dyn rusqlite::types::ToSql>,
                Box::new(limit),
            ],
        ),
        None => (
            "SELECT id, aggregate_id, aggregate_type, event_type, data, hlc_timestamp,
                    node_id, version, created_at
             FROM events ORDER BY hlc_timestamp ASC LIMIT ?1"
                .to_string(),
            vec![Box::new(limit) as Box<dyn rusqlite::types::ToSql>],
        ),
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();

    let events = stmt
        .query_map(params_refs.as_slice(), |row| {
            let data_str: String = row.get(4)?;
            let data: serde_json::Value =
                serde_json::from_str(&data_str)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
            Ok(EventRecord {
                id: Some(row.get(0)?),
                aggregate_id: row.get(1)?,
                aggregate_type: row.get(2)?,
                event_type: row.get(3)?,
                data,
                hlc_timestamp: row.get(5)?,
                node_id: row.get(6)?,
                version: row.get(7)?,
                created_at: Some(row.get(8)?),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(events)
}

pub fn get_price_history(conn: &Connection, item_id: &str) -> Result<Vec<PriceRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, item_id, price, effective_at, event_id, created_at
             FROM item_prices WHERE item_id = ?1 ORDER BY effective_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let records = stmt
        .query_map([item_id], |row| {
            Ok(PriceRecord {
                id: row.get(0)?,
                item_id: row.get(1)?,
                price: row.get(2)?,
                effective_at: row.get(3)?,
                event_id: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(records)
}

pub fn get_sync_state(conn: &Connection) -> Result<Vec<SyncPeer>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT peer_node_id, last_hlc, last_event_id, last_synced_at
             FROM sync_state ORDER BY last_synced_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let peers = stmt
        .query_map([], |row| {
            Ok(SyncPeer {
                peer_node_id: row.get(0)?,
                last_hlc: row.get(1)?,
                last_event_id: row.get(2)?,
                last_synced_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(peers)
}

pub fn get_node_id(conn: &Connection) -> Result<String, String> {
    conn.query_row(
        "SELECT value FROM local_config WHERE key = 'node_id'",
        [],
        |row| row.get(0),
    )
    .map_err(|e| e.to_string())
}

pub fn get_next_version(
    conn: &Connection,
    aggregate_id: &str,
    node_id: &str,
) -> Result<i64, String> {
    let result: Option<i64> = conn
        .query_row(
            "SELECT MAX(version) FROM events WHERE aggregate_id = ?1 AND node_id = ?2",
            params![aggregate_id, node_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(result.unwrap_or(0) + 1)
}

// ============================================================
// Categories
// ============================================================

pub fn get_categories(conn: &Connection) -> Result<Vec<Category>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, color, icon, created_at FROM categories ORDER BY name ASC",
        )
        .map_err(|e| e.to_string())?;

    let cats = stmt
        .query_map([], |row| {
            Ok(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                icon: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(cats)
}

pub fn insert_category(
    conn: &Connection,
    id: &str,
    payload: &CreateCategoryPayload,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO categories (id, name, color, icon) VALUES (?1, ?2, ?3, ?4)",
        params![id, payload.name, payload.color, payload.icon],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn update_category(
    conn: &Connection,
    payload: &UpdateCategoryPayload,
) -> Result<(), String> {
    conn.execute(
        "UPDATE categories SET
            name  = COALESCE(?2, name),
            color = COALESCE(?3, color),
            icon  = COALESCE(?4, icon)
         WHERE id = ?1",
        params![payload.id, payload.name, payload.color, payload.icon],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_category(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM categories WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================
// Orders
// ============================================================

pub fn insert_order(
    conn: &Connection,
    id: &str,
    payload: &CreateOrderPayload,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO orders (id, customer_name, customer_email, customer_phone, deadline, notes,
                             card_color, contact_id, contact_location_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            payload.customer_name,
            payload.customer_email,
            payload.customer_phone,
            payload.deadline,
            payload.notes,
            payload.card_color,
            payload.contact_id,
            payload.contact_location_id,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn update_order_status(
    conn: &Connection,
    order_id: &str,
    status: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE orders SET status = ?2, updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
         WHERE id = ?1",
        params![order_id, status],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_orders(
    conn: &Connection,
    status_filter: Option<&str>,
) -> Result<Vec<Order>, String> {
    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match status_filter {
        Some(s) => (
            "SELECT id, customer_name, customer_email, customer_phone, deadline, status,
                    total_amount, notes, created_at, updated_at,
                    customer_company, delivery_address, delivery_notes,
                    pack_count_ordered, pack_count_ready, deadline_confirmed, card_color,
                    contact_id, contact_location_id
             FROM orders WHERE status = ?1 ORDER BY created_at DESC"
                .to_string(),
            vec![Box::new(s.to_string()) as Box<dyn rusqlite::types::ToSql>],
        ),
        None => (
            "SELECT id, customer_name, customer_email, customer_phone, deadline, status,
                    total_amount, notes, created_at, updated_at,
                    customer_company, delivery_address, delivery_notes,
                    pack_count_ordered, pack_count_ready, deadline_confirmed, card_color,
                    contact_id, contact_location_id
             FROM orders ORDER BY created_at DESC"
                .to_string(),
            vec![],
        ),
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();

    let orders = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(Order {
                id: row.get(0)?,
                customer_name: row.get(1)?,
                customer_email: row.get(2)?,
                customer_phone: row.get(3)?,
                deadline: row.get(4)?,
                status: row.get(5)?,
                total_amount: row.get(6)?,
                notes: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                customer_company: row.get(10)?,
                delivery_address: row.get(11)?,
                delivery_notes: row.get(12)?,
                pack_count_ordered: row.get::<_, Option<i32>>(13)?.unwrap_or(0),
                pack_count_ready: row.get::<_, Option<i32>>(14)?.unwrap_or(0),
                deadline_confirmed: row.get::<_, Option<i32>>(15)?.unwrap_or(0) != 0,
                card_color: row.get::<_, Option<String>>(16).ok().flatten(),
                contact_id: row.get::<_, Option<String>>(17).ok().flatten(),
                contact_location_id: row.get::<_, Option<String>>(18).ok().flatten(),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(orders)
}

pub fn get_order(conn: &Connection, order_id: &str) -> Result<Option<Order>, String> {
    let result = conn.query_row(
        "SELECT id, customer_name, customer_email, customer_phone, deadline, status,
                total_amount, notes, created_at, updated_at,
                customer_company, delivery_address, delivery_notes,
                pack_count_ordered, pack_count_ready, deadline_confirmed, card_color,
                contact_id, contact_location_id
         FROM orders WHERE id = ?1",
        [order_id],
        |row| {
            Ok(Order {
                id: row.get(0)?,
                customer_name: row.get(1)?,
                customer_email: row.get(2)?,
                customer_phone: row.get(3)?,
                deadline: row.get(4)?,
                status: row.get(5)?,
                total_amount: row.get(6)?,
                notes: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                customer_company: row.get(10)?,
                delivery_address: row.get(11)?,
                delivery_notes: row.get(12)?,
                pack_count_ordered: row.get::<_, Option<i32>>(13)?.unwrap_or(0),
                pack_count_ready: row.get::<_, Option<i32>>(14)?.unwrap_or(0),
                deadline_confirmed: row.get::<_, Option<i32>>(15)?.unwrap_or(0) != 0,
                card_color: row.get::<_, Option<String>>(16).ok().flatten(),
                contact_id: row.get::<_, Option<String>>(17).ok().flatten(),
                contact_location_id: row.get::<_, Option<String>>(18).ok().flatten(),
            })
        },
    );

    match result {
        Ok(o) => Ok(Some(o)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn insert_order_item(
    conn: &Connection,
    id: &str,
    payload: &AddOrderItemPayload,
) -> Result<(), String> {
    let specs_str = serde_json::to_string(
        &payload.specifications.clone().unwrap_or(serde_json::json!({})),
    )
    .map_err(|e| e.to_string())?;

    let pack_count = payload.pack_count.unwrap_or(0);
    let stems_per_pack = payload.stems_per_pack.unwrap_or(0);

    // Flowers mode passes item_id = flower_sort.id, which would violate the
    // hard FK order_items.item_id → items(id). Lazily mirror the sort into
    // items so the FK holds. INSERT OR IGNORE is a no-op for real items rows
    // and for sorts already mirrored (migration 018 seeds them up front).
    conn.execute(
        "INSERT OR IGNORE INTO items (id, name, category, initial_price, current_price)
         SELECT fs.id, fs.name, 'flower_sort', 0.0, 0.0
           FROM flower_sorts fs
          WHERE fs.id = ?1
            AND NOT EXISTS (SELECT 1 FROM items WHERE id = fs.id)",
        params![payload.item_id],
    )
    .map_err(|e| e.to_string())?;

    // sort_id: prefer the explicit value from the payload. If absent, fall
    // back to a COALESCE lookup — if item_id happens to match an existing
    // flower_sorts row, the link is restored automatically. This keeps
    // older non-flower callers working without changes while making the
    // flowers flow robust.
    conn.execute(
        "INSERT INTO order_items
            (id, order_id, item_id, sort_id, quantity, unit_price,
             specifications, pack_count, stems_per_pack)
         VALUES (?1, ?2, ?3,
                 COALESCE(?4, (SELECT id FROM flower_sorts WHERE id = ?3)),
                 ?5, ?6, ?7, ?8, ?9)",
        params![
            id,
            payload.order_id,
            payload.item_id,
            payload.sort_id,
            payload.quantity,
            payload.unit_price,
            specs_str,
            pack_count,
            stems_per_pack
        ],
    )
    .map_err(|e| e.to_string())?;

    recalculate_order_total(conn, &payload.order_id)?;
    Ok(())
}

pub fn recalculate_order_total(conn: &Connection, order_id: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE orders SET
            total_amount = (
                SELECT COALESCE(SUM(quantity * unit_price), 0.0)
                FROM order_items WHERE order_id = ?1
            ),
            updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
         WHERE id = ?1",
        [order_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_order_items(conn: &Connection, order_id: &str) -> Result<Vec<OrderItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, order_id, item_id, quantity, unit_price, specifications, created_at,
                    pack_count, stems_per_pack, sort_id
             FROM order_items WHERE order_id = ?1 ORDER BY created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let items = stmt
        .query_map([order_id], |row| {
            let specs_str: String = row.get(5)?;
            let specifications: serde_json::Value =
                serde_json::from_str(&specs_str)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
            Ok(OrderItem {
                id: row.get(0)?,
                order_id: row.get(1)?,
                item_id: row.get(2)?,
                quantity: row.get(3)?,
                unit_price: row.get(4)?,
                specifications,
                created_at: row.get(6)?,
                pack_count: row.get::<_, i32>(7).unwrap_or(0),
                stems_per_pack: row.get::<_, i32>(8).unwrap_or(0),
                sort_id: row.get::<_, Option<String>>(9).unwrap_or(None),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(items)
}

// ============================================================
// Audit Logs
// ============================================================

pub fn insert_audit_log(
    conn: &Connection,
    user_id: &str,
    action: &str,
    payload: &serde_json::Value,
) -> Result<(), String> {
    let payload_str = serde_json::to_string(payload).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO audit_logs (user_id, action, payload) VALUES (?1, ?2, ?3)",
        params![user_id, action, payload_str],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_audit_logs(
    conn: &Connection,
    filter: &AuditLogFilter,
) -> Result<Vec<AuditLog>, String> {
    let limit = filter.limit.unwrap_or(500);
    let mut conditions: Vec<String> = Vec::new();
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1usize;

    if let Some(ref uid) = filter.user_id {
        conditions.push(format!("user_id = ?{}", idx));
        params_vec.push(Box::new(uid.clone()));
        idx += 1;
    }
    if let Some(ref act) = filter.action {
        conditions.push(format!("action LIKE ?{}", idx));
        params_vec.push(Box::new(format!("%{}%", act)));
        idx += 1;
    }
    if let Some(ref since) = filter.since {
        conditions.push(format!("timestamp >= ?{}", idx));
        params_vec.push(Box::new(since.clone()));
        idx += 1;
    }
    if let Some(ref until) = filter.until {
        conditions.push(format!("timestamp <= ?{}", idx));
        params_vec.push(Box::new(until.clone()));
        idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT id, timestamp, user_id, action, payload, ip_address, session_id
         FROM audit_logs {} ORDER BY timestamp DESC LIMIT ?{}",
        where_clause, idx
    );
    params_vec.push(Box::new(limit));

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();

    let logs = stmt
        .query_map(params_refs.as_slice(), |row| {
            let payload_str: String = row.get(4)?;
            let payload: serde_json::Value = serde_json::from_str(&payload_str)
                .unwrap_or(serde_json::Value::Object(Default::default()));
            Ok(AuditLog {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                user_id: row.get(2)?,
                action: row.get(3)?,
                payload,
                ip_address: row.get(5)?,
                session_id: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(logs)
}

// ============================================================
// Local Config
// ============================================================

pub fn get_local_config(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    let result = conn.query_row(
        "SELECT value FROM local_config WHERE key = ?1",
        [key],
        |row| row.get(0),
    );
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn set_local_config(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO local_config (key, value) VALUES (?1, ?2)",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================
// Trusted Nodes
// ============================================================

pub fn get_trusted_nodes(conn: &Connection) -> Result<Vec<TrustedNode>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT node_id, alias, ip_hint, added_at FROM trusted_nodes ORDER BY added_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let nodes = stmt
        .query_map([], |row| {
            Ok(TrustedNode {
                node_id: row.get(0)?,
                alias: row.get(1)?,
                ip_hint: row.get(2)?,
                added_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(nodes)
}

pub fn is_trusted_node(conn: &Connection, node_id: &str) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM trusted_nodes WHERE node_id = ?1",
            [node_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(count > 0)
}

pub fn get_trusted_node_alias(
    conn: &Connection,
    node_id: &str,
) -> Result<Option<String>, String> {
    let result = conn.query_row(
        "SELECT alias FROM trusted_nodes WHERE node_id = ?1",
        [node_id],
        |row| row.get(0),
    );
    match result {
        Ok(v) => Ok(v),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn insert_trusted_node(
    conn: &Connection,
    payload: &AddTrustedNodePayload,
) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO trusted_nodes (node_id, alias, ip_hint) VALUES (?1, ?2, ?3)",
        params![payload.node_id, payload.alias, payload.ip_hint],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_trusted_node(conn: &Connection, node_id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM trusted_nodes WHERE node_id = ?1", [node_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_peer_last_hlc(
    conn: &Connection,
    peer_node_id: &str,
) -> Result<Option<String>, String> {
    let result = conn.query_row(
        "SELECT last_hlc FROM sync_state WHERE peer_node_id = ?1",
        [peer_node_id],
        |row| row.get(0),
    );
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

// ============================================================
// Flower Sorts
// ============================================================

pub fn get_flower_sorts(conn: &Connection) -> Result<Vec<FlowerSort>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, variety, color_hex, raw_stock, pkg_stock,
                    purchase_price, sell_price_stem, flowers_per_pack_override,
                    created_at, updated_at,
                    photo_path, description, total_harvested
             FROM flower_sorts ORDER BY name ASC, variety ASC",
        )
        .map_err(|e| e.to_string())?;

    let sorts = stmt
        .query_map([], |row| {
            Ok(FlowerSort {
                id: row.get(0)?,
                name: row.get(1)?,
                variety: row.get(2)?,
                color_hex: row.get(3)?,
                raw_stock: row.get(4)?,
                pkg_stock: row.get(5)?,
                purchase_price: row.get(6)?,
                sell_price_stem: row.get(7)?,
                flowers_per_pack_override: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                photo_path: row.get(11)?,
                description: row.get(12)?,
                total_harvested: row.get::<_, Option<i32>>(13)?.unwrap_or(0),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(sorts)
}

pub fn insert_flower_sort(
    conn: &Connection,
    id: &str,
    payload: &CreateFlowerSortPayload,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO flower_sorts (id, name, variety, color_hex, purchase_price, sell_price_stem, flowers_per_pack_override, description)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            payload.name,
            payload.variety,
            payload.color_hex,
            payload.purchase_price.unwrap_or(0.0),
            payload.sell_price_stem.unwrap_or(0.0),
            payload.flowers_per_pack_override,
            payload.description,
        ],
    )
    .map_err(|e| e.to_string())?;

    // Shadow row in items: required so order_items.item_id → items(id) FK
    // holds when this sort is later referenced from an order. Migration 018
    // seeds shadows for pre-existing sorts; this keeps new sorts consistent.
    conn.execute(
        "INSERT OR IGNORE INTO items (id, name, category, initial_price, current_price)
         VALUES (?1, ?2, 'flower_sort', 0.0, 0.0)",
        params![id, payload.name],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn update_flower_sort(
    conn: &Connection,
    payload: &UpdateFlowerSortPayload,
) -> Result<(), String> {
    conn.execute(
        "UPDATE flower_sorts SET
            name                      = COALESCE(?2,  name),
            variety                   = COALESCE(?3,  variety),
            color_hex                 = COALESCE(?4,  color_hex),
            raw_stock                 = COALESCE(?5,  raw_stock),
            pkg_stock                 = COALESCE(?6,  pkg_stock),
            purchase_price            = COALESCE(?7,  purchase_price),
            sell_price_stem           = COALESCE(?8,  sell_price_stem),
            flowers_per_pack_override = COALESCE(?9,  flowers_per_pack_override),
            description               = COALESCE(?10, description),
            photo_path                = COALESCE(?11, photo_path),
            updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
         WHERE id = ?1",
        params![
            payload.id,
            payload.name,
            payload.variety,
            payload.color_hex,
            payload.raw_stock,
            payload.pkg_stock,
            payload.purchase_price,
            payload.sell_price_stem,
            payload.flowers_per_pack_override,
            payload.description,
            payload.photo_path,
        ],
    )
    .map_err(|e| e.to_string())?;

    // Keep the shadow items row in sync on rename so printouts and lookups
    // that join through items see the current name.
    if payload.name.is_some() {
        conn.execute(
            "UPDATE items SET name = COALESCE(?2, name)
               WHERE id = ?1 AND category = 'flower_sort'",
            params![payload.id, payload.name],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn package_flowers(
    conn: &Connection,
    log_id: &str,
    sort_id: &str,
    pack_count: i32,
    flowers_per_pack: i32,
    order_id: Option<&str>,
) -> Result<PackageResult, String> {
    // Read current stock
    let (raw_stock, _pkg_stock): (i32, i32) = conn
        .query_row(
            "SELECT raw_stock, pkg_stock FROM flower_sorts WHERE id = ?1",
            [sort_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    let stems_needed = pack_count * flowers_per_pack;

    if raw_stock < stems_needed {
        return Err(format!(
            "Недостаточно стеблей: нужно {}, есть {}",
            stems_needed, raw_stock
        ));
    }

    // Deduct raw, add packed
    conn.execute(
        "UPDATE flower_sorts SET
            raw_stock  = raw_stock - ?2,
            pkg_stock  = pkg_stock + ?3,
            updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
         WHERE id = ?1",
        params![sort_id, stems_needed, pack_count],
    )
    .map_err(|e| e.to_string())?;

    // Log the transaction. `order_id` is optional — when supplied, the
    // packaging entry is immediately linked to the order so the warehouse
    // ↔ orders chain is queryable in both directions.
    conn.execute(
        "INSERT INTO packaging_log (id, sort_id, pack_count, stems_used, order_id)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![log_id, sort_id, pack_count, stems_needed, order_id],
    )
    .map_err(|e| e.to_string())?;

    // Read back new values
    let (new_raw, new_pkg): (i32, i32) = conn
        .query_row(
            "SELECT raw_stock, pkg_stock FROM flower_sorts WHERE id = ?1",
            [sort_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    Ok(PackageResult {
        sort_id: sort_id.to_string(),
        new_raw_stock: new_raw,
        new_pkg_stock: new_pkg,
        stems_used: stems_needed,
        packs_created: pack_count,
    })
}

pub fn delete_flower_sort(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM flower_sorts WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn adjust_flower_stock(
    conn: &Connection,
    id: &str,
    raw_delta: i32,
    pkg_delta: i32,
) -> Result<(), String> {
    conn.execute(
        "UPDATE flower_sorts SET
            raw_stock  = MAX(0, raw_stock  + ?2),
            pkg_stock  = MAX(0, pkg_stock  + ?3),
            updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
         WHERE id = ?1",
        params![id, raw_delta, pkg_delta],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================
// Flower Constants
// ============================================================

pub fn get_flower_constants(conn: &Connection) -> Result<FlowerConstants, String> {
    fn get_val(conn: &Connection, k: &str) -> f64 {
        conn.query_row(
            "SELECT value FROM flower_constants WHERE key = ?1",
            [k],
            |row| row.get(0),
        )
        .unwrap_or(0.0)
    }
    let pricing_mode = conn
        .query_row(
            "SELECT value FROM local_config WHERE key = 'pricing_mode'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "pack".to_string());
    Ok(FlowerConstants {
        weight_per_flower: get_val(conn, "weight_per_flower"),
        flowers_per_pack: get_val(conn, "flowers_per_pack"),
        price_per_pack: get_val(conn, "price_per_pack"),
        price_per_flower: get_val(conn, "price_per_flower"),
        pricing_mode,
    })
}

pub fn set_flower_constants(conn: &Connection, c: &FlowerConstants) -> Result<(), String> {
    let pairs = [
        ("weight_per_flower", c.weight_per_flower),
        ("flowers_per_pack", c.flowers_per_pack),
        ("price_per_pack", c.price_per_pack),
        ("price_per_flower", c.price_per_flower),
    ];
    for (k, v) in &pairs {
        conn.execute(
            "INSERT OR REPLACE INTO flower_constants (key, value) VALUES (?1, ?2)",
            params![k, v],
        )
        .map_err(|e| e.to_string())?;
    }
    // pricing_mode is stored as a text key-value in a separate config entry
    conn.execute(
        "INSERT OR REPLACE INTO local_config (key, value) VALUES ('pricing_mode', ?1)",
        params![c.pricing_mode],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================
// Items — delete & duplicate (local ops, no event sourcing)
// ============================================================

pub fn delete_item(conn: &Connection, item_id: &str) -> Result<(), String> {
    // Remove FK references first to avoid constraint violations
    conn.execute("DELETE FROM order_items WHERE item_id = ?1", [item_id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM items WHERE id = ?1", [item_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_all_items(conn: &Connection) -> Result<usize, String> {
    // Scope the wipe to user-visible items only. Shadow rows carrying
    // category = 'flower_sort' are owned by the flowers preset and their
    // removal would cascade-break order_items that reference them.
    conn.execute_batch(
        "DELETE FROM order_items WHERE item_id IN
             (SELECT id FROM items WHERE category != 'flower_sort');
         DELETE FROM item_prices WHERE item_id IN
             (SELECT id FROM items WHERE category != 'flower_sort');",
    )
    .map_err(|e| e.to_string())?;
    let deleted = conn
        .execute("DELETE FROM items WHERE category != 'flower_sort'", [])
        .map_err(|e| e.to_string())?;
    Ok(deleted)
}

pub fn duplicate_item(conn: &Connection, source_id: &str, new_id: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO items (id, name, category, initial_price, current_price, production_cost,
                            current_stock, sold_count, revenue, created_at, updated_at,
                            category_id, image_path, card_color)
         SELECT ?2,
                name || ' (копия)',
                category,
                initial_price,
                current_price,
                production_cost,
                current_stock,
                0,
                0.0,
                strftime('%Y-%m-%dT%H:%M:%f', 'now'),
                strftime('%Y-%m-%dT%H:%M:%f', 'now'),
                category_id,
                image_path,
                card_color
         FROM items WHERE id = ?1",
        params![source_id, new_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================
// Orders — extended operations (migration 011)
// ============================================================

pub fn update_order_extended(
    conn: &Connection,
    order_id: &str,
    customer_company: Option<&str>,
    delivery_address: Option<&str>,
    delivery_notes: Option<&str>,
    pack_count_ordered: Option<i32>,
) -> Result<(), String> {
    conn.execute(
        "UPDATE orders SET
            customer_company   = COALESCE(?2, customer_company),
            delivery_address   = COALESCE(?3, delivery_address),
            delivery_notes     = COALESCE(?4, delivery_notes),
            pack_count_ordered = COALESCE(?5, pack_count_ordered),
            updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
         WHERE id = ?1",
        params![
            order_id,
            customer_company,
            delivery_address,
            delivery_notes,
            pack_count_ordered,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ────────────────────────────────────────────────────────────────
// Broad order update (migration 016 + ongoing edit UX).
//
// Semantics:
//   • Option::None on any nullable field → keep existing value (COALESCE).
//   • Option::Some(v)                    → overwrite with v.
//   • clear_* = true                     → force NULL, overriding Some/None.
//
// customer_name is special: we only overwrite when Some AND non-empty —
// the customer name is the one required field on an order and we never
// want an edit to silently blank it. Blank strings from the form are
// treated as "no change".
// ────────────────────────────────────────────────────────────────
pub fn update_order_full(
    conn: &Connection,
    payload: &UpdateOrderPayload,
) -> Result<(), String> {
    let customer_name_opt: Option<&str> = payload
        .customer_name
        .as_deref()
        .filter(|s| !s.trim().is_empty());

    let card_color_final: Option<String> = if payload.clear_card_color {
        None
    } else {
        payload.card_color.clone()
    };
    let deadline_final: Option<String> = if payload.clear_deadline {
        None
    } else {
        payload.deadline.clone()
    };
    let contact_id_final: Option<String> = if payload.clear_contact {
        None
    } else {
        payload.contact_id.clone()
    };
    let contact_location_id_final: Option<String> = if payload.clear_contact {
        None
    } else {
        payload.contact_location_id.clone()
    };

    conn.execute(
        "UPDATE orders SET
            customer_name        = COALESCE(?2, customer_name),
            customer_email       = COALESCE(?3, customer_email),
            customer_phone       = COALESCE(?4, customer_phone),
            customer_company     = COALESCE(?5, customer_company),
            delivery_address     = COALESCE(?6, delivery_address),
            delivery_notes       = COALESCE(?7, delivery_notes),
            notes                = COALESCE(?8, notes),
            deadline             = CASE WHEN ?10 = 1 THEN NULL ELSE COALESCE(?9, deadline) END,
            card_color           = CASE WHEN ?12 = 1 THEN NULL ELSE COALESCE(?11, card_color) END,
            contact_id           = CASE WHEN ?14 = 1 THEN NULL ELSE COALESCE(?13, contact_id) END,
            contact_location_id  = CASE WHEN ?14 = 1 THEN NULL ELSE COALESCE(?15, contact_location_id) END,
            updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
         WHERE id = ?1",
        params![
            payload.order_id,                              // ?1
            customer_name_opt,                             // ?2
            payload.customer_email,                        // ?3
            payload.customer_phone,                        // ?4
            payload.customer_company,                      // ?5
            payload.delivery_address,                      // ?6
            payload.delivery_notes,                        // ?7
            payload.notes,                                 // ?8
            deadline_final,                                // ?9
            if payload.clear_deadline { 1 } else { 0 },    // ?10
            card_color_final,                              // ?11
            if payload.clear_card_color { 1 } else { 0 },  // ?12
            contact_id_final,                              // ?13
            if payload.clear_contact { 1 } else { 0 },     // ?14
            contact_location_id_final,                     // ?15
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn confirm_order_deadline(conn: &Connection, order_id: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE orders SET deadline_confirmed = 1,
                updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
         WHERE id = ?1",
        [order_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_overdue_unconfirmed_orders(conn: &Connection) -> Result<Vec<Order>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, customer_name, customer_email, customer_phone, deadline, status,
                    total_amount, notes, created_at, updated_at,
                    customer_company, delivery_address, delivery_notes,
                    pack_count_ordered, pack_count_ready, deadline_confirmed, card_color,
                    contact_id, contact_location_id
             FROM orders
             WHERE deadline IS NOT NULL
               AND deadline < strftime('%Y-%m-%dT%H:%M:%f', 'now')
               AND deadline_confirmed = 0
               AND status NOT IN ('completed', 'cancelled')
             ORDER BY deadline ASC",
        )
        .map_err(|e| e.to_string())?;

    let orders = stmt
        .query_map([], |row| {
            Ok(Order {
                id: row.get(0)?,
                customer_name: row.get(1)?,
                customer_email: row.get(2)?,
                customer_phone: row.get(3)?,
                deadline: row.get(4)?,
                status: row.get(5)?,
                total_amount: row.get(6)?,
                notes: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                customer_company: row.get(10)?,
                delivery_address: row.get(11)?,
                delivery_notes: row.get(12)?,
                pack_count_ordered: row.get::<_, Option<i32>>(13)?.unwrap_or(0),
                pack_count_ready: row.get::<_, Option<i32>>(14)?.unwrap_or(0),
                deadline_confirmed: row.get::<_, Option<i32>>(15)?.unwrap_or(0) != 0,
                card_color: row.get::<_, Option<String>>(16).ok().flatten(),
                contact_id: row.get::<_, Option<String>>(17).ok().flatten(),
                contact_location_id: row.get::<_, Option<String>>(18).ok().flatten(),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(orders)
}

pub fn check_order_shortages(conn: &Connection) -> Result<Vec<OrderShortage>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT
                oi.order_id,
                o.customer_name,
                oi.sort_id,
                fs.name,
                oi.pack_count,
                fs.pkg_stock,
                (oi.pack_count - fs.pkg_stock) AS shortage
             FROM order_items oi
             JOIN orders o ON o.id = oi.order_id
             JOIN flower_sorts fs ON fs.id = oi.sort_id
             WHERE o.status NOT IN ('completed', 'cancelled')
               AND oi.sort_id IS NOT NULL
               AND oi.pack_count > 0
               AND oi.pack_count > fs.pkg_stock
             ORDER BY shortage DESC",
        )
        .map_err(|e| e.to_string())?;

    let shortages = stmt
        .query_map([], |row| {
            Ok(OrderShortage {
                order_id: row.get(0)?,
                customer_name: row.get(1)?,
                sort_id: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                sort_name: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                ordered_packs: row.get(4)?,
                available_packs: row.get(5)?,
                shortage: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(shortages)
}

/// Return the ISO-8601 timestamp of the earliest order, or `None` when
/// the table is empty. Used to initialize the from-date in the
/// "print registry" dialog.
pub fn get_earliest_order_date(conn: &Connection) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT MIN(created_at) FROM orders",
        [],
        |row| row.get::<_, Option<String>>(0),
    )
    .map_err(|e| e.to_string())
}

// ============================================================
// Greenhouse harvest log (migration 010)
// ============================================================

pub fn insert_harvest_log(
    conn: &Connection,
    id: &str,
    sort_id: &str,
    delta: i32,
    reason: &str,
    note: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO greenhouse_harvest_log (id, sort_id, delta, reason, note)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, sort_id, delta, reason, note],
    )
    .map_err(|e| e.to_string())?;

    // Update total_harvested accumulator for positive deltas
    if delta > 0 {
        conn.execute(
            "UPDATE flower_sorts SET total_harvested = total_harvested + ?2,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
             WHERE id = ?1",
            params![sort_id, delta],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn get_harvest_log(
    conn: &Connection,
    sort_id: Option<&str>,
    limit: Option<i64>,
) -> Result<Vec<HarvestLogEntry>, String> {
    let lim = limit.unwrap_or(200);

    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match sort_id {
        Some(sid) => (
            "SELECT hl.id, hl.sort_id, COALESCE(fs.name, ''), hl.delta, hl.reason, hl.note, hl.created_at
             FROM greenhouse_harvest_log hl
             LEFT JOIN flower_sorts fs ON fs.id = hl.sort_id
             WHERE hl.sort_id = ?1
             ORDER BY hl.created_at DESC LIMIT ?2"
                .to_string(),
            vec![
                Box::new(sid.to_string()) as Box<dyn rusqlite::types::ToSql>,
                Box::new(lim),
            ],
        ),
        None => (
            "SELECT hl.id, hl.sort_id, COALESCE(fs.name, ''), hl.delta, hl.reason, hl.note, hl.created_at
             FROM greenhouse_harvest_log hl
             LEFT JOIN flower_sorts fs ON fs.id = hl.sort_id
             ORDER BY hl.created_at DESC LIMIT ?1"
                .to_string(),
            vec![Box::new(lim) as Box<dyn rusqlite::types::ToSql>],
        ),
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();

    let entries = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(HarvestLogEntry {
                id: row.get(0)?,
                sort_id: row.get(1)?,
                sort_name: row.get(2)?,
                delta: row.get(3)?,
                reason: row.get(4)?,
                note: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(entries)
}

pub fn set_flower_photo_path(
    conn: &Connection,
    sort_id: &str,
    photo_path: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE flower_sorts SET photo_path = ?2,
             updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
         WHERE id = ?1",
        params![sort_id, photo_path],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================
// App settings (migration 012)
// ============================================================

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    let result = conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        [key],
        |row| row.get(0),
    );
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value, updated_at)
         VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_all_settings(conn: &Connection) -> Result<Vec<AppSetting>, String> {
    let mut stmt = conn
        .prepare("SELECT key, value, value_type FROM app_settings ORDER BY key ASC")
        .map_err(|e| e.to_string())?;

    let settings = stmt
        .query_map([], |row| {
            Ok(AppSetting {
                key: row.get(0)?,
                value: row.get(1)?,
                value_type: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(settings)
}

// ============================================================
// Schema version (for VersionInfo)
// ============================================================

pub fn get_schema_version(conn: &Connection) -> i32 {
    conn.query_row(
        "SELECT MAX(version) FROM schema_migrations",
        [],
        |row| row.get::<_, Option<i32>>(0),
    )
    .unwrap_or(None)
    .unwrap_or(0)
}

pub fn update_item_card_color(conn: &Connection, item_id: &str, color: Option<&str>) -> Result<(), String> {
    conn.execute(
        "UPDATE items SET card_color = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now') WHERE id = ?2",
        params![color, item_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================
// Category — Sort Bridge (Task 2)
// ============================================================

pub fn sync_flower_sorts_to_categories(conn: &Connection) -> Result<(), String> {
    // For each flower sort, ensure a matching category entry exists (id = sort.id)
    conn.execute_batch(
        "INSERT OR IGNORE INTO categories (id, name, color, sort_id)
         SELECT id, name, color_hex, id FROM flower_sorts;
         UPDATE categories SET
             name     = fs.name,
             color    = fs.color_hex,
             sort_id  = fs.id
         FROM flower_sorts fs WHERE categories.id = fs.id;",
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================
// Pack Assignments (Task 9)
// ============================================================

pub fn insert_pack_assignment(
    conn: &Connection,
    id: &str,
    payload: &CreatePackAssignmentPayload,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO pack_assignments (id, sort_id, order_id, pack_count, stems_per_pack, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            payload.sort_id,
            payload.order_id,
            payload.pack_count,
            payload.stems_per_pack,
            payload.note,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_pack_assignments(
    conn: &Connection,
    order_id: Option<&str>,
) -> Result<Vec<PackAssignment>, String> {
    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match order_id {
        Some(oid) => (
            "SELECT id, sort_id, order_id, pack_count, stems_per_pack, status, note, created_at
             FROM pack_assignments WHERE order_id = ?1 ORDER BY created_at DESC"
                .to_string(),
            vec![Box::new(oid.to_string()) as Box<dyn rusqlite::types::ToSql>],
        ),
        None => (
            "SELECT id, sort_id, order_id, pack_count, stems_per_pack, status, note, created_at
             FROM pack_assignments ORDER BY created_at DESC"
                .to_string(),
            vec![],
        ),
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();

    let assignments = stmt
        .query_map(params_refs.as_slice(), |row| {
            Ok(PackAssignment {
                id: row.get(0)?,
                sort_id: row.get(1)?,
                order_id: row.get(2)?,
                pack_count: row.get(3)?,
                stems_per_pack: row.get(4)?,
                status: row.get(5)?,
                note: row.get(6)?,
                created_at: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(assignments)
}

pub fn update_pack_status(conn: &Connection, id: &str, status: &str) -> Result<(), String> {
    match status {
        "prepared" | "loaded" | "delivered" => {}
        other => return Err(format!("Invalid status: {}", other)),
    }
    conn.execute(
        "UPDATE pack_assignments SET status = ?2 WHERE id = ?1",
        params![id, status],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Packaging Log ────────────────────────────────────────────

// Shared SELECT — every packaging_log reader joins flower_sorts so the
// frontend can render a full print row (sort name + variety + unit price)
// without a second round-trip. Keeping the projection in one place avoids
// drift between readers.
const PACKAGING_LOG_SELECT: &str = "SELECT pl.id, pl.sort_id, fs.name, fs.variety,
                pl.pack_count, pl.stems_used,
                COALESCE(fs.sell_price_stem, 0.0),
                pl.order_id, pl.created_at
         FROM packaging_log pl
         LEFT JOIN flower_sorts fs ON fs.id = pl.sort_id";

fn map_packaging_log_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PackagingLogEntry> {
    let pack_count: i32 = row.get(4)?;
    let stems_used: i32 = row.get(5)?;
    // stems_used is the total — derive per-pack on the backend so every
    // caller agrees on the value even if pack_count is 0 (legacy guard).
    let stems_per_pack = if pack_count > 0 { stems_used / pack_count } else { 0 };
    Ok(PackagingLogEntry {
        id: row.get(0)?,
        sort_id: row.get(1)?,
        sort_name: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        variety: row.get::<_, Option<String>>(3)?,
        pack_count,
        stems_used,
        stems_per_pack,
        sell_price_stem: row.get::<_, Option<f64>>(6)?.unwrap_or(0.0),
        order_id: row.get::<_, Option<String>>(7)?,
        created_at: row.get(8)?,
    })
}

pub fn get_packaging_log(
    conn: &Connection,
    limit: Option<i64>,
) -> Result<Vec<PackagingLogEntry>, String> {
    let lim = limit.unwrap_or(200);
    let sql = format!("{} ORDER BY pl.created_at DESC LIMIT ?1", PACKAGING_LOG_SELECT);
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([lim], map_packaging_log_row)
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn get_packaging_log_by_sort(
    conn: &Connection,
    sort_id: &str,
    limit: Option<i64>,
) -> Result<Vec<PackagingLogEntry>, String> {
    let lim = limit.unwrap_or(50);
    let sql = format!(
        "{} WHERE pl.sort_id = ?1 ORDER BY pl.created_at DESC LIMIT ?2",
        PACKAGING_LOG_SELECT
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![sort_id, lim], map_packaging_log_row)
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Active orders that still need packs of a given sort. Drives the
/// "Orders waiting for this sort" section on the greenhouse detail modal
/// and the reserved-packs badge on flower cards.
///
/// An order qualifies when it is `pending` or `in_progress` AND at least one
/// of its `order_items` references the requested `sort_id`. For each order
/// we return:
///   - `ordered_packs`  — sum of pack_count across that order's items for
///     this sort (falls back to `quantity` for rows without pack_count, which
///     can happen on non-flowers presets but is defensive here).
///   - `reserved_packs` — sum of pack_assignments.pack_count for the same
///     (sort, order) tuple, excluding rows already marked `delivered` so we
///     only count stock that is reserved-but-still-on-hand.
///   - `shortage`       — max(0, ordered - reserved).
///
/// Ordered by deadline ascending (nulls last, using created_at as fallback)
/// so the operator sees the most urgent order first.
pub fn get_orders_waiting_for_sort(
    conn: &Connection,
    sort_id: &str,
) -> Result<Vec<OrderWaitingForSort>, String> {
    let sql = "
        SELECT o.id, o.customer_name, o.deadline, o.status, o.created_at,
               (SELECT COALESCE(SUM(COALESCE(oi.pack_count, oi.quantity, 0)), 0)
                FROM order_items oi
                WHERE oi.order_id = o.id AND oi.sort_id = ?1) AS ordered_packs,
               (SELECT COALESCE(SUM(pa.pack_count), 0)
                FROM pack_assignments pa
                WHERE pa.order_id = o.id
                  AND pa.sort_id = ?1
                  AND pa.status != 'delivered') AS reserved_packs
        FROM orders o
        WHERE o.status IN ('pending', 'in_progress')
          AND EXISTS (
              SELECT 1 FROM order_items oi
              WHERE oi.order_id = o.id AND oi.sort_id = ?1
          )
        ORDER BY COALESCE(o.deadline, o.created_at) ASC
    ";
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![sort_id], |row| {
            let ordered_packs: i32 = row.get::<_, i64>(5)? as i32;
            let reserved_packs: i32 = row.get::<_, i64>(6)? as i32;
            let shortage = if ordered_packs > reserved_packs {
                ordered_packs - reserved_packs
            } else {
                0
            };
            Ok(OrderWaitingForSort {
                order_id: row.get(0)?,
                customer_name: row.get(1)?,
                deadline: row.get(2)?,
                status: row.get(3)?,
                created_at: row.get(4)?,
                ordered_packs,
                reserved_packs,
                shortage,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Packaging entries linked to a specific order. Returned chronologically
/// (oldest first) because the print layout reads top-to-bottom along the
/// warehouse→order chain. Includes unlinked log rows? No — strict filter.
pub fn get_packaging_log_by_order(
    conn: &Connection,
    order_id: &str,
) -> Result<Vec<PackagingLogEntry>, String> {
    let sql = format!(
        "{} WHERE pl.order_id = ?1 ORDER BY pl.created_at ASC",
        PACKAGING_LOG_SELECT
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([order_id], map_packaging_log_row)
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

// ============================================================
// Deletion — orders, packaging, pack assignments
// ============================================================

/// Delete a single order. `pack_assignments.order_id` becomes NULL so physically
/// packaged stock stays available; `packaging_log.order_id` is similarly cleared.
/// `order_items` are removed (FK CASCADE is also in place as a backstop).
pub fn delete_order(conn: &Connection, order_id: &str) -> Result<(), String> {
    conn.execute("BEGIN IMMEDIATE", []).map_err(|e| e.to_string())?;

    let result: Result<(), String> = (|| {
        conn.execute(
            "UPDATE pack_assignments SET order_id = NULL WHERE order_id = ?1",
            [order_id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE packaging_log SET order_id = NULL WHERE order_id = ?1",
            [order_id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM order_items WHERE order_id = ?1", [order_id])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM orders WHERE id = ?1", [order_id])
            .map_err(|e| e.to_string())?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
            Ok(())
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", []);
            Err(e)
        }
    }
}

/// Delete every order. Unlinks pack_assignments and packaging_log, returns count.
pub fn delete_all_orders(conn: &Connection) -> Result<i64, String> {
    conn.execute("BEGIN IMMEDIATE", []).map_err(|e| e.to_string())?;

    let result: Result<i64, String> = (|| {
        conn.execute("UPDATE pack_assignments SET order_id = NULL", [])
            .map_err(|e| e.to_string())?;
        conn.execute("UPDATE packaging_log SET order_id = NULL", [])
            .map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM order_items", [])
            .map_err(|e| e.to_string())?;
        let deleted = conn
            .execute("DELETE FROM orders", [])
            .map_err(|e| e.to_string())?;
        Ok(deleted as i64)
    })();

    match result {
        Ok(count) => {
            conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
            Ok(count)
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", []);
            Err(e)
        }
    }
}

/// Delete a single packaging_log entry, rolling back the stock movement:
/// `pkg_stock -= pack_count`, `raw_stock += stems_used`, and record the
/// inverse move in `greenhouse_harvest_log` as a `correction`.
///
/// Fails if pkg_stock would go negative (i.e. some packs already shipped).
pub fn delete_packaging_entry(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("BEGIN IMMEDIATE", []).map_err(|e| e.to_string())?;

    let result: Result<(), String> = (|| {
        let (sort_id, pack_count, stems_used): (String, i32, i32) = conn
            .query_row(
                "SELECT sort_id, pack_count, stems_used FROM packaging_log WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => "packaging_entry_not_found".to_string(),
                other => other.to_string(),
            })?;

        let current_pkg: i32 = conn
            .query_row(
                "SELECT pkg_stock FROM flower_sorts WHERE id = ?1",
                [&sort_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        if current_pkg < pack_count {
            return Err("pkg_stock_underflow".to_string());
        }

        conn.execute(
            "UPDATE flower_sorts SET
                pkg_stock  = pkg_stock - ?2,
                raw_stock  = raw_stock + ?3,
                updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
             WHERE id = ?1",
            params![sort_id, pack_count, stems_used],
        )
        .map_err(|e| e.to_string())?;

        let harvest_id = uuid::Uuid::new_v4().to_string();
        let note = format!("rollback packaging {}", id);
        conn.execute(
            "INSERT INTO greenhouse_harvest_log (id, sort_id, delta, reason, note)
             VALUES (?1, ?2, ?3, 'correction', ?4)",
            params![harvest_id, sort_id, stems_used, note],
        )
        .map_err(|e| e.to_string())?;

        conn.execute("DELETE FROM packaging_log WHERE id = ?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
            Ok(())
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", []);
            Err(e)
        }
    }
}

/// Delete every packaging_log entry, rolling back each one. If any single
/// rollback is impossible (pkg_stock underflow), the whole operation aborts
/// and nothing is deleted.
pub fn delete_all_packaging(conn: &Connection) -> Result<i64, String> {
    conn.execute("BEGIN IMMEDIATE", []).map_err(|e| e.to_string())?;

    let result: Result<i64, String> = (|| {
        let mut stmt = conn
            .prepare("SELECT id, sort_id, pack_count, stems_used FROM packaging_log")
            .map_err(|e| e.to_string())?;
        let rows: Vec<(String, String, i32, i32)> = stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        drop(stmt);

        let mut count = 0i64;
        for (pl_id, sort_id, pack_count, stems_used) in rows {
            let current_pkg: i32 = conn
                .query_row(
                    "SELECT pkg_stock FROM flower_sorts WHERE id = ?1",
                    [&sort_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            if current_pkg < pack_count {
                return Err(format!("pkg_stock_underflow:{}", sort_id));
            }

            conn.execute(
                "UPDATE flower_sorts SET
                    pkg_stock  = pkg_stock - ?2,
                    raw_stock  = raw_stock + ?3,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
                 WHERE id = ?1",
                params![sort_id, pack_count, stems_used],
            )
            .map_err(|e| e.to_string())?;

            let harvest_id = uuid::Uuid::new_v4().to_string();
            let note = format!("rollback packaging {}", pl_id);
            conn.execute(
                "INSERT INTO greenhouse_harvest_log (id, sort_id, delta, reason, note)
                 VALUES (?1, ?2, ?3, 'correction', ?4)",
                params![harvest_id, sort_id, stems_used, note],
            )
            .map_err(|e| e.to_string())?;

            count += 1;
        }

        conn.execute("DELETE FROM packaging_log", [])
            .map_err(|e| e.to_string())?;
        Ok(count)
    })();

    match result {
        Ok(count) => {
            conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
            Ok(count)
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", []);
            Err(e)
        }
    }
}

/// Remove a pack-assignment row. The warehouse pkg_stock is NOT changed:
/// deleting an assignment means the reservation is released; the physical
/// packs remain available on the shelf.
pub fn delete_pack_assignment(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM pack_assignments WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================
// Contacts (migration 017) — Phase E
// ============================================================

fn row_to_contact(row: &rusqlite::Row<'_>) -> rusqlite::Result<Contact> {
    Ok(Contact {
        id: row.get(0)?,
        name: row.get(1)?,
        surname: row.get(2)?,
        email: row.get(3)?,
        phone: row.get(4)?,
        company: row.get(5)?,
        notes: row.get(6)?,
        photo_path: row.get(7)?,
        card_color: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        order_count: row.get::<_, Option<i64>>(11)?.unwrap_or(0),
        total_spent: row.get::<_, Option<f64>>(12)?.unwrap_or(0.0),
        default_address: row.get::<_, Option<String>>(13).ok().flatten(),
    })
}

/// List contacts (optionally filtered by case-insensitive substring on
/// name/surname/email/phone/company). Aggregates (order_count, total_spent)
/// are computed by LEFT JOIN on orders — a contact with zero orders returns
/// 0/0.0. The default address is pulled via a correlated subquery so the
/// row-set stays one row per contact.
pub fn list_contacts(
    conn: &Connection,
    search: Option<&str>,
) -> Result<Vec<Contact>, String> {
    let base_sql = "
        SELECT c.id, c.name, c.surname, c.email, c.phone, c.company,
               c.notes, c.photo_path, c.card_color, c.created_at, c.updated_at,
               COALESCE(agg.order_count, 0)   AS order_count,
               COALESCE(agg.total_spent, 0.0) AS total_spent,
               (SELECT address FROM contact_locations
                 WHERE contact_id = c.id AND is_default = 1
                 LIMIT 1)                      AS default_address
          FROM contacts c
          LEFT JOIN (
              SELECT contact_id,
                     COUNT(*)              AS order_count,
                     COALESCE(SUM(total_amount), 0.0) AS total_spent
                FROM orders
               WHERE contact_id IS NOT NULL
               GROUP BY contact_id
          ) agg ON agg.contact_id = c.id
    ";

    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match search {
        Some(q) if !q.trim().is_empty() => {
            let pattern = format!("%{}%", q.trim());
            (
                format!(
                    "{base_sql}
                     WHERE c.name     LIKE ?1 COLLATE NOCASE
                        OR c.surname  LIKE ?1 COLLATE NOCASE
                        OR c.email    LIKE ?1 COLLATE NOCASE
                        OR c.phone    LIKE ?1 COLLATE NOCASE
                        OR c.company  LIKE ?1 COLLATE NOCASE
                     ORDER BY c.name COLLATE NOCASE ASC"
                ),
                vec![Box::new(pattern) as Box<dyn rusqlite::types::ToSql>],
            )
        }
        _ => (
            format!("{base_sql} ORDER BY c.name COLLATE NOCASE ASC"),
            vec![],
        ),
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();

    let rows = stmt
        .query_map(params_refs.as_slice(), row_to_contact)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(rows)
}

pub fn get_contact(conn: &Connection, contact_id: &str) -> Result<Option<Contact>, String> {
    let sql = "
        SELECT c.id, c.name, c.surname, c.email, c.phone, c.company,
               c.notes, c.photo_path, c.card_color, c.created_at, c.updated_at,
               COALESCE(agg.order_count, 0),
               COALESCE(agg.total_spent, 0.0),
               (SELECT address FROM contact_locations
                 WHERE contact_id = c.id AND is_default = 1
                 LIMIT 1)
          FROM contacts c
          LEFT JOIN (
              SELECT contact_id,
                     COUNT(*) AS order_count,
                     COALESCE(SUM(total_amount), 0.0) AS total_spent
                FROM orders
               WHERE contact_id IS NOT NULL
               GROUP BY contact_id
          ) agg ON agg.contact_id = c.id
         WHERE c.id = ?1
    ";

    let result = conn.query_row(sql, [contact_id], row_to_contact);
    match result {
        Ok(c) => Ok(Some(c)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn insert_contact(
    conn: &Connection,
    id: &str,
    payload: &CreateContactPayload,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO contacts (id, name, surname, email, phone, company, notes, card_color)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            id,
            payload.name,
            payload.surname,
            payload.email,
            payload.phone,
            payload.company,
            payload.notes,
            payload.card_color,
        ],
    )
    .map_err(|e| e.to_string())?;

    // Seed optional default address so quick-create from the picker can
    // produce a fully-populated contact in one call.
    if let Some(addr) = payload.default_address.as_deref() {
        let addr_trim = addr.trim();
        if !addr_trim.is_empty() {
            let loc_id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO contact_locations (id, contact_id, label, address, is_default)
                 VALUES (?1, ?2, NULL, ?3, 1)",
                params![loc_id, id, addr_trim],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

pub fn update_contact(
    conn: &Connection,
    payload: &UpdateContactPayload,
) -> Result<(), String> {
    let name_opt: Option<&str> = payload
        .name
        .as_deref()
        .filter(|s| !s.trim().is_empty());

    let card_color_final: Option<String> = if payload.clear_card_color {
        None
    } else {
        payload.card_color.clone()
    };

    conn.execute(
        "UPDATE contacts SET
            name       = COALESCE(?2, name),
            surname    = COALESCE(?3, surname),
            email      = COALESCE(?4, email),
            phone      = COALESCE(?5, phone),
            company    = COALESCE(?6, company),
            notes      = COALESCE(?7, notes),
            card_color = CASE WHEN ?9 = 1 THEN NULL ELSE COALESCE(?8, card_color) END,
            updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
         WHERE id = ?1",
        params![
            payload.contact_id,
            name_opt,
            payload.surname,
            payload.email,
            payload.phone,
            payload.company,
            payload.notes,
            card_color_final,
            if payload.clear_card_color { 1 } else { 0 },
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete a contact. orders.contact_id is detached (set to NULL) so the
/// order history remains intact — deleting a client in the UI should not
/// nuke their past orders' totals.
pub fn delete_contact(conn: &Connection, contact_id: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE orders SET contact_id = NULL, contact_location_id = NULL WHERE contact_id = ?1",
        [contact_id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM contacts WHERE id = ?1", [contact_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_contact_photo_path(
    conn: &Connection,
    contact_id: &str,
    path: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE contacts SET photo_path = ?1,
             updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
         WHERE id = ?2",
        params![path, contact_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ── Locations ─────────────────────────────────────────────────

fn row_to_location(row: &rusqlite::Row<'_>) -> rusqlite::Result<ContactLocation> {
    Ok(ContactLocation {
        id: row.get(0)?,
        contact_id: row.get(1)?,
        label: row.get(2)?,
        address: row.get(3)?,
        is_default: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
    })
}

pub fn list_contact_locations(
    conn: &Connection,
    contact_id: &str,
) -> Result<Vec<ContactLocation>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, contact_id, label, address, is_default, created_at
               FROM contact_locations
              WHERE contact_id = ?1
              ORDER BY is_default DESC, created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([contact_id], row_to_location)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(rows)
}

pub fn insert_contact_location(
    conn: &Connection,
    id: &str,
    payload: &CreateContactLocationPayload,
) -> Result<(), String> {
    // If this is the first location OR caller asked for default, atomically
    // demote any existing default and flag the new one.
    let mut flag_default = payload.is_default;
    if !flag_default {
        let existing: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM contact_locations WHERE contact_id = ?1",
                [&payload.contact_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if existing == 0 {
            flag_default = true;
        }
    }

    if flag_default {
        conn.execute(
            "UPDATE contact_locations SET is_default = 0 WHERE contact_id = ?1",
            [&payload.contact_id],
        )
        .map_err(|e| e.to_string())?;
    }

    conn.execute(
        "INSERT INTO contact_locations (id, contact_id, label, address, is_default)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            id,
            payload.contact_id,
            payload.label,
            payload.address,
            if flag_default { 1 } else { 0 },
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn update_contact_location(
    conn: &Connection,
    payload: &UpdateContactLocationPayload,
) -> Result<(), String> {
    conn.execute(
        "UPDATE contact_locations SET
            label   = COALESCE(?2, label),
            address = COALESCE(?3, address)
         WHERE id = ?1",
        params![payload.location_id, payload.label, payload.address],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_contact_location(conn: &Connection, location_id: &str) -> Result<(), String> {
    // Forget the reference from any orders that pointed here (rare — but
    // keeps us consistent if a user deletes a location already attached to
    // a shipped order).
    conn.execute(
        "UPDATE orders SET contact_location_id = NULL WHERE contact_location_id = ?1",
        [location_id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM contact_locations WHERE id = ?1", [location_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Atomically promote `location_id` to default for its contact. Any sibling
/// rows for the same contact are demoted.
pub fn set_default_contact_location(
    conn: &Connection,
    location_id: &str,
) -> Result<(), String> {
    let contact_id: Option<String> = conn
        .query_row(
            "SELECT contact_id FROM contact_locations WHERE id = ?1",
            [location_id],
            |row| row.get(0),
        )
        .ok();

    let Some(contact_id) = contact_id else {
        return Err("Location not found".to_string());
    };

    conn.execute(
        "UPDATE contact_locations SET is_default = 0 WHERE contact_id = ?1",
        [&contact_id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE contact_locations SET is_default = 1 WHERE id = ?1",
        [location_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Orders belonging to a specific contact. Reuses the full Order SELECT
/// shape so the frontend can render with the same card/row components.
pub fn get_orders_for_contact(
    conn: &Connection,
    contact_id: &str,
) -> Result<Vec<Order>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, customer_name, customer_email, customer_phone, deadline, status,
                    total_amount, notes, created_at, updated_at,
                    customer_company, delivery_address, delivery_notes,
                    pack_count_ordered, pack_count_ready, deadline_confirmed, card_color,
                    contact_id, contact_location_id
               FROM orders
              WHERE contact_id = ?1
              ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let orders = stmt
        .query_map([contact_id], |row| {
            Ok(Order {
                id: row.get(0)?,
                customer_name: row.get(1)?,
                customer_email: row.get(2)?,
                customer_phone: row.get(3)?,
                deadline: row.get(4)?,
                status: row.get(5)?,
                total_amount: row.get(6)?,
                notes: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
                customer_company: row.get(10)?,
                delivery_address: row.get(11)?,
                delivery_notes: row.get(12)?,
                pack_count_ordered: row.get::<_, Option<i32>>(13)?.unwrap_or(0),
                pack_count_ready: row.get::<_, Option<i32>>(14)?.unwrap_or(0),
                deadline_confirmed: row.get::<_, Option<i32>>(15)?.unwrap_or(0) != 0,
                card_color: row.get::<_, Option<String>>(16).ok().flatten(),
                contact_id: row.get::<_, Option<String>>(17).ok().flatten(),
                contact_location_id: row.get::<_, Option<String>>(18).ok().flatten(),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(orders)
}

/// Backfill helper: groups existing orders by normalized customer_name and
/// creates a Contact per unique name. Each created contact is then linked
/// back to every order that matches its canonical name (case-insensitive
/// trim). Idempotent — orders already linked are skipped.
///
/// Returns (created_contacts, linked_orders).
pub fn backfill_contacts_from_orders(conn: &Connection) -> Result<(i64, i64), String> {
    // Canonical name → existing contact id (if any). We use the lowercased
    // trimmed name as the dedupe key.
    let mut stmt = conn
        .prepare(
            "SELECT TRIM(LOWER(customer_name)) AS key, customer_name,
                    MIN(customer_email)  AS email,
                    MIN(customer_phone)  AS phone,
                    MIN(delivery_address) AS addr
               FROM orders
              WHERE contact_id IS NULL
                AND customer_name IS NOT NULL
                AND TRIM(customer_name) <> ''
              GROUP BY key",
        )
        .map_err(|e| e.to_string())?;

    let groups: Vec<(String, String, Option<String>, Option<String>, Option<String>)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut created = 0i64;
    let mut linked = 0i64;

    for (key, display_name, email, phone, addr) in groups {
        // See if a contact with the same canonical name already exists.
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM contacts
                  WHERE TRIM(LOWER(name)) = ?1
                  LIMIT 1",
                [&key],
                |row| row.get(0),
            )
            .ok();

        let contact_id = match existing {
            Some(id) => id,
            None => {
                let new_id = uuid::Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO contacts (id, name, email, phone)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![new_id, display_name, email, phone],
                )
                .map_err(|e| e.to_string())?;

                // Seed an address if any matched order had one.
                if let Some(a) = addr.as_deref() {
                    let a_trim = a.trim();
                    if !a_trim.is_empty() {
                        let loc_id = uuid::Uuid::new_v4().to_string();
                        conn.execute(
                            "INSERT INTO contact_locations
                                 (id, contact_id, label, address, is_default)
                             VALUES (?1, ?2, NULL, ?3, 1)",
                            params![loc_id, new_id, a_trim],
                        )
                        .map_err(|e| e.to_string())?;
                    }
                }
                created += 1;
                new_id
            }
        };

        // Link every matching, still-unlinked order to this contact.
        let rows = conn
            .execute(
                "UPDATE orders SET contact_id = ?1
                  WHERE contact_id IS NULL
                    AND TRIM(LOWER(customer_name)) = ?2",
                params![contact_id, key],
            )
            .map_err(|e| e.to_string())?;
        linked += rows as i64;
    }

    Ok((created, linked))
}
