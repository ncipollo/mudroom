pub fn render() -> String {
    let lines = vec![
        "mudroom instructions entities — entities/*.toml file reference".to_string(),
        String::new(),
        "Entity config files define the NPCs, creatures, and objects that populate the".to_string(),
        "world: their type, persona, dialog, starting attributes, and combat behavior.".to_string(),
        String::new(),
        "Location: <mud-dir>/entities/<entity-id>.toml — one file per entity.".to_string(),
        "The entity ID defaults to the file path relative to the mud dir, without the".to_string(),
        "\".toml\" extension (e.g. \"entities/innkeeper.toml\" -> \"entities/innkeeper\"),"
            .to_string(),
        "unless overridden by the `id` field. Room files reference entities by this ID".to_string(),
        "in their `entities` array, e.g. entities = [\"entities/innkeeper\"].".to_string(),
        String::new(),
        "Example (muds/basic/entities/innkeeper.toml):".to_string(),
        String::new(),
        "  entity_type = \"character\"".to_string(),
        String::new(),
        "  [persona]".to_string(),
        "  type = \"standard\"".to_string(),
        "  dialog_file = \"innkeeper_dialog.md\"".to_string(),
        String::new(),
        "  [[attributes]]".to_string(),
        "  definition_id = \"hp\"".to_string(),
        "  min_value = 0".to_string(),
        "  max_value = 100".to_string(),
        "  current_value = 100".to_string(),
        String::new(),
        "Top-level fields:".to_string(),
        String::new(),
        "  id             Optional. Overrides the derived entity ID.".to_string(),
        "  name           Optional. Display name. Defaults to a name derived from the".to_string(),
        "                 filename if omitted.".to_string(),
        "  entity_type    Required. One of \"character\", \"enemy\", \"object\".".to_string(),
        "  description    Optional. Flavor text describing the entity.".to_string(),
        "  persona        Optional. See [persona] below.".to_string(),
        "  attributes     Optional list of [[attributes]] tables. See below.".to_string(),
        "  innate_abilities  Optional list of ability IDs the entity always has".to_string(),
        "                 available in combat, without needing to learn them, e.g.".to_string(),
        "                 innate_abilities = [\"swing_ax\", \"scream_nonsense\", \"defend\"]."
            .to_string(),
        "  factions       Optional list of faction IDs this entity belongs to.".to_string(),
        "  faction_relations  Optional. See [faction_relations.factions] below.".to_string(),
        "  battle_ai      Optional. See [battle_ai] below.".to_string(),
        "  entity_effects Optional list of standing effects applied to the entity".to_string(),
        "                 (advanced; same effect structure used by abilities).".to_string(),
        String::new(),
        "[persona] sub-table:".to_string(),
        String::new(),
        "  type           Required. \"standard\" or \"agent\".".to_string(),
        String::new(),
        "  type = \"standard\" — a scripted dialog tree:".to_string(),
        "    dialog_file  Path to a Markdown dialog tree, relative to the entity file's"
            .to_string(),
        "                 directory, e.g. dialog_file = \"innkeeper_dialog.md\".".to_string(),
        "    dialog_tree  Alternative to dialog_file: an inline [persona.dialog_tree]".to_string(),
        "                 table with the same structure the Markdown file parses into.".to_string(),
        String::new(),
        "  type = \"agent\" — an LLM-driven persona:".to_string(),
        "    agent_type   Optional. Selects the agent provider/behavior. Defaults to".to_string(),
        "                 \"default\".".to_string(),
        "    persona_file Path to a Markdown persona file, relative to the entity".to_string(),
        "                 file's directory, e.g. persona_file = \"mysterious_man.md\".".to_string(),
        "                 See `mudroom instructions mud-config` for the persona file".to_string(),
        "                 format (front matter, preamble, conditional sections).".to_string(),
        String::new(),
        "[[attributes]] (repeatable table, same shape as class attribute overrides):".to_string(),
        String::new(),
        "  definition_id  ID of an attribute defined in attributes.toml, e.g. \"hp\".".to_string(),
        "  min_value      Minimum value for the attribute.".to_string(),
        "  max_value      Maximum value for the attribute.".to_string(),
        "  current_value  Starting value for the attribute.".to_string(),
        String::new(),
        "[faction_relations.factions] sub-table:".to_string(),
        String::new(),
        "  Maps a faction ID to this entity's relation towards it: \"hostile\",".to_string(),
        "  \"unfriendly\", \"friendly\", or \"non_interactive\" (default when unset).".to_string(),
        String::new(),
        "[battle_ai] sub-table:".to_string(),
        String::new(),
        "  ai_type        Optional. \"none\" (default) or \"simple_random\".".to_string(),
        String::new(),
        "Combat entity example (muds/basic/entities/zombie.toml):".to_string(),
        String::new(),
        "  entity_type = \"enemy\"".to_string(),
        "  factions = [\"enemy\"]".to_string(),
        String::new(),
        "  name = \"Zombie\"".to_string(),
        "  description = \"A shambling corpse, visibly decayed.\"".to_string(),
        "  innate_abilities = [\"basic_attack\", \"defend\"]".to_string(),
        String::new(),
        "  [battle_ai]".to_string(),
        "  ai_type = \"simple_random\"".to_string(),
        String::new(),
        "  [[attributes]]".to_string(),
        "  definition_id = \"hp\"".to_string(),
        "  min_value = 0".to_string(),
        "  max_value = 50".to_string(),
        "  current_value = 50".to_string(),
        String::new(),
        "  [faction_relations.factions]".to_string(),
        "  player = \"hostile\"".to_string(),
    ];
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_header() {
        let text = render();
        assert!(text.contains("mudroom instructions entities"));
        assert!(text.contains("entities/*.toml"));
    }

    #[test]
    fn render_documents_top_level_fields() {
        let text = render();
        assert!(text.contains("entity_type"));
        assert!(text.contains("[persona]"));
        assert!(text.contains("[[attributes]]"));
    }

    #[test]
    fn render_documents_persona_fields() {
        let text = render();
        assert!(text.contains("dialog_file"));
        assert!(text.contains("persona_file"));
        assert!(text.contains("\"standard\""));
        assert!(text.contains("\"agent\""));
    }

    #[test]
    fn render_documents_innate_abilities() {
        let text = render();
        assert!(text.contains("innate_abilities"));
    }

    #[test]
    fn render_documents_entity_id_reference_from_rooms() {
        let text = render();
        assert!(text.contains("entities ="));
    }
}
