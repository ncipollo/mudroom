use serde::{Deserialize, Serialize};

use crate::game::Description;

/// One state a [`super::RoomFeature`] can be in: what it looks like, what it holds, and
/// where interacting with it while in this state leads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureState {
    pub description: Description,
    #[serde(default)]
    pub items: Vec<String>,
    #[serde(default)]
    pub interact_script: Option<InteractScript>,
    #[serde(default)]
    pub interact_next_state: Option<String>,
    /// Verbs that trigger this state's `interact_next_state` transition just like
    /// `interact` (e.g. `open` while closed, `close` while open). `interact` itself
    /// always works, whether or not it's listed here.
    #[serde(default)]
    pub alt_verbs: Vec<String>,
    /// Optional template appended verbatim after this state's `description.text` when the
    /// state currently holds items, e.g. `", inside there is {items}."`. `{items}` is
    /// replaced by an indefinite-article, Oxford-comma list of the held items' names. When
    /// `None`, or when none of the held item ids resolve to a known item definition, `look`
    /// falls back to a separate themed message per held item.
    #[serde(default)]
    pub item_summary: Option<String>,
}

impl FeatureState {
    /// Whether `verb` can trigger this state's `interact_next_state` transition.
    /// `interact` always works; anything else must be one of this state's declared
    /// alternate verbs.
    pub fn allows_verb(&self, verb: &str) -> bool {
        verb.eq_ignore_ascii_case("interact")
            || self.alt_verbs.iter().any(|v| v.eq_ignore_ascii_case(verb))
    }
}

/// Stub for now; fleshed out in a later ticket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractScript {
    pub id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::RoomFeature;

    fn feature_state(text: &str) -> FeatureState {
        FeatureState {
            description: Description::new(Some(text.to_string())),
            items: Vec::new(),
            interact_script: None,
            interact_next_state: None,
            alt_verbs: Vec::new(),
            item_summary: None,
        }
    }

    #[test]
    fn allows_verb_always_allows_interact() {
        let state = feature_state("A closed chest.");
        assert!(state.allows_verb("interact"));
        assert!(state.allows_verb("INTERACT"));
    }

    #[test]
    fn allows_verb_matches_declared_alt_verb_case_insensitively() {
        let state = FeatureState {
            alt_verbs: vec!["open".to_string()],
            ..feature_state("A closed chest.")
        };
        assert!(state.allows_verb("open"));
        assert!(state.allows_verb("OPEN"));
    }

    #[test]
    fn allows_verb_rejects_undeclared_verb() {
        let state = FeatureState {
            alt_verbs: vec!["open".to_string()],
            ..feature_state("A closed chest.")
        };
        assert!(!state.allows_verb("push"));
    }

    #[test]
    fn allows_verb_rejects_verb_declared_on_a_different_state() {
        // "open" is only declared as an alt_verb on a *different* state's config.
        let state = feature_state("An open chest.");
        assert!(!state.allows_verb("open"));
    }

    #[test]
    fn state_defaults_items_script_next_state_and_alt_verbs_when_omitted() {
        let toml = r#"
name = "Button"
default_state = "idle"

[states.idle]
description = "A dusty button."
"#;
        let feature: RoomFeature = toml::from_str(toml).unwrap();
        let state = feature.states.get("idle").unwrap();
        assert!(state.items.is_empty());
        assert!(state.interact_script.is_none());
        assert!(state.interact_next_state.is_none());
        assert!(state.alt_verbs.is_empty());
        assert!(state.item_summary.is_none());
    }

    #[test]
    fn parses_item_summary_template_from_toml() {
        let toml = r#"
name = "Oak Chest"
default_state = "open"

[states.open]
description = "The oak chest stands open"
item_summary = ", inside there is {items}."
items = ["medicine"]
"#;
        let feature: RoomFeature = toml::from_str(toml).unwrap();
        let state = feature.states.get("open").unwrap();
        assert_eq!(
            state.item_summary.as_deref(),
            Some(", inside there is {items}.")
        );
    }
}
