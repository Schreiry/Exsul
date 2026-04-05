-- ============================================================
-- EXSUL: Migration 003 — Categories, Orders, Audit
-- ============================================================

-- =========================================
-- CATEGORIES (standalone entities)
-- Must be created BEFORE ALTER TABLE items
-- =========================================
CREATE TABLE IF NOT EXISTS categories (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,
    color      TEXT,
    icon       TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_categories_name ON categories(name);

-- =========================================
-- Extend items with category FK + image
-- =========================================
ALTER TABLE items ADD COLUMN category_id TEXT REFERENCES categories(id) ON DELETE SET NULL;
ALTER TABLE items ADD COLUMN image_path TEXT;

-- =========================================
-- ORDERS
-- =========================================
CREATE TABLE IF NOT EXISTS orders (
    id             TEXT PRIMARY KEY,
    customer_name  TEXT NOT NULL,
    customer_email TEXT,
    customer_phone TEXT,
    deadline       TEXT,
    status         TEXT NOT NULL DEFAULT 'pending'
                   CHECK(status IN ('pending','in_progress','completed','cancelled')),
    total_amount   REAL NOT NULL DEFAULT 0.0,
    notes          TEXT,
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    updated_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_orders_status   ON orders(status);
CREATE INDEX IF NOT EXISTS idx_orders_deadline ON orders(deadline);

-- =========================================
-- ORDER ITEMS
-- =========================================
CREATE TABLE IF NOT EXISTS order_items (
    id             TEXT PRIMARY KEY,
    order_id       TEXT NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    item_id        TEXT NOT NULL REFERENCES items(id),
    quantity       INTEGER NOT NULL DEFAULT 1,
    unit_price     REAL    NOT NULL DEFAULT 0.0,
    specifications TEXT    NOT NULL DEFAULT '{}',
    created_at     TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_order_items_order ON order_items(order_id);

-- =========================================
-- AUDIT LOGS (immutable, append-only)
-- =========================================
CREATE TABLE IF NOT EXISTS audit_logs (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    user_id    TEXT    NOT NULL DEFAULT 'local',
    action     TEXT    NOT NULL,
    payload    TEXT    NOT NULL DEFAULT '{}',
    ip_address TEXT,
    session_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_audit_user      ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_action    ON audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_logs(timestamp);
