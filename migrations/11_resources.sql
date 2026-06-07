CREATE TABLE IF NOT EXISTS resource_definitions (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    min_value INTEGER NOT NULL,
    max_value INTEGER NOT NULL,
    lifespan TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS entity_resources (
    entity_id INTEGER NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    resource_id TEXT NOT NULL REFERENCES resource_definitions(id) ON DELETE CASCADE,
    current_value INTEGER NOT NULL,
    PRIMARY KEY (entity_id, resource_id)
);
