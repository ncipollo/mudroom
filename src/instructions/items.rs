pub fn render() -> String {
    let sections = [
        header(),
        fields_section(),
        use_type_section(),
        equipped_bonuses_section(),
        use_effects_section(),
        example_section(),
    ];
    sections.join("\n\n")
}

fn header() -> String {
    r#"mudroom instructions items — item config file reference

Item files live at items/*.toml under a mud's config directory (e.g.
muds/basic/items/medicine.toml). Each file defines one item. The
item's id is derived from its file path relative to the items directory
(extension stripped) — it is not set in the TOML itself."#
        .to_string()
}

fn fields_section() -> String {
    r#"Top-level fields:
  name                 string            Display name.
  description          string, optional  Short description.
  use_type             string            "used" or "passive". See
                                          "use_type values" below.
  item_type            string            Free-form, mud-defined category
                                          (e.g. "weapon", "armor",
                                          "medicine" — anything).
  equipped_bonuses     table             What the item grants while
                                          equipped (passive items). Defaults
                                          to an empty table if omitted. See
                                          "equipped_bonuses" below.
  [[use_effects]]      array of tables   What happens when a used item is
                                          consumed. Defaults to [] if
                                          omitted. See "use_effects" below."#
        .to_string()
}

fn use_type_section() -> String {
    r#"use_type values:
  "used"      Consumed on use to adjust attributes or apply an effect.
  "passive"   Grants attribute bonuses and abilities while equipped."#
        .to_string()
}

fn equipped_bonuses_section() -> String {
    r#"equipped_bonuses fields:
  attributes   array of tables   Flat attribute adjustments granted while
                                  the item is equipped, e.g. max health + 10
                                  or +1 to strength. Each entry is
                                  { attribute_id = "<id>", amount = <int> }.
                                  Defaults to [] if omitted.
  equipped     array of strings  Ability ids granted to the character while
                                  the item is equipped. Defaults to [] if
                                  omitted."#
        .to_string()
}

fn use_effects_section() -> String {
    r#"[[use_effects]] variants:
  { type = "stat_boost", attribute_id = "<id>", operator = "add" | "subtract" | "multiply" | "divide" }
    Applies an attribute-scaling bonus when the item is used.
  { type = "apply_effect", name = "...", trigger_info = { ... }, effect_type = { ... }, description = { ... }, scope = "..." }
    Applies an effect when the item is used — the same effect shape used
    by abilities. See `mudroom instructions abilities` for the full
    trigger_info/effect_type/scope field reference."#
        .to_string()
}

fn example_section() -> String {
    r#"Complete annotated examples:

muds/basic/items/medicine.toml (used):

  name = "Medicine"
  description = "A restorative brew."
  use_type = "used"                       # consumed on use
  item_type = "consumable"

  [[use_effects]]
  type = "apply_effect"                   # applies an Effect on use
  name = "heal_hp"
  trigger_info = { type = "once" }
  effect_type = { type = "attribute_update", attribute_id = "hp", value = 20 }
  description = { text = "Restores {{value}} HP" }

muds/basic/items/leather_vest.toml (passive):

  name = "Leather Vest"
  description = "A simple protective vest."
  use_type = "passive"                    # grants bonuses while equipped
  item_type = "armor"

  [[equipped_bonuses.attributes]]
  attribute_id = "constitution"
  amount = 2"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_documents_top_level_fields() {
        let text = render();
        assert!(text.contains("use_type"));
        assert!(text.contains("item_type"));
        assert!(text.contains("equipped_bonuses"));
        assert!(text.contains("[[use_effects]]"));
        assert!(text.contains("equipped"));
    }

    #[test]
    fn render_documents_use_type_values() {
        let text = render();
        assert!(text.contains("\"used\""));
        assert!(text.contains("\"passive\""));
    }

    #[test]
    fn render_documents_use_effect_variants() {
        let text = render();
        assert!(text.contains("stat_boost"));
        assert!(text.contains("apply_effect"));
    }

    #[test]
    fn render_includes_worked_example() {
        let text = render();
        assert!(text.contains("Complete annotated examples"));
        assert!(text.contains("medicine.toml"));
        assert!(text.contains("leather_vest.toml"));
    }
}
