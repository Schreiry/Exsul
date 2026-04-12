-- Migration 012: Universal app_settings key-value table

CREATE TABLE IF NOT EXISTS app_settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    value_type  TEXT NOT NULL DEFAULT 'string',
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- Seed defaults for flower mode
INSERT OR IGNORE INTO app_settings (key, value, value_type) VALUES
    ('flower.stems_per_pack',   '10',       'number'),
    ('flower.weight_per_pack',  '0.5',      'number'),
    ('flower.pricing_mode',     'pack',     'string'),
    ('flower.price_per_pack',   '0',        'number'),
    ('flower.price_per_stem',   '0',        'number'),
    ('ui.theme_seed',           '#6b7280',  'string'),
    ('ui.color_mode',           'dark',     'string'),
    ('ui.dock_order_flowers',   '["dashboard","greenhouse","warehouse","orders","analytics","settings","sync"]', 'json');
