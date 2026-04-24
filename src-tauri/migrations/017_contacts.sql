-- ============================================================
-- Migration 017 — Contacts system (Phase E)
-- ============================================================
-- Introduces a reusable customer/contact directory that can be
-- linked to orders (optional — old orders without contact_id
-- keep working as before). Each contact may have many locations
-- (addresses), exactly one of which can be marked default.
--
-- Design notes:
--  * orders.contact_id/contact_location_id are soft FKs (no
--    REFERENCES) so the P2P projection never rejects a legacy
--    order that arrived before the contact's row synced.
--  * contact_locations.contact_id IS a hard FK with ON DELETE
--    CASCADE — locations without a parent contact are meaningless.
--  * All timestamps are ISO-8601 with millisecond precision to
--    match the rest of the schema and the HLC.
-- ============================================================

CREATE TABLE IF NOT EXISTS contacts (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    surname     TEXT,
    email       TEXT,
    phone       TEXT,
    company     TEXT,
    notes       TEXT,
    photo_path  TEXT,
    card_color  TEXT,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE TABLE IF NOT EXISTS contact_locations (
    id          TEXT PRIMARY KEY,
    contact_id  TEXT NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    label       TEXT,
    address     TEXT NOT NULL,
    is_default  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

-- Orders: attach optional contact linkage. Nullable — old orders
-- and orders created from the free-form "Имя клиента" field keep
-- working unchanged.
ALTER TABLE orders ADD COLUMN contact_id TEXT;
ALTER TABLE orders ADD COLUMN contact_location_id TEXT;

CREATE INDEX IF NOT EXISTS idx_orders_contact
    ON orders(contact_id);
CREATE INDEX IF NOT EXISTS idx_contact_locations_contact
    ON contact_locations(contact_id);
CREATE INDEX IF NOT EXISTS idx_contacts_name
    ON contacts(name);
