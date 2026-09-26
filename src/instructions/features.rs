pub fn render() -> String {
    let sections = [
        header(),
        top_level_fields_section(),
        state_fields_section(),
        validation_section(),
        example_section(),
    ];
    sections.join("\n\n")
}

fn header() -> String {
    r#"mudroom instructions features — room feature config file reference

Feature files live at features/*.toml under a mud's config directory (e.g.
muds/basic/features/chest.toml). Each file defines one interactive, stateful
fixture in a room (a chest, a lever, a shrine...). The feature's id is
derived from its file path relative to the features directory (extension
stripped, e.g. features/chest.toml -> "chest"; features/dungeon/chest.toml
-> "dungeon/chest") — it is not set in the TOML itself."#
        .to_string()
}

fn top_level_fields_section() -> String {
    r#"Top-level fields:
  name              string            Display name.
  default_state     string            The state key (from `states`) the
                                       feature is in before anything has
                                       interacted with it.
  alternate_names   array of strings  Extra names players can use to refer
                                       to the feature with `look`, `interact`,
                                       and `take ... from` (e.g. ["chest"]
                                       for an Oak Chest). Matched
                                       case-insensitively alongside the
                                       primary name. Defaults to [] if
                                       omitted.
  [states.<id>]     table             One entry per state the feature can be
                                       in, keyed by state id. See
                                       "states.<id> fields" below."#
        .to_string()
}

fn state_fields_section() -> String {
    r#"states.<id> fields:
  description            string             The state's prose description,
                                             shown to players.
  items                  array of item ids  What the feature holds while in
                                             this state (e.g. what `take ...
                                             from` can retrieve). Defaults to
                                             [] if omitted.
  interact_script        table, optional    Stub for now; not yet used.
  interact_next_state    string, optional   Another state key to transition
                                             to when interacted with.
                                             Omit to make this state a dead
                                             end for interaction.
  alt_verbs              array of strings   Extra verbs beyond `interact`
                                             that trigger this state's
                                             interact_next_state transition
                                             (e.g. ["open"] while closed,
                                             ["close"] while open). `interact`
                                             itself always works, whether or
                                             not it's listed here. Defaults to
                                             [] if omitted."#
        .to_string()
}

fn validation_section() -> String {
    r#"Validation:
  A feature fails to load if `default_state` names a state not present in
  `states`, or if any state's `interact_next_state` names a state not
  present in `states`."#
        .to_string()
}

fn example_section() -> String {
    r#"Complete annotated example (muds/basic/features/chest.toml):

  name = "Oak Chest"
  default_state = "open"
  alternate_names = ["chest"]           # `take medicine from chest` also works

  [states.closed]
  description = "A heavy oak chest bound with iron bands. The lid is shut."
  interact_next_state = "open"          # `interact` (or `open`) opens it
  alt_verbs = ["open"]

  [states.open]
  description = "The oak chest stands open, its lid tipped back."
  items = ["medicine"]                  # what `take medicine from chest` finds
  interact_next_state = "closed"        # `interact` (or `close`) closes it
  alt_verbs = ["close"]"#
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_documents_top_level_fields() {
        let text = render();
        assert!(text.contains("default_state"));
        assert!(text.contains("alternate_names"));
        assert!(text.contains("[states.<id>]"));
    }

    #[test]
    fn render_documents_state_fields() {
        let text = render();
        assert!(text.contains("description"));
        assert!(text.contains("items"));
        assert!(text.contains("interact_script"));
        assert!(text.contains("interact_next_state"));
        assert!(text.contains("alt_verbs"));
    }

    #[test]
    fn render_documents_id_derivation() {
        let text = render();
        assert!(text.contains("features/chest.toml"));
        assert!(text.contains("dungeon/chest"));
    }

    #[test]
    fn render_documents_validation_rules() {
        let text = render();
        assert!(text.contains("Validation:"));
        assert!(text.contains("default_state"));
    }

    #[test]
    fn render_includes_worked_example() {
        let text = render();
        assert!(text.contains("Complete annotated example"));
        assert!(text.contains("chest.toml"));
        assert!(text.contains("Oak Chest"));
    }
}
