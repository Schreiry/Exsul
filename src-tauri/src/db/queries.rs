use crate::events::types::{
    AddOrderItemPayload, AddTrustedNodePayload, AuditLog, AuditLogFilter, Category,
    CreateCategoryPayload, CreateFlowerSortPayload, EventRecord, FlowerConstants, FlowerSort,
    Item, Order, OrderItem, PriceRecord, SyncPeer, TrustedNode, UpdateCategoryPayload,
    UpdateFlowerSortPayload, CreateOrderPayload,
};
use rusqlite::{params, Connection};

// ============================================================
// Items
// ============================================================

pub fn get_items(conn: &Connection) -> Result<Vec<Item>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, category, initial_price, current_price, production_cost,
                    current_stock, sold_count, revenue, created_at, updated_at,
                    category_id, image_path
             FROM items ORDER BY updated_at DESC",
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
                category_id, image_path
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
        "INSERT INTO orders (id, customer_name, customer_email, customer_phone, deadline, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            id,
            payload.customer_name,
            payload.customer_email,
            payload.customer_phone,
            payload.deadline,
            payload.notes
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
                    total_amount, notes, created_at, updated_at
             FROM orders WHERE status = ?1 ORDER BY created_at DESC"
                .to_string(),
            vec![Box::new(s.to_string()) as Box<dyn rusqlite::types::ToSql>],
        ),
        None => (
            "SELECT id, customer_name, customer_email, customer_phone, deadline, status,
                    total_amount, notes, created_at, updated_at
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
                total_amount, notes, created_at, updated_at
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

    conn.execute(
        "INSERT INTO order_items (id, order_id, item_id, quantity, unit_price, specifications)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, payload.order_id, payload.item_id, payload.quantity, payload.unit_price, specs_str],
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
            "SELECT id, order_id, item_id, quantity, unit_price, specifications, created_at
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
            "SELECT id, name, variety, color_hex, raw_stock, pkg_stock, created_at, updated_at
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
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
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
        "INSERT INTO flower_sorts (id, name, variety, color_hex) VALUES (?1, ?2, ?3, ?4)",
        params![id, payload.name, payload.variety, payload.color_hex],
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
            name      = COALESCE(?2, name),
            variety   = COALESCE(?3, variety),
            color_hex = COALESCE(?4, color_hex),
            raw_stock = COALESCE(?5, raw_stock),
            pkg_stock = COALESCE(?6, pkg_stock),
            updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
         WHERE id = ?1",
        params![
            payload.id,
            payload.name,
            payload.variety,
            payload.color_hex,
            payload.raw_stock,
            payload.pkg_stock
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
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
    Ok(FlowerConstants {
        weight_per_flower: get_val(conn, "weight_per_flower"),
        flowers_per_pack: get_val(conn, "flowers_per_pack"),
        price_per_pack: get_val(conn, "price_per_pack"),
        price_per_flower: get_val(conn, "price_per_flower"),
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
    Ok(())
}
