ALTER TABLE entities ADD COLUMN config_id TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_entities_config_room
    ON entities (config_id, room_id)
    WHERE config_id IS NOT NULL;
