CREATE TABLE IF NOT EXISTS feature_definitions (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    default_state TEXT NOT NULL,
    states_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS room_features (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    feature_definition_id TEXT NOT NULL REFERENCES feature_definitions(id) ON DELETE CASCADE,
    world_id TEXT NOT NULL REFERENCES worlds(id),
    dungeon_id TEXT NOT NULL REFERENCES dungeons(id),
    room_id TEXT NOT NULL REFERENCES rooms(id),
    current_state TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_room_features_location
    ON room_features (world_id, dungeon_id, room_id, feature_definition_id);
