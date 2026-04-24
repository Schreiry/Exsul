// ──────────────────────────────────────────────────────────────────────
// Backfill for legacy orders that were created BEFORE the warehouse↔order
// chain was wired (PackModal used to create an order but never insert
// `order_items` nor link `packaging_log.order_id`). Those orders print
// with empty Sort/Packs/Stems/Price columns — which is what the user is
// seeing in the "Print all orders" report over the archive range.
//
// Strategy (per-order, one transaction wrapping the whole sweep):
//
//   1. Enumerate orders with NO rows in `order_items`.
//   2. For each, gather candidate `packaging_log` rows where
//        packaging_log.order_id IS NULL
//        AND packaging_log.created_at is within ±15 minutes of order.created_at
//      (The old PackModal flow created the log row and the order within the
//      same click — milliseconds apart. 15 min is a generous window that
//      accommodates clock skew and batch operations.)
//   3. Group candidates by `sort_id`.
//   4. Resolve the match:
//        a) If exactly one sort appears in the window → strong match,
//           link all its log rows.
//        b) If multiple sorts but `order.pack_count_ordered > 0` and exactly
//           one sort-group's total pack_count equals pack_count_ordered →
//           link that group.
//        c) Otherwise → mark as ambiguous, skip (operator must resolve
//           manually).
//   5. For a successful match, write:
//        - UPDATE packaging_log SET order_id = ? for matched rows
//        - INSERT into order_items (quantity = total packs, pack_count,
//          stems_per_pack, unit_price = sort.sell_price_stem * stems_per_pack)
//        - Recalculate order.total_amount via the same COALESCE path used
//          elsewhere (see insert_order_item in queries.rs).
//        - INSERT into pack_assignments if one doesn't already exist for
//          (sort_id, order_id).
//
// Idempotent: after a successful run, the order has `order_items` rows, so
// the next sweep skips it (step 1 filter).
//
// This command is flowers-specific by design — it only touches rows that
// the flowers ERP schema owns (packaging_log, order_items, pack_assignments).
// ──────────────────────────────────────────────────────────────────────

