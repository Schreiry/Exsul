-- ============================================================
-- EXSUL: Event-Sourced Inventory Schema
-- Migration 001: Initial schema
-- ============================================================

PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

-- ===================
-- EVENT LEDGER (append-only, single source of truth)
-- ===================
CREATE TABLE IF NOT EXISTS events (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    aggregate_id    TEXT    NOT NULL,
    aggregate_type  TEXT    NOT NULL,
    event_type      TEXT    NOT NULL,
    data            TEXT    NOT NULL DEFAULT '{}',
    hlc_timestamp   TEXT    NOT NULL,
    node_id         TEXT    NOT NULL,
    version         INTEGER NOT NULL,
    created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),

    UNIQUE(aggregate_id, node_id, version)
);

CREATE INDEX IF NOT EXISTS idx_events_aggregate ON events(aggregate_id, version);
CREATE INDEX IF NOT EXISTS idx_events_hlc ON events(hlc_timestamp);
CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_node ON events(node_id);

-- ===================
-- MATERIALIZED PROJECTION: Items (current state)
-- ===================
CREATE TABLE IF NOT EXISTS items (
    id               TEXT    PRIMARY KEY,
    name             TEXT    NOT NULL,
    category         TEXT    NOT NULL DEFAULT 'uncategorized',
    initial_price    REAL    NOT NULL DEFAULT 0.0,
    current_price    REAL    NOT NULL DEFAULT 0.0,
    production_cost  REAL    NOT NULL DEFAULT 0.0,
    current_stock    INTEGER NOT NULL DEFAULT 0,
    sold_count       INTEGER NOT NULL DEFAULT 0,
    revenue          REAL    NOT NULL DEFAULT 0.0,
    created_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    updated_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_items_category ON items(category);
CREATE INDEX IF NOT EXISTS idx_items_name ON items(name);

-- ===================
-- MATERIALIZED PROJECTION: Price History
-- ===================
CREATE TABLE IF NOT EXISTS item_prices (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id       TEXT    NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    price         REAL    NOT NULL,
    effective_at  TEXT    NOT NULL,
    event_id      INTEGER NOT NULL REFERENCES events(id),
    created_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_item_prices_item ON item_prices(item_id, effective_at);

-- ===================
-- SYNC STATE: Vector clocks per peer
-- ===================
CREATE TABLE IF NOT EXISTS sync_state (
    peer_node_id     TEXT    PRIMARY KEY,
    last_hlc         TEXT    NOT NULL,
    last_event_id    INTEGER NOT NULL DEFAULT 0,
    last_synced_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

-- ===================
-- LOCAL CONFIG
-- ===================
CREATE TABLE IF NOT EXISTS local_config (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL
);

-- ===================
-- SCHEMA MIGRATIONS TRACKER
-- ===================
CREATE TABLE IF NOT EXISTS schema_migrations (
    version  INTEGER PRIMARY KEY,
    name     TEXT    NOT NULL,
    applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);
