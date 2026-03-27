CREATE TABLE IF NOT EXISTS entity_effects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    effect_type_json TEXT NOT NULL,
    trigger_info_json TEXT NOT NULL,
    description_json TEXT NOT NULL
);
