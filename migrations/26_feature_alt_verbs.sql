ALTER TABLE feature_definitions
    ADD COLUMN alt_verbs_json TEXT NOT NULL DEFAULT '[]';
