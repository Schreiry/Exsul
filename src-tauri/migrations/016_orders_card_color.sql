-- Migration 016: per-order card accent color for UI personalization (flowers preset).
-- Purely additive: existing rows get NULL, which the UI treats as "default theme styling".
ALTER TABLE orders ADD COLUMN card_color TEXT;
