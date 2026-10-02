pub fn render() -> String {
    r#"mudroom instructions attributes — attributes.toml reference

Location:
  <mud-dir>/attributes.toml — a single file containing every [[attributes]]
  entry for the mud. Unlike abilities/classes/entities, attributes are not
  split across a subfolder.

Each [[attributes]] entry defines one stat or resource that entities can
have. The `id` is the key used to reference this attribute elsewhere, such
as an ability's effect_type or a class's [[attributes]] override.

Fields:
  id                  string — unique identifier for this attribute, used to
                      reference it from abilities, classes, etc.
  title               string — display name shown to players.
  description         string — longer text shown to players (e.g. in help).
  min_value           i64 — the minimum allowed value for this attribute.
                      This is a global floor; classes may narrow it further.
  max_value           i64 — the maximum allowed value for this attribute.
                      This is a global ceiling; classes may narrow it further.

Example (from muds/basic/attributes.toml):

  [[attributes]]
  id = "hp"
  title = "Hit Points"
  description = "The amount of damage you can sustain before falling."
  min_value = 0
  max_value = 999

  [[attributes]]
  id = "strength"
  title = "Strength"
  description = "Raw physical power and carrying capacity."
  min_value = 1
  max_value = 20"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_field_reference() {
        let text = render();
        assert!(text.contains("attributes.toml"));
        assert!(text.contains("min_value"));
        assert!(text.contains("max_value"));
    }

    #[test]
    fn render_includes_example() {
        let text = render();
        assert!(text.contains("[[attributes]]"));
        assert!(text.contains("id = \"hp\""));
    }
}
