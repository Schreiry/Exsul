-- ============================================================
-- EXSUL: Migration 005 — Flower ERP (Transformation Pipeline)
-- ============================================================

-- =========================================
-- ERP fields on flower_sorts
-- purchase_price    — cost per bulb/stem at purchase
-- sell_price_stem   — selling price per single stem
-- flowers_per_pack_override — per-sort override; NULL = use global constant
-- =========================================
ALTER TABLE flower_sorts ADD COLUMN purchase_price          REAL NOT NULL DEFAULT 0;
ALTER TABLE flower_sorts ADD COLUMN sell_price_stem         REAL NOT NULL DEFAULT 0;
ALTER TABLE flower_sorts ADD COLUMN flowers_per_pack_override INTEGER;

-- =========================================
-- PACKAGING LOG
-- Records every "package" transaction:
--   stems_used = pack_count * effective_flowers_per_pack
-- =========================================
CREATE TABLE IF NOT EXISTS packaging_log (
    id          TEXT    PRIMARY KEY,
    sort_id     TEXT    NOT NULL REFERENCES flower_sorts(id) ON DELETE CASCADE,
    pack_count  INTEGER NOT NULL CHECK(pack_count > 0),
    stems_used  INTEGER NOT NULL CHECK(stems_used > 0),
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_pkg_log_sort ON packaging_log(sort_id);
CREATE INDEX IF NOT EXISTS idx_pkg_log_date ON packaging_log(created_at);

-- =========================================
-- ORDER ITEMS — extend for package linking
-- item_type: 'item' (generic) | 'package' (flower pack)
-- sort_id:   nullable FK to flower_sorts for package rows
-- =========================================
ALTER TABLE order_items ADD COLUMN item_type TEXT NOT NULL DEFAULT 'item';
ALTER TABLE order_items ADD COLUMN sort_id   TEXT REFERENCES flower_sorts(id);
