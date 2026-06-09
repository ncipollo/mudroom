CREATE TABLE IF NOT EXISTS entity_faction_relations (
    entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    faction_id TEXT NOT NULL,
    relation TEXT NOT NULL,
    PRIMARY KEY (entity_id, faction_id)
);
