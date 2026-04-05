-- ============================================================
-- EXSUL: Projection Rebuild Triggers
-- Migration 002: Auto-project events into materialized tables
-- ============================================================

-- ----- ItemCreated -----
CREATE TRIGGER IF NOT EXISTS trg_project_item_created
AFTER INSERT ON events
WHEN NEW.event_type = 'ItemCreated'
BEGIN
    INSERT OR REPLACE INTO items (
        id, name, category, initial_price, current_price,
        production_cost, current_stock, sold_count, revenue, created_at, updated_at
    ) VALUES (
        NEW.aggregate_id,
        json_extract(NEW.data, '$.name'),
        COALESCE(json_extract(NEW.data, '$.category'), 'uncategorized'),
        COALESCE(json_extract(NEW.data, '$.price'), 0.0),
        COALESCE(json_extract(NEW.data, '$.price'), 0.0),
        COALESCE(json_extract(NEW.data, '$.production_cost'), 0.0),
        COALESCE(json_extract(NEW.data, '$.initial_stock'), 0),
        0,
        0.0,
        NEW.created_at,
        NEW.created_at
    );

    INSERT INTO item_prices (item_id, price, effective_at, event_id)
    VALUES (
        NEW.aggregate_id,
        COALESCE(json_extract(NEW.data, '$.price'), 0.0),
        NEW.hlc_timestamp,
        NEW.id
    );
END;

-- ----- StockAdjusted -----
CREATE TRIGGER IF NOT EXISTS trg_project_stock_adjusted
AFTER INSERT ON events
WHEN NEW.event_type = 'StockAdjusted'
BEGIN
    UPDATE items SET
        current_stock = current_stock + COALESCE(json_extract(NEW.data, '$.delta'), 0),
        updated_at = NEW.created_at
    WHERE id = NEW.aggregate_id;
END;

-- ----- PriceChanged -----
CREATE TRIGGER IF NOT EXISTS trg_project_price_changed
AFTER INSERT ON events
WHEN NEW.event_type = 'PriceChanged'
BEGIN
    UPDATE items SET
        current_price = json_extract(NEW.data, '$.new_price'),
        updated_at = NEW.created_at
    WHERE id = NEW.aggregate_id;

    INSERT INTO item_prices (item_id, price, effective_at, event_id)
    VALUES (
        NEW.aggregate_id,
        json_extract(NEW.data, '$.new_price'),
        NEW.hlc_timestamp,
        NEW.id
    );
END;

-- ----- SaleRecorded -----
CREATE TRIGGER IF NOT EXISTS trg_project_sale_recorded
AFTER INSERT ON events
WHEN NEW.event_type = 'SaleRecorded'
BEGIN
    UPDATE items SET
        current_stock = current_stock - COALESCE(json_extract(NEW.data, '$.quantity'), 1),
        sold_count = sold_count + COALESCE(json_extract(NEW.data, '$.quantity'), 1),
        revenue = revenue + (
            COALESCE(json_extract(NEW.data, '$.sale_price'), current_price)
            * COALESCE(json_extract(NEW.data, '$.quantity'), 1)
        ),
        updated_at = NEW.created_at
    WHERE id = NEW.aggregate_id;
END;

-- ----- ItemUpdated (name, category, production_cost) -----
CREATE TRIGGER IF NOT EXISTS trg_project_item_updated
AFTER INSERT ON events
WHEN NEW.event_type = 'ItemUpdated'
BEGIN
    UPDATE items SET
        name = COALESCE(json_extract(NEW.data, '$.name'), name),
        category = COALESCE(json_extract(NEW.data, '$.category'), category),
        production_cost = COALESCE(json_extract(NEW.data, '$.production_cost'), production_cost),
        updated_at = NEW.created_at
    WHERE id = NEW.aggregate_id;
END;
