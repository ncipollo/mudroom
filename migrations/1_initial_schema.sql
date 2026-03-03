CREATE TABLE IF NOT EXISTS worlds (
    id TEXT PRIMARY KEY NOT NULL
);

CREATE TABLE IF NOT EXISTS dungeons (
    id TEXT PRIMARY KEY NOT NULL,
    world_id TEXT NOT NULL REFERENCES worlds(id)
);

CREATE TABLE IF NOT EXISTS rooms (
    id TEXT PRIMARY KEY NOT NULL,
    dungeon_id TEXT NOT NULL REFERENCES dungeons(id),
    description_json TEXT NOT NULL,
    north_json TEXT,
    south_json TEXT,
    east_json TEXT,
    west_json TEXT
);

CREATE TABLE IF NOT EXISTS entities (
    id INTEGER PRIMARY KEY NOT NULL,
    entity_type TEXT NOT NULL,
    world_id TEXT NOT NULL REFERENCES worlds(id),
    dungeon_id TEXT NOT NULL REFERENCES dungeons(id),
    room_id TEXT NOT NULL REFERENCES rooms(id)
);

CREATE TABLE IF NOT EXISTS attributes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    definition_id TEXT NOT NULL,
    min_value INTEGER NOT NULL,
    max_value INTEGER NOT NULL,
    current_value INTEGER NOT NULL
);
