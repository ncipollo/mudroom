CREATE TABLE IF NOT EXISTS factions (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS entity_factions (
    entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    faction_id TEXT NOT NULL REFERENCES factions(id) ON DELETE CASCADE,
    PRIMARY KEY (entity_id, faction_id)
);
