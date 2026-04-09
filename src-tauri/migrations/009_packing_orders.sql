-- Migration 009: pack assignments linking flower packaging to orders
CREATE TABLE IF NOT EXISTS pack_assignments (
  id             TEXT PRIMARY KEY,
  sort_id        TEXT NOT NULL REFERENCES flower_sorts(id),
  order_id       TEXT REFERENCES orders(id) ON DELETE SET NULL,
  pack_count     INTEGER NOT NULL DEFAULT 1,
  stems_per_pack INTEGER NOT NULL,
  status         TEXT NOT NULL DEFAULT 'prepared'
                 CHECK (status IN ('prepared', 'loaded', 'delivered')),
  note           TEXT,
  created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);
CREATE INDEX IF NOT EXISTS idx_pack_assignments_sort  ON pack_assignments(sort_id);
CREATE INDEX IF NOT EXISTS idx_pack_assignments_order ON pack_assignments(order_id);
CREATE INDEX IF NOT EXISTS idx_pack_assignments_status ON pack_assignments(status);
