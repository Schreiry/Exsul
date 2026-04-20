-- Migration 015: Deletion-support indexes.
-- Enables efficient cascade unlinking when an order is deleted and
-- lookups of packaging_log rows by sort/order during history display.
-- Additive only: no ALTER/DROP.
CREATE INDEX IF NOT EXISTS idx_packaging_log_order ON packaging_log(order_id);
CREATE INDEX IF NOT EXISTS idx_packaging_log_sort  ON packaging_log(sort_id);
