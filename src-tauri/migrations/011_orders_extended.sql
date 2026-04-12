-- Migration 011: Extended order fields for flower ERP

-- Extend orders table
ALTER TABLE orders ADD COLUMN customer_company    TEXT;
ALTER TABLE orders ADD COLUMN delivery_address    TEXT;
ALTER TABLE orders ADD COLUMN delivery_notes      TEXT;
ALTER TABLE orders ADD COLUMN pack_count_ordered  INTEGER NOT NULL DEFAULT 0;
ALTER TABLE orders ADD COLUMN pack_count_ready    INTEGER NOT NULL DEFAULT 0;
ALTER TABLE orders ADD COLUMN deadline_confirmed  INTEGER NOT NULL DEFAULT 0;

-- Extend order_items for flower mode
-- NOTE: sort_id already added in migration 005 (flower_erp) — do not re-add
ALTER TABLE order_items ADD COLUMN pack_count     INTEGER NOT NULL DEFAULT 0;
ALTER TABLE order_items ADD COLUMN stems_per_pack INTEGER NOT NULL DEFAULT 0;

-- Indices for deadline/status queries
CREATE INDEX IF NOT EXISTS idx_orders_deadline ON orders(deadline) WHERE deadline IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_orders_status   ON orders(status);
