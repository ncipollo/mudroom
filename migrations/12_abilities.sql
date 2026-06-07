CREATE TABLE IF NOT EXISTS abilities (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    effects_json TEXT NOT NULL DEFAULT '[]',
    costs_json TEXT NOT NULL DEFAULT '[]',
    modifiers_json TEXT NOT NULL DEFAULT '[]',
    engagement_types_json TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS entity_abilities (
    entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    ability_id TEXT NOT NULL REFERENCES abilities(id) ON DELETE CASCADE,
    PRIMARY KEY (entity_id, ability_id)
);
