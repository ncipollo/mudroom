ALTER TABLE item_definitions
    ADD COLUMN alternate_names_json TEXT NOT NULL DEFAULT '[]';
