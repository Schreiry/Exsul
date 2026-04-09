-- Migration 007: link categories to flower_sorts for unified inventory mode
ALTER TABLE categories ADD COLUMN sort_id TEXT REFERENCES flower_sorts(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_categories_sort_id ON categories(sort_id);
