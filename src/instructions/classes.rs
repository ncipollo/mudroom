pub fn render() -> String {
    r#"mudroom instructions classes — class config file (classes/*.toml) reference

Location: <mud-dir>/classes/<class-id>.toml — one file per class. The filename
(without .toml) is the class ID, used with `mudroom players reset <player> <class>`.
You can override the ID explicitly with a top-level `id` field in the file.

Example (muds/basic/classes/survivor.toml):

  name = "Survivor"
  description = "A scrappy melee brawler who clawed their way through the wasteland."
  innate_abilities = ["basic_attack", "sawed_off_shotgun", "defend"]

  [[attributes]]
  definition_id = "hp"
  min_value = 0
  max_value = 120
  current_value = 120

  [[attributes]]
  definition_id = "mp"
  min_value = 0
  max_value = 20
  current_value = 20

Fields:
  name              — display name shown to players (string, required)
  description       — player-visible flavor text (string, optional)
  innate_abilities  — array of ability IDs granted by this class. Each ID must
                      match the filename (without .toml) of a file under
                      abilities/ — e.g. "basic_attack" resolves to
                      abilities/basic_attack.toml.
  [[attributes]]    — repeated table overriding one attribute for this class:
    definition_id   — must match the `id` of an entry in attributes.toml
    min_value       — lower bound for this class; can be tighter than the
                      global attribute's range
    max_value       — upper bound for this class; can be tighter than the
                      global attribute's range
    current_value   — value the attribute is set to when this class is applied

Key behaviors:
  - Only attributes listed under [[attributes]] are overridden for this class;
    any attribute defined in attributes.toml but omitted here keeps its global
    default range and value.
  - innate_abilities are granted to the player's entity when the class is
    applied via `mudroom players reset <player> <class>`.
  - The class ID (filename without .toml, or the `id` field if set) is the
    <class> argument passed to `mudroom players reset`."#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_documents_file_location() {
        let text = render();
        assert!(text.contains("classes/<class-id>.toml"));
        assert!(text.contains("mudroom players reset"));
    }

    #[test]
    fn render_documents_fields() {
        let text = render();
        assert!(text.contains("innate_abilities"));
        assert!(text.contains("definition_id"));
        assert!(text.contains("min_value"));
        assert!(text.contains("max_value"));
        assert!(text.contains("current_value"));
        assert!(text.contains("attributes.toml"));
    }
}
