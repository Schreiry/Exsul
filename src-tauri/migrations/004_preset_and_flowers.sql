-- ============================================================
-- EXSUL: Migration 004 — Preset System, Trusted Nodes, Flowers
-- ============================================================

-- =========================================
-- APP PRESET (stored in local_config)
-- values: 'flowers' | 'ochokochi' | 'balanced'
-- =========================================
INSERT OR IGNORE INTO local_config (key, value) VALUES ('app_preset', 'balanced');

-- =========================================
-- TRUSTED NODES (cryptographic whitelist)
-- node_id = UUID assigned on first launch
-- ip_hint = Tailscale / static IP for auto-connect
-- =========================================
CREATE TABLE IF NOT EXISTS trusted_nodes (
    node_id    TEXT PRIMARY KEY,
    alias      TEXT,
    ip_hint    TEXT,
    added_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_trusted_nodes_ip ON trusted_nodes(ip_hint);

-- =========================================
-- FLOWER SORTS (domain catalog)
-- raw_stock  = unpackaged stems
-- pkg_stock  = completed packs
-- =========================================
CREATE TABLE IF NOT EXISTS flower_sorts (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    variety     TEXT,
    color_hex   TEXT,
    raw_stock   INTEGER NOT NULL DEFAULT 0,
    pkg_stock   INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_flower_sorts_name ON flower_sorts(name);

-- =========================================
-- FLOWER CONSTANTS (detached island)
-- weight_per_flower  kg per single stem
-- flowers_per_pack   stems bundled per pack
-- price_per_pack     selling price per pack
-- price_per_flower   selling price per single stem
-- =========================================
CREATE TABLE IF NOT EXISTS flower_constants (
    key   TEXT PRIMARY KEY,
    value REAL NOT NULL DEFAULT 0
);

INSERT OR IGNORE INTO flower_constants (key, value) VALUES
    ('weight_per_flower', 0.05),
    ('flowers_per_pack',  10),
    ('price_per_pack',    500.0),
    ('price_per_flower',  50.0);

-- =========================================
-- SEED: 60 non-rose flower sorts
-- =========================================
INSERT OR IGNORE INTO flower_sorts (id, name, variety, color_hex) VALUES
    ('fs-001', 'Пион',         'Coral Sunset',       '#FF6B6B'),
    ('fs-002', 'Пион',         'Sarah Bernhardt',     '#FFB3C6'),
    ('fs-003', 'Пион',         'Festiva Maxima',      '#FFFFFF'),
    ('fs-004', 'Тюльпан',      'Queen of Night',      '#2D1B4E'),
    ('fs-005', 'Тюльпан',      'Parrot Mix',          '#FF8C42'),
    ('fs-006', 'Тюльпан',      'White Dream',         '#F5F5F5'),
    ('fs-007', 'Тюльпан',      'Purple Prince',       '#7B2FBE'),
    ('fs-008', 'Гортензия',    'Incrediball',         '#C8E6C9'),
    ('fs-009', 'Гортензия',    'Endless Summer',      '#90CAF9'),
    ('fs-010', 'Гортензия',    'Limelight',           '#F0FFF4'),
    ('fs-011', 'Лилия',        'Stargazer',           '#E91E63'),
    ('fs-012', 'Лилия',        'Casa Blanca',         '#FFFFFF'),
    ('fs-013', 'Лилия',        'Sunset Mix',          '#FF9800'),
    ('fs-014', 'Лилия',        'Purple Rain',         '#9C27B0'),
    ('fs-015', 'Ранункулюс',   'Cloni Yellow',        '#FFD54F'),
    ('fs-016', 'Ранункулюс',   'Cloni Salmon',        '#FFAB91'),
    ('fs-017', 'Ранункулюс',   'Cloni White',         '#FAFAFA'),
    ('fs-018', 'Ранункулюс',   'Cloni Red',           '#E53935'),
    ('fs-019', 'Эустома',      'Echo White',          '#FFFFFF'),
    ('fs-020', 'Эустома',      'Echo Purple',         '#CE93D8'),
    ('fs-021', 'Эустома',      'Echo Champagne',      '#F5DEB3'),
    ('fs-022', 'Фрезия',       'White',               '#FFFDE7'),
    ('fs-023', 'Фрезия',       'Yellow',              '#FFF176'),
    ('fs-024', 'Фрезия',       'Pink',                '#F48FB1'),
    ('fs-025', 'Хризантема',   'Anastasia Bronze',    '#CD853F'),
    ('fs-026', 'Хризантема',   'Yoko Ono White',      '#F5F5F5'),
    ('fs-027', 'Хризантема',   'Tom Pearce Green',    '#AED581'),
    ('fs-028', 'Нарцисс',      'Tête-à-tête',         '#FFD600'),
    ('fs-029', 'Нарцисс',      'Thalia',              '#FFFDE7'),
    ('fs-030', 'Нарцисс',      'Ice Follies',         '#FFFFFF'),
    ('fs-031', 'Ирис',         'Blue Magic',          '#3F51B5'),
    ('fs-032', 'Ирис',         'White Wedgwood',      '#E8EAF6'),
    ('fs-033', 'Ирис',         'Purple Sensation',    '#673AB7'),
    ('fs-034', 'Альстромерия', 'Yellow King',         '#FFEE58'),
    ('fs-035', 'Альстромерия', 'Salmon Pink',         '#FFAB91'),
    ('fs-036', 'Альстромерия', 'White Blush',         '#FCE4EC'),
    ('fs-037', 'Гвоздика',     'Scarlet Red',         '#E53935'),
    ('fs-038', 'Гвоздика',     'Soft Pink',           '#F48FB1'),
    ('fs-039', 'Гвоздика',     'Pure White',          '#FFFFFF'),
    ('fs-040', 'Антуриум',     'Classic Red',         '#D32F2F'),
    ('fs-041', 'Антуриум',     'Champion White',      '#ECEFF1'),
    ('fs-042', 'Антуриум',     'Purple Love',         '#880E4F'),
    ('fs-043', 'Снежноягодник','White Pearl',         '#F5F5F5'),
    ('fs-044', 'Эрингиум',     'Blue Hobbit',         '#1565C0'),
    ('fs-045', 'Вероника',     'Inspire White',       '#FFFFFF'),
    ('fs-046', 'Вероника',     'Inspire Blue',        '#1E88E5'),
    ('fs-047', 'Лаванда',      'Grosso',              '#B39DDB'),
    ('fs-048', 'Маттиола',     'Katz Cream',          '#FFF8E1'),
    ('fs-049', 'Маттиола',     'Katz Deep Purple',    '#4A148C'),
    ('fs-050', 'Амарант',      'Hot Bisccus',         '#BF360C'),
    ('fs-051', 'Целозия',      'Flamingo Feather',    '#F06292'),
    ('fs-052', 'Целозия',      'Bombay Purple',       '#7B1FA2'),
    ('fs-053', 'Ромашка',      'Classic White',       '#FFFFFF'),
    ('fs-054', 'Подсолнух',    'Sunrich Orange',      '#FF8F00'),
    ('fs-055', 'Подсолнух',    'Moulin Rouge',        '#B71C1C'),
    ('fs-056', 'Буплерум',     'Griffithii',          '#CDDC39'),
    ('fs-057', 'Статица',      'QIS White',           '#FAFAFA'),
    ('fs-058', 'Статица',      'QIS Purple',          '#7E57C2'),
    ('fs-059', 'Зелень Эвкалипт','Baby Blue',         '#80CBC4'),
    ('fs-060', 'Зелень Ричардия','Flame',             '#FF7043');