use crate::db::Database;
use crate::events::store;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillOrderResult {
    pub order_id: String,
    pub customer_name: String,
    pub status: String, // "repaired" | "ambiguous" | "no_match"
    pub message: String,
    pub packs_restored: i32,
    pub total_restored: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillReport {
    pub total_legacy_orders: i64,
    pub repaired: i64,
    pub ambiguous: i64,
    pub no_match: i64,
    pub details: Vec<BackfillOrderResult>,
}

// Window for matching packaging_log.created_at → order.created_at, in minutes.
// Generous but not open-ended — the old PackModal wrote both rows within the
// same click so they're always within seconds of each other in practice.
const MATCH_WINDOW_MINUTES: i64 = 15;

#[tauri::command]
pub fn backfill_legacy_orders(db: State<'_, Database>) -> Result<BackfillReport, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // 1. Enumerate orders without any order_items. Joining to flower_sorts
    //    via packaging_log later — we only need the order's identity here.
    let mut legacy_stmt = conn
        .prepare(
            "SELECT o.id, o.customer_name, o.created_at, o.pack_count_ordered
             FROM orders o
             WHERE NOT EXISTS (
                 SELECT 1 FROM order_items oi WHERE oi.order_id = o.id
             )
             ORDER BY o.created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let legacy_orders: Vec<(String, String, String, i32)> = legacy_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i32>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    drop(legacy_stmt);

    let total_legacy = legacy_orders.len() as i64;
    let mut details: Vec<BackfillOrderResult> = Vec::with_capacity(legacy_orders.len());
    let mut repaired = 0i64;
    let mut ambiguous = 0i64;
    let mut no_match = 0i64;

    conn.execute("BEGIN IMMEDIATE", [])
        .map_err(|e| e.to_string())?;

    let process: Result<(), String> = (|| {
        for (order_id, customer_name, order_created, pack_count_ordered) in &legacy_orders {
            // 2. Candidate packaging_log rows. Time window applied in SQL via
            //    julianday(): avoids any locale parsing on the Rust side.
            let mut cand_stmt = conn
                .prepare(
                    "SELECT pl.id, pl.sort_id, pl.pack_count, pl.stems_used,
                            COALESCE(fs.flowers_per_pack_override, 0) AS fpp_override,
                            COALESCE(fs.sell_price_stem, 0.0) AS sell_price
                     FROM packaging_log pl
                     LEFT JOIN flower_sorts fs ON fs.id = pl.sort_id
                     WHERE pl.order_id IS NULL
                       AND ABS((julianday(pl.created_at) - julianday(?1)) * 24 * 60) <= ?2",
                )
                .map_err(|e| e.to_string())?;

            #[derive(Clone)]
            struct Candidate {
                log_id: String,
                sort_id: String,
                pack_count: i32,
                stems_used: i32,
                fpp_override: i32,
                sell_price_stem: f64,
            }

            let candidates: Vec<Candidate> = cand_stmt
                .query_map(
                    params![order_created, MATCH_WINDOW_MINUTES],
                    |row| {
                        Ok(Candidate {
                            log_id: row.get(0)?,
                            sort_id: row.get(1)?,
                            pack_count: row.get(2)?,
                            stems_used: row.get(3)?,
                            fpp_override: row.get(4)?,
                            sell_price_stem: row.get(5)?,
                        })
                    },
                )
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;

            drop(cand_stmt);

            if candidates.is_empty() {
                no_match += 1;
                details.push(BackfillOrderResult {
                    order_id: order_id.clone(),
                    customer_name: customer_name.clone(),
                    status: "no_match".into(),
                    message: format!(
                        "No unlinked packaging_log rows within ±{}m of order creation",
                        MATCH_WINDOW_MINUTES
                    ),
                    packs_restored: 0,
                    total_restored: 0.0,
                });
                continue;
            }

            // 3. Group by sort_id.
            use std::collections::HashMap;
            let mut by_sort: HashMap<String, Vec<Candidate>> = HashMap::new();
            for c in &candidates {
                by_sort.entry(c.sort_id.clone()).or_default().push(c.clone());
            }

            // 4. Resolve match.
            let chosen_sort: Option<String> = if by_sort.len() == 1 {
                Some(by_sort.keys().next().unwrap().clone())
            } else if *pack_count_ordered > 0 {
                let matching: Vec<String> = by_sort
                    .iter()
                    .filter_map(|(sid, rows)| {
                        let sum: i32 = rows.iter().map(|r| r.pack_count).sum();
                        if sum == *pack_count_ordered {
                            Some(sid.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                if matching.len() == 1 {
                    Some(matching.into_iter().next().unwrap())
                } else {
                    None
                }
            } else {
                None
            };

            let Some(sort_id) = chosen_sort else {
                ambiguous += 1;
                let sorts_seen: Vec<String> = by_sort.keys().cloned().collect();
                details.push(BackfillOrderResult {
                    order_id: order_id.clone(),
                    customer_name: customer_name.clone(),
                    status: "ambiguous".into(),
                    message: format!(
                        "{} candidate sort(s) in window; manual review required",
                        sorts_seen.len()
                    ),
                    packs_restored: 0,
                    total_restored: 0.0,
                });
                continue;
            };

            let rows = by_sort.remove(&sort_id).expect("chosen sort must be in map");
            let total_packs: i32 = rows.iter().map(|r| r.pack_count).sum();

            // Derive stems_per_pack. Prefer per-sort override; fall back to
            // stems_used / pack_count on the log row (historical truth).
            let stems_per_pack: i32 = if rows[0].fpp_override > 0 {
                rows[0].fpp_override
            } else if rows[0].pack_count > 0 {
                (rows[0].stems_used / rows[0].pack_count).max(1)
            } else {
                1
            };

            let price_per_pack: f64 = rows[0].sell_price_stem * (stems_per_pack as f64);

            // 5a. Link the packaging_log rows. Multi-row UPDATE with an IN list
            //     would need dynamic SQL; a loop is simpler and still fast.
            for r in &rows {
                conn.execute(
                    "UPDATE packaging_log SET order_id = ?1 WHERE id = ?2",
                    params![order_id, r.log_id],
                )
                .map_err(|e| e.to_string())?;
            }

            // 5b. Insert the order_item. We mirror the payload insert_order_item
            //     would produce so subsequent reads look indistinguishable from
            //     a natively-created order.
            let oi_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO order_items
                    (id, order_id, item_id, sort_id, quantity, unit_price,
                     specifications, pack_count, stems_per_pack)
                 VALUES (?1, ?2, ?3, ?3, ?4, ?5, '{}', ?4, ?6)",
                params![
                    oi_id,
                    order_id,
                    sort_id,
                    total_packs,
                    price_per_pack,
                    stems_per_pack
                ],
            )
            .map_err(|e| e.to_string())?;

            // 5c. Recalculate order total from the now-present order_items.
            conn.execute(
                "UPDATE orders SET
                    total_amount = (
                        SELECT COALESCE(SUM(quantity * unit_price), 0.0)
                        FROM order_items WHERE order_id = ?1
                    ),
                    updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now')
                 WHERE id = ?1",
                params![order_id],
            )
            .map_err(|e| e.to_string())?;

            // 5d. Create a pack_assignment if none exists for this sort+order.
            //     Status defaults to 'prepared' — the packs already exist as
            //     physical stock at this point.
            let existing_pa: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM pack_assignments
                     WHERE sort_id = ?1 AND order_id = ?2",
                    params![sort_id, order_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            if existing_pa == 0 {
                let pa_id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO pack_assignments
                        (id, sort_id, order_id, pack_count, stems_per_pack, status)
                     VALUES (?1, ?2, ?3, ?4, ?5, 'prepared')",
                    params![pa_id, sort_id, order_id, total_packs, stems_per_pack],
                )
                .map_err(|e| e.to_string())?;
            }

            let line_total = (total_packs as f64) * price_per_pack;
            repaired += 1;
            details.push(BackfillOrderResult {
                order_id: order_id.clone(),
                customer_name: customer_name.clone(),
                status: "repaired".into(),
                message: format!(
                    "Linked {} packaging_log row(s), sort {}, {} pack(s) @ {:.2}/pack",
                    rows.len(),
                    sort_id,
                    total_packs,
                    price_per_pack
                ),
                packs_restored: total_packs,
                total_restored: line_total,
            });
        }
        Ok(())
    })();

    match process {
        Ok(()) => {
            conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", []);
            return Err(e);
        }
    }

    drop(conn);

    // Audit log — one entry summarising the whole sweep. Per-order detail
    // already flows back to the caller through the report.
    if let Err(e) = store::append_audit_log(
        &db,
        "local",
        "OrdersBackfilled",
        json!({
            "total_legacy_orders": total_legacy,
            "repaired": repaired,
            "ambiguous": ambiguous,
            "no_match": no_match,
        }),
    ) {
        log::warn!("audit log write failed: {}", e);
    }

    Ok(BackfillReport {
        total_legacy_orders: total_legacy,
        repaired,
        ambiguous,
        no_match,
        details,
    })
}
