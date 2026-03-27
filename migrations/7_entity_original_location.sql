ALTER TABLE entities ADD COLUMN original_world_id TEXT;
ALTER TABLE entities ADD COLUMN original_dungeon_id TEXT;
ALTER TABLE entities ADD COLUMN original_room_id TEXT;

-- Backfill existing config entities with their current location as the original
UPDATE entities
SET original_world_id  = world_id,
    original_dungeon_id = dungeon_id,
    original_room_id   = room_id
WHERE config_id IS NOT NULL;

-- Replace the old config+room index with one keyed on original location
DROP INDEX IF EXISTS idx_entities_config_room;

CREATE UNIQUE INDEX IF NOT EXISTS idx_entities_config_original_room
    ON entities (config_id, original_world_id, original_dungeon_id, original_room_id)
    WHERE config_id IS NOT NULL;
