-- Migration 008: per-item card accent color for UI personalization
ALTER TABLE items ADD COLUMN card_color TEXT;
