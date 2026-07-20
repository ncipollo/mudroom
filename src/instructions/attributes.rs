pub fn render() -> String {
    [
        "mudroom instructions attributes — attributes.toml reference",
        "",
        "Location:",
        "  <mud-dir>/attributes.toml — a single file containing every [[attributes]]",
        "  entry for the mud. Unlike abilities/classes/entities, attributes are not",
        "  split across a subfolder.",
        "",
        "Each [[attributes]] entry defines one stat or resource that entities can",
        "have. The `id` is the key used to reference this attribute elsewhere, such",
        "as an ability's effect_type or a class's [[attributes]] override.",
        "",
        "Fields:",
        "  id                  string — unique identifier for this attribute, used to",
        "                      reference it from abilities, classes, etc.",
        "  title               string — display name shown to players.",
        "  description         string — longer text shown to players (e.g. in help).",
        "  min_value           i64 — the minimum allowed value for this attribute.",
        "                      This is a global floor; classes may narrow it further.",
        "  max_value           i64 — the maximum allowed value for this attribute.",
        "                      This is a global ceiling; classes may narrow it further.",
        "  attribute_type      enum — categorizes how the engine treats this",
        "                      attribute mechanically. See values below.",
        "  attribute_category  enum — groups the attribute for display/organization",
        "                      purposes. See values below.",
        "  reset_condition     enum, optional — when this attribute's value resets",
        "                      to its default during engagements. Defaults to",
        "                      each_engagement_turn if omitted. See values below.",
        "",
        "attribute_type values:",
        "  hp     — a hit point pool; tracks damage an entity can sustain.",
        "  mp     — a mana/energy pool; tracks resources spent on abilities.",
        "  level  — an entity's overall experience level.",
        "  xp     — accumulated experience points.",
        "  stat   — a general-purpose stat (e.g. strength, dexterity) that does not",
        "           fit the other categories.",
        "",
        "attribute_category values:",
        "  life     — resources tied to survival, such as hp.",
        "  speed    — attributes that affect turn order or action speed.",
        "  general  — everything else (stats, level, xp, mp, etc).",
        "",
        "reset_condition values:",
        "  each_engagement_turn  — the attribute resets at the start of every turn",
        "                          within an engagement (battle). This is the",
        "                          default when the field is omitted.",
        "  end_of_engagement     — the attribute resets once, when the engagement",
        "                          ends.",
        "  never                 — the attribute is never automatically reset; its",
        "                          value persists across turns and engagements.",
        "",
        "Example (from muds/basic/attributes.toml):",
        "",
        "  [[attributes]]",
        "  id = \"hp\"",
        "  title = \"Hit Points\"",
        "  description = \"The amount of damage you can sustain before falling.\"",
        "  min_value = 0",
        "  max_value = 999",
        "  attribute_type = \"hp\"",
        "  attribute_category = \"life\"",
        "  reset_condition = \"never\"",
        "",
        "  [[attributes]]",
        "  id = \"strength\"",
        "  title = \"Strength\"",
        "  description = \"Raw physical power and carrying capacity.\"",
        "  min_value = 1",
        "  max_value = 20",
        "  attribute_type = \"stat\"",
        "  attribute_category = \"general\"",
        "  # reset_condition omitted — defaults to each_engagement_turn",
    ]
    .join("\n")
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
        assert!(text.contains("attribute_type"));
        assert!(text.contains("attribute_category"));
        assert!(text.contains("reset_condition"));
    }

    #[test]
    fn render_lists_attribute_type_values() {
        let text = render();
        assert!(text.contains("hp     —"));
        assert!(text.contains("mp     —"));
        assert!(text.contains("level  —"));
        assert!(text.contains("xp     —"));
        assert!(text.contains("stat   —"));
    }

    #[test]
    fn render_lists_attribute_category_values() {
        let text = render();
        assert!(text.contains("life     —"));
        assert!(text.contains("speed    —"));
        assert!(text.contains("general  —"));
    }

    #[test]
    fn render_lists_reset_condition_values() {
        let text = render();
        assert!(text.contains("each_engagement_turn"));
        assert!(text.contains("end_of_engagement"));
        assert!(text.contains("never"));
    }

    #[test]
    fn render_includes_example() {
        let text = render();
        assert!(text.contains("[[attributes]]"));
        assert!(text.contains("id = \"hp\""));
    }
}
