CREATE TABLE IF NOT EXISTS item_definitions (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    use_type TEXT NOT NULL,
    item_type TEXT NOT NULL,
    attribute_bonuses_json TEXT NOT NULL DEFAULT '[]',
    use_effects_json TEXT NOT NULL DEFAULT '[]',
    equipped_abilities_json TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS world_loot (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_definition_id TEXT NOT NULL REFERENCES item_definitions(id) ON DELETE CASCADE,
    world_id TEXT NOT NULL REFERENCES worlds(id),
    dungeon_id TEXT NOT NULL REFERENCES dungeons(id),
    room_id TEXT NOT NULL REFERENCES rooms(id)
);
