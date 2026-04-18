use crate::events::types::{
    AddOrderItemPayload, AddTrustedNodePayload, AppSetting, AuditLog, AuditLogFilter, Category,
    CreateCategoryPayload, CreateFlowerSortPayload, CreatePackAssignmentPayload, EventRecord,
    FlowerConstants, FlowerSort, HarvestLogEntry, Item, Order, OrderItem, OrderShortage,
    PackAssignment, PackageResult, PackagingLogEntry, PriceRecord, SyncPeer, TrustedNode,
    UpdateCategoryPayload, UpdateFlowerSortPayload, CreateOrderPayload,
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
                    category_id, image_path, card_color
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
                    total_amount, notes, created_at, updated_at,
                    customer_company, delivery_address, delivery_notes,
                    pack_count_ordered, pack_count_ready, deadline_confirmed
             FROM orders WHERE status = ?1 ORDER BY created_at DESC"
                .to_string(),
            vec![Box::new(s.to_string()) as Box<dyn rusqlite::types::ToSql>],
        ),
        None => (
            "SELECT id, customer_name, customer_email, customer_phone, deadline, status,
                    total_amount, notes, created_at, updated_at,
                    customer_company, delivery_address, delivery_notes,
                    pack_count_ordered, pack_count_ready, deadline_confirmed
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
                pack_count_ordered, pack_count_ready, deadline_confirmed
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
    Ok(())
}

pub fn package_flowers(
    conn: &Connection,
    log_id: &str,
    sort_id: &str,
    pack_count: i32,
    flowers_per_pack: i32,
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

    // Log the transaction
    conn.execute(
        "INSERT INTO packaging_log (id, sort_id, pack_count, stems_used) VALUES (?1, ?2, ?3, ?4)",
        params![log_id, sort_id, pack_count, stems_needed],
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
    conn.execute_batch(
        "DELETE FROM order_items WHERE item_id IN (SELECT id FROM items);
         DELETE FROM item_prices WHERE item_id IN (SELECT id FROM items);",
    )
    .map_err(|e| e.to_string())?;
    let deleted = conn
        .execute("DELETE FROM items", [])
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
                    pack_count_ordered, pack_count_ready, deadline_confirmed
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

pub fn get_packaging_log(
    conn: &Connection,
    limit: Option<i64>,
) -> Result<Vec<PackagingLogEntry>, String> {
    let lim = limit.unwrap_or(200);
    let mut stmt = conn
        .prepare(
            "SELECT pl.id, pl.sort_id, fs.name, pl.pack_count, pl.stems_used, pl.created_at
             FROM packaging_log pl
             LEFT JOIN flower_sorts fs ON fs.id = pl.sort_id
             ORDER BY pl.created_at DESC
             LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([lim], |row| {
            Ok(PackagingLogEntry {
                id: row.get(0)?,
                sort_id: row.get(1)?,
                sort_name: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                pack_count: row.get(3)?,
                stems_used: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
