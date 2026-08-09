pub fn render() -> String {
    r#"mudroom instructions entities — entities/*.toml file reference

Character config files define the NPCs, creatures, and objects that populate the
world: their type, persona, dialog, starting attributes, and combat behavior.

Location: <mud-dir>/entities/<character-id>.toml — one file per character.
The character ID defaults to the file path relative to the mud dir, without the
".toml" extension (e.g. "entities/innkeeper.toml" -> "entities/innkeeper"),
unless overridden by the `id` field. Room files reference entities by this ID
in their `entities` array, e.g. entities = ["entities/innkeeper"].

Example (muds/basic/entities/innkeeper.toml):

  entity_type = "character"

  [persona]
  type = "standard"
  dialog_file = "innkeeper_dialog.md"

  [[attributes]]
  definition_id = "hp"
  min_value = 0
  max_value = 100
  current_value = 100

Top-level fields:

  id             Optional. Overrides the derived character ID.
  name           Optional. Display name. Defaults to a name derived from the
                 filename if omitted.
  entity_type    Required. One of "character", "enemy".
  description    Optional. Flavor text describing the character.
  persona        Optional. See [persona] below.
  attributes     Optional list of [[attributes]] tables. See below.
  innate_abilities  Optional list of ability IDs the character always has
                 available in combat, without needing to learn them, e.g.
                 innate_abilities = ["swing_ax", "scream_nonsense", "defend"].
  factions       Optional list of faction IDs this character belongs to.
  faction_relations  Optional. See [faction_relations.factions] below.
  battle_ai      Optional. See [battle_ai] below.
  entity_effects Optional list of standing effects applied to the character
                 (advanced; same effect structure used by abilities).

[persona] sub-table:

  type           Required. "standard" or "agent".

  type = "standard" — a scripted dialog tree:
    dialog_file  Path to a Markdown dialog tree, relative to the character file's
                 directory, e.g. dialog_file = "innkeeper_dialog.md".
    dialog_tree  Alternative to dialog_file: an inline [persona.dialog_tree]
                 table with the same structure the Markdown file parses into.

  type = "agent" — an LLM-driven persona:
    agent_type   Optional. Selects the agent provider/behavior. Defaults to
                 "default".
    persona_file Path to a Markdown persona file, relative to the character
                 file's directory, e.g. persona_file = "mysterious_man.md".
                 See `mudroom instructions mud-config` for the persona file
                 format (front matter, preamble, conditional sections).

[[attributes]] (repeatable table, same shape as class attribute overrides):

  definition_id  ID of an attribute defined in attributes.toml, e.g. "hp".
  min_value      Minimum value for the attribute.
  max_value      Maximum value for the attribute.
  current_value  Starting value for the attribute.

[faction_relations.factions] sub-table:

  Maps a faction ID to this character's relation towards it: "hostile",
  "unfriendly", "friendly", or "non_interactive" (default when unset).

[battle_ai] sub-table:

  ai_type        Optional. "none" (default) or "simple_random".

Combat character example (muds/basic/entities/zombie.toml):

  entity_type = "enemy"
  factions = ["enemy"]

  name = "Zombie"
  description = "A shambling corpse, visibly decayed."
  innate_abilities = ["basic_attack", "defend"]

  [battle_ai]
  ai_type = "simple_random"

  [[attributes]]
  definition_id = "hp"
  min_value = 0
  max_value = 50
  current_value = 50

  [faction_relations.factions]
  player = "hostile""#
        .to_string()
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
