CREATE TABLE IF NOT EXISTS inventories (
    character_id INTEGER PRIMARY KEY NOT NULL REFERENCES characters(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS inventory_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    character_id INTEGER NOT NULL REFERENCES inventories(character_id) ON DELETE CASCADE,
    item_definition_id TEXT NOT NULL REFERENCES item_definitions(id) ON DELETE CASCADE,
    equipped BOOLEAN NOT NULL DEFAULT 1
);
