-- ============================================================
-- EXSUL: Migration 018 — Items shadow for flower_sorts
-- ============================================================
-- The `order_items.item_id` column was declared in migration 003
-- with a hard FK to `items(id)`. In the flowers preset, order_items
-- are inserted with item_id = flower_sort.id — which breaks the FK
-- because flower_sorts and items are independent tables.
--
-- Before this fix, the FK violation silently aborted the whole
-- "pack & create order" flow from the warehouse, leaving the order
-- with total_amount=0, no pack_assignment, and empty linked-packs.
--
-- Additive fix: for every existing flower_sort, insert a shadow row
-- into items with the same id so the FK is satisfied. Going forward,
-- insert_flower_sort writes the shadow row too, and insert_order_item
-- lazily creates one if it's still missing.
--
-- category = 'flower_sort' marks these as synthetic so the items-UI
-- can filter them out.
-- ============================================================

INSERT OR IGNORE INTO items (id, name, category, initial_price, current_price)
SELECT fs.id, fs.name, 'flower_sort', 0.0, 0.0
  FROM flower_sorts fs
 WHERE NOT EXISTS (SELECT 1 FROM items i WHERE i.id = fs.id);
