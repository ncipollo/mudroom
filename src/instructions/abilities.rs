pub fn render() -> String {
    let sections = [
        header(),
        fields_section(),
        engagement_types_section(),
        costs_section(),
        role_section(),
        targets_section(),
        effects_section(),
        example_section(),
    ];
    sections.join("\n\n")
}

fn header() -> String {
    [
        "mudroom instructions abilities — ability config file reference",
        "",
        "Ability files live at abilities/*.toml under a mud's config directory",
        "(e.g. muds/basic/abilities/basic_attack.toml). Each file defines one ability.",
        "The ability's id is derived from its file path relative to the abilities",
        "directory (extension stripped) — it is not set in the TOML itself.",
    ]
    .join("\n")
}

fn fields_section() -> String {
    [
        "Top-level fields:",
        "  name              string            Display name.",
        "  description       string, optional  Short description.",
        "  action_text       string, optional  Narration template shown when the",
        "                                       ability is used, e.g.",
        "                                       \"{{entity}} swings ax at {{target}}\".",
        "  engagement_types  array of strings   Engagement types this ability can be",
        "                                       used in. See \"engagement_types\" below.",
        "  costs             array of tables    Resources consumed to use the ability.",
        "                                       See \"costs\" below. Use [] for none.",
        "  role              string, optional   The ability's role. See \"role\" below.",
        "                                       Defaults to \"attack\" if omitted.",
        "  targets           array of strings   Who the ability can target. See",
        "                                       \"targets\" below. Defaults to [] if",
        "                                       omitted.",
        "  modifiers         array of tables    Optional attribute-scaling modifiers.",
        "                                       Each has attribute_id (string) and",
        "                                       operator (\"add\", \"subtract\",",
        "                                       \"multiply\", or \"divide\"). Defaults to",
        "                                       [] if omitted.",
        "  [[effects]]       array of tables     One or more effect blocks describing",
        "                                       what the ability does. See \"effects\"",
        "                                       below.",
    ]
    .join("\n")
}

fn engagement_types_section() -> String {
    [
        "engagement_types values:",
        "  \"battle\"         Usable during a battle engagement.",
        "  \"conversation\"   Usable during a conversation engagement.",
    ]
    .join("\n")
}

fn costs_section() -> String {
    [
        "costs entries:",
        "  { type = \"resource\", resource_id = \"<attribute id>\", amount = <integer> }",
        "    Consumes `amount` of the entity's `resource_id` attribute (e.g. mp or",
        "    stamina) when the ability is used. Use an empty array (costs = [])",
        "    for abilities with no cost.",
    ]
    .join("\n")
}

fn role_section() -> String {
    [
        "role values:",
        "  \"attack\"   Offensive ability (default).",
        "  \"defend\"   Defensive ability.",
        "  \"world\"    Non-combat / world-interaction ability.",
    ]
    .join("\n")
}

fn targets_section() -> String {
    [
        "targets values:",
        "  \"opponent\"   Targets the opposing side.",
        "  \"allies\"     Targets allies.",
        "  \"self\"       Targets the acting entity.",
    ]
    .join("\n")
}

fn effects_section() -> String {
    [
        "[[effects]] fields:",
        "  name           string            Identifier for the effect.",
        "  trigger_info   inline table      When/how the effect fires. See",
        "                                   \"trigger_info variants\" below.",
        "  effect_type    inline table      What the effect does. See",
        "                                   \"effect_type variants\" below.",
        "  description    inline table,     Optional narration for the effect.",
        "                 optional          { text = \"...\" } supports variables",
        "                                   like {{value}} and {{abs_value}}.",
        "  scope          string, optional  \"world\" (default), \"battle\", or",
        "                                   \"conversation\" — where the effect",
        "                                   applies.",
        "",
        "trigger_info variants:",
        "  { type = \"once\" }",
        "    Fires a single time when the ability is used.",
        "  { type = \"over_time\", start = <ticks>, end = <ticks, optional>, rate = <ticks> }",
        "    Fires repeatedly starting at `start`, every `rate` ticks, until `end`",
        "    (or indefinitely if `end` is omitted).",
        "",
        "effect_type variants:",
        "  { type = \"attribute_update\", attribute_id = \"<id>\", value = <integer> }",
        "    Adds `value` to the target's `attribute_id` attribute (negative values",
        "    deal damage, positive values heal/buff).",
        "  { type = \"attribute_shield\", attribute_id = \"<id>\", absorb_amount = <integer> }",
        "    Grants a shield absorbing up to `absorb_amount` of damage to",
        "    `attribute_id` before it is reduced directly.",
        "  { type = \"entity_spawn\", entity_id = \"<id>\", location = <table, optional> }",
        "    Spawns an entity with the given `entity_id`, optionally at a specific",
        "    location (world_id, dungeon_id, room_id). Omit `location` to spawn near",
        "    the acting entity.",
    ]
    .join("\n")
}

fn example_section() -> String {
    [
        "Complete annotated example (muds/basic/abilities/defend.toml):",
        "",
        "  name = \"Defend\"",
        "  description = \"Brace for impact, absorbing incoming damage.\"",
        "  engagement_types = [\"battle\"]           # usable in battle only",
        "  costs = []                              # no resource cost",
        "  role = \"defend\"                         # defensive ability",
        "  targets = [\"self\"]                      # targets the caster",
        "",
        "  [[effects]]",
        "  name = \"damage_reduction\"",
        "  trigger_info = { type = \"once\" }        # fires once when used",
        "  effect_type = { type = \"attribute_shield\", attribute_id = \"hp\", absorb_amount = 5 }",
        "  description = { text = \"Defend for {{abs_value}}\" }",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_documents_top_level_fields() {
        let text = render();
        assert!(text.contains("engagement_types"));
        assert!(text.contains("costs"));
        assert!(text.contains("role"));
        assert!(text.contains("targets"));
        assert!(text.contains("[[effects]]"));
    }

    #[test]
    fn render_documents_effect_type_variants() {
        let text = render();
        assert!(text.contains("attribute_update"));
        assert!(text.contains("attribute_shield"));
        assert!(text.contains("entity_spawn"));
    }

    #[test]
    fn render_documents_trigger_info_variants() {
        let text = render();
        assert!(text.contains("\"once\""));
        assert!(text.contains("over_time"));
    }

    #[test]
    fn render_documents_role_and_target_values() {
        let text = render();
        assert!(text.contains("\"attack\""));
        assert!(text.contains("\"defend\""));
        assert!(text.contains("\"world\""));
        assert!(text.contains("\"opponent\""));
        assert!(text.contains("\"allies\""));
        assert!(text.contains("\"self\""));
    }

    #[test]
    fn render_includes_worked_example() {
        let text = render();
        assert!(text.contains("Complete annotated example"));
        assert!(text.contains("damage_reduction"));
    }
}
