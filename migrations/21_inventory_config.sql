ALTER TABLE inventories ADD COLUMN inventory_type TEXT NOT NULL DEFAULT 'inventory';
ALTER TABLE inventory_items ADD COLUMN slot_name TEXT;
