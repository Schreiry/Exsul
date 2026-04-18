-- Migration 014: Backfill order_items.sort_id for legacy rows.
-- The sort_id column was added in migration 005 but was never written by
-- the Rust layer, so existing order_items have sort_id=NULL even when
-- item_id matches a real flower_sorts.id. This one-shot UPDATE restores
-- the link so printouts, shortage checks and order-detail views can
-- resolve the linked sort reliably.
-- Idempotent: subsequent runs find no NULL rows and skip.
UPDATE order_items
   SET sort_id = item_id
 WHERE sort_id IS NULL
   AND item_id IN (SELECT id FROM flower_sorts);

CREATE INDEX IF NOT EXISTS idx_order_items_sort_id
    ON order_items(sort_id) WHERE sort_id IS NOT NULL;
