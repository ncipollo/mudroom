ALTER TABLE world_loot ADD COLUMN original_world_id TEXT;
ALTER TABLE world_loot ADD COLUMN original_dungeon_id TEXT;
ALTER TABLE world_loot ADD COLUMN original_room_id TEXT;
ALTER TABLE world_loot ADD COLUMN taken BOOLEAN NOT NULL DEFAULT 0;

CREATE UNIQUE INDEX IF NOT EXISTS idx_world_loot_item_original_room
    ON world_loot (item_definition_id, original_world_id, original_dungeon_id, original_room_id)
    WHERE original_world_id IS NOT NULL;
