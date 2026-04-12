-- Migration 010: Greenhouse harvest log + packaging_log extensions

-- Extend flower_sorts with photo, description, total_harvested
ALTER TABLE flower_sorts ADD COLUMN photo_path TEXT;
ALTER TABLE flower_sorts ADD COLUMN description TEXT;
ALTER TABLE flower_sorts ADD COLUMN total_harvested INTEGER NOT NULL DEFAULT 0;

-- Audit log of raw stock movements (manual additions, packaged deductions, corrections)
CREATE TABLE IF NOT EXISTS greenhouse_harvest_log (
    id           TEXT PRIMARY KEY,
    sort_id      TEXT NOT NULL REFERENCES flower_sorts(id) ON DELETE CASCADE,
    delta        INTEGER NOT NULL,
    reason       TEXT NOT NULL DEFAULT 'manual',
    note         TEXT,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- Extend packaging_log with optional order link and warehouse confirmation
ALTER TABLE packaging_log ADD COLUMN order_id TEXT REFERENCES orders(id);
ALTER TABLE packaging_log ADD COLUMN warehouse_confirmed INTEGER NOT NULL DEFAULT 0;
