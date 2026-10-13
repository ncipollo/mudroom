pub fn render() -> String {
    let sections = [
        header(),
        scripts_folder_section(),
        runtime_section(),
        character_context_section(),
        effect_context_section(),
        fallback_section(),
        example_section(),
    ];
    sections.join("\n\n")
}

fn header() -> String {
    r#"mudroom instructions scripting — JS scripting system reference

Abilities can compute their effect values dynamically with a JS script
instead of static values. This topic covers the scripting system itself —
the scripts/ folder, the JS runtime, and the character/effect contexts a
script can read or mutate. See `mudroom instructions abilities` for how a
script attaches to an ability via the "script"/"inline_script" fields."#
        .to_string()
}

fn scripts_folder_section() -> String {
    r#"Scripts folder:
  Script files live at scripts/**/*.js under a mud's config directory (e.g.
  muds/basic/scripts/abilities/damage_bonus.js). The whole tree is walked
  and registered at startup, keyed by each file's path relative to scripts/
  with the extension stripped (e.g. "abilities/damage_bonus").

  An ability's "script" field names a script by that same key, e.g.
  script = "abilities/damage_bonus" resolves to
  scripts/abilities/damage_bonus.js. The .js extension is assumed if
  omitted; a reference with an explicit extension also works."#
        .to_string()
}

fn runtime_section() -> String {
    r#"Runtime:
  Scripts run as plain JavaScript via an embedded engine (Boa). A script
  referenced by "script" is read from disk and syntax-checked once, on
  first use, then cached — later casts reuse the cached source without
  touching disk again. Despite the cache, each run re-evaluates the script
  from source in a fresh, isolated context: scripts cannot share state
  across effects or across casts.

  A script runs once per [[effects]] entry on the ability, with two
  globals available: "character" (the caster) and "effect" (the effect
  currently being computed). See "character context" and "effect context"
  below for what each exposes."#
        .to_string()
}

fn character_context_section() -> String {
    r#"character context (read-only):
  character.name                           string
  character.location.world_id              string
  character.location.dungeon_id            string
  character.location.room_id               string
  character.attributes.<id>.definition_id  string
  character.attributes.<id>.min_value      integer
  character.attributes.<id>.max_value      integer
  character.attributes.<id>.current_value  integer
  character.active_effects[i]              same shape as "effect" below —
                                            the caster's other currently
                                            active effects."#
        .to_string()
}

fn effect_context_section() -> String {
    r#"effect context (read/write — mutations are applied to the effect used
in battle resolution):
  effect.name           string
  effect.scope          "world" | "battle" | "conversation"
  effect.trigger_info   { type = "once" } or
                        { type = "over_time", start, end, rate }
  effect.description    { start_description, end_description, text }
  effect.effect_type    one of:
    { type = "attribute_update", attribute_id, value }
    { type = "attribute_shield", attribute_id, absorb_amount }
    { type = "attribute_buff", attribute_id, value }
    { type = "entity_spawn", entity_id, location }

  See `mudroom instructions abilities` for what each effect_type and
  trigger_info variant actually does — this topic only covers the property
  names a script sees, not their game-rule meaning."#
        .to_string()
}

fn fallback_section() -> String {
    r#"Fallback behavior:
  An ability's static effect values (from its [[effects]] entries) are
  used as-is whenever there's no script configured, there's no caster
  present to run it against, or the script fails to load, compile, or run.
  Each effect's script runs and fails independently — one bad effect
  script doesn't affect the ability's other effects."#
        .to_string()
}

fn example_section() -> String {
    r#"Worked example:

  An ability's [[effects]] entry sets a static fallback value, e.g.
  effect_type = { type = "attribute_update", attribute_id = "hp", value = -18 }.
  Attaching this script (shipped with the sample mud as
  scripts/abilities/damage_bonus.js) overrides it whenever the script runs:

  effect.effect_type.value = -(character.attributes.strength.current_value * 2) | 0;

  It reads the caster's strength via
  character.attributes.strength.current_value, then writes a scaled
  damage value back to effect.effect_type.value. The | 0 forces integer
  coercion — JS arithmetic otherwise produces a float, which fails to
  deserialize into the effect's integer value field. A caster with 14
  strength produces -28, overriding the static -18 above; if the script
  were missing or failed, -18 would be used instead (see "Fallback
  behavior" above)."#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_documents_scripts_folder() {
        let text = render();
        assert!(text.contains("scripts/"));
        assert!(text.contains(".js"));
        assert!(text.contains("abilities/damage_bonus"));
    }

    #[test]
    fn render_documents_runtime_behavior() {
        let text = render();
        assert!(text.contains("Boa"));
        assert!(text.contains("cached"));
        assert!(text.contains("isolated"));
    }

    #[test]
    fn render_documents_character_context() {
        let text = render();
        assert!(text.contains("character.name"));
        assert!(text.contains("character.attributes"));
        assert!(text.contains("current_value"));
        assert!(text.contains("character.active_effects"));
        assert!(text.contains("character.location"));
    }

    #[test]
    fn render_documents_effect_context() {
        let text = render();
        assert!(text.contains("effect.name"));
        assert!(text.contains("effect.effect_type"));
        assert!(text.contains("attribute_update"));
        assert!(text.contains("attribute_shield"));
        assert!(text.contains("attribute_buff"));
        assert!(text.contains("entity_spawn"));
        assert!(text.contains("trigger_info"));
    }

    #[test]
    fn render_documents_fallback_behavior() {
        let text = render();
        assert!(text.contains("Fallback behavior"));
        assert!(text.contains("no caster"));
    }

    #[test]
    fn render_includes_worked_example() {
        let text = render();
        assert!(text.contains("damage_bonus.js"));
        assert!(text.contains("current_value * 2"));
    }
}
