-- Rename the entities table to characters and its type column.
ALTER TABLE entities RENAME TO characters;
ALTER TABLE characters RENAME COLUMN entity_type TO character_type;

-- Rename the join tables.
ALTER TABLE entity_effects RENAME TO character_effects;
ALTER TABLE entity_factions RENAME TO character_factions;
ALTER TABLE entity_resources RENAME TO character_resources;
ALTER TABLE entity_abilities RENAME TO character_abilities;
ALTER TABLE entity_faction_relations RENAME TO character_faction_relations;

-- Rename the join-table entity_id columns to character_id.
ALTER TABLE character_effects RENAME COLUMN entity_id TO character_id;
ALTER TABLE character_factions RENAME COLUMN entity_id TO character_id;
ALTER TABLE character_resources RENAME COLUMN entity_id TO character_id;
ALTER TABLE character_abilities RENAME COLUMN entity_id TO character_id;
ALTER TABLE character_faction_relations RENAME COLUMN entity_id TO character_id;

-- Rename the attributes join column.
ALTER TABLE attributes RENAME COLUMN entity_id TO character_id;