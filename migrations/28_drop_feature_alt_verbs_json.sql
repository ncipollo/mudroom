-- alt_verbs moved from RoomFeature (feature-wide) onto FeatureState (per-state), where it
-- is now serialized inside states_json instead of its own column.
ALTER TABLE feature_definitions
    DROP COLUMN alt_verbs_json;
