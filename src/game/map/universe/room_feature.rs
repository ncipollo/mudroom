use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::game::Description;

/// An interactive, stateful fixture in a room (a chest, a button, a shrine...).
/// Every feature declares a `default_state`, even when it only has one state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomFeature {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub default_state: String,
    pub states: HashMap<String, FeatureState>,
    /// Verbs that trigger this feature's `interact_next_state` transition just like
    /// `interact` (e.g. `open` for a chest). `interact` itself always works, whether or
    /// not it's listed here.
    #[serde(default)]
    pub alt_verbs: Vec<String>,
    /// Extra names the feature can be referred to by (e.g. `["chest"]` for an Oak Chest).
    /// Matched case-insensitively by `look` and `interact` alongside the primary name.
    #[serde(default)]
    pub alternate_names: Vec<String>,
}

/// One state a [`RoomFeature`] can be in: what it looks like, what it holds, and
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
}

/// Stub for now; fleshed out in a later ticket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractScript {
    pub id: String,
}

/// A [`RoomFeature`] whose `default_state` or a state's `interact_next_state` names
/// a state that isn't defined in `states`.
#[derive(Debug, Error, PartialEq)]
pub enum FeatureValidationError {
    #[error("feature `{feature}` default_state `{state}` is not a defined state")]
    UnknownDefaultState { feature: String, state: String },
    #[error("feature `{feature}` state `{from}` transitions to undefined state `{next}`")]
    UnknownNextState {
        feature: String,
        from: String,
        next: String,
    },
}

impl RoomFeature {
    /// The state this feature is in when nothing has interacted with it yet.
    pub fn default_state(&self) -> Option<&FeatureState> {
        self.states.get(&self.default_state)
    }

    /// Checks that `default_state` and every `interact_next_state` name a state that
    /// actually exists in `states`.
    pub fn validate(&self) -> Result<(), FeatureValidationError> {
        if !self.states.contains_key(&self.default_state) {
            return Err(FeatureValidationError::UnknownDefaultState {
                feature: self.id.clone(),
                state: self.default_state.clone(),
            });
        }
        self.validate_next_states()
    }

    /// Whether `verb` can trigger this feature's `interact_next_state` transition.
    /// `interact` always works; anything else must be one of this feature's declared
    /// alternate verbs.
    pub fn allows_verb(&self, verb: &str) -> bool {
        verb.eq_ignore_ascii_case("interact")
            || self.alt_verbs.iter().any(|v| v.eq_ignore_ascii_case(verb))
    }

    pub fn matches_name(&self, target: &str) -> bool {
        self.name.eq_ignore_ascii_case(target)
    }

    pub fn matches_alternate_name(&self, target: &str) -> bool {
        self.alternate_names
            .iter()
            .any(|alt| alt.eq_ignore_ascii_case(target))
    }

    fn validate_next_states(&self) -> Result<(), FeatureValidationError> {
        for (from, state) in &self.states {
            let Some(next) = &state.interact_next_state else {
                continue;
            };
            if !self.states.contains_key(next) {
                return Err(FeatureValidationError::UnknownNextState {
                    feature: self.id.clone(),
                    from: from.clone(),
                    next: next.clone(),
                });
            }
        }
        Ok(())
    }
}

/// Filters `candidates` down to those whose resolved [`RoomFeature`] matches `target` by
/// name. Primary-name matches (case-insensitive) win: alternate-name matches are only used
/// when nothing matched by primary name, so an alias collision never makes a real name
/// ambiguous. `feature` resolves a candidate to its [`RoomFeature`].
pub fn select_by_name<T>(
    candidates: Vec<T>,
    target: &str,
    feature: impl Fn(&T) -> &RoomFeature,
) -> Vec<T> {
    let mut name_matches = Vec::new();
    let mut alternate_matches = Vec::new();
    for candidate in candidates {
        if feature(&candidate).matches_name(target) {
            name_matches.push(candidate);
        } else if feature(&candidate).matches_alternate_name(target) {
            alternate_matches.push(candidate);
        }
    }
    if name_matches.is_empty() {
        alternate_matches
    } else {
        name_matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feature_state(text: &str) -> FeatureState {
        FeatureState {
            description: Description::new(Some(text.to_string())),
            items: Vec::new(),
            interact_script: None,
            interact_next_state: None,
        }
    }

    fn chest() -> RoomFeature {
        let mut states = HashMap::new();
        states.insert(
            "closed".to_string(),
            FeatureState {
                interact_next_state: Some("open".to_string()),
                ..feature_state("A closed chest.")
            },
        );
        states.insert(
            "open".to_string(),
            FeatureState {
                items: vec!["medicine".to_string()],
                ..feature_state("An open chest.")
            },
        );
        RoomFeature {
            id: "chest".to_string(),
            name: "Oak Chest".to_string(),
            default_state: "closed".to_string(),
            states,
            alt_verbs: vec!["open".to_string()],
            alternate_names: vec!["chest".to_string()],
        }
    }

    #[test]
    fn serde_round_trip() {
        let feature = chest();
        let json = serde_json::to_string(&feature).unwrap();
        let restored: RoomFeature = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, feature);
    }

    #[test]
    fn parses_bare_string_description_from_toml() {
        let toml = r#"
name = "Button"
default_state = "idle"

[states.idle]
description = "A dusty button."
"#;
        let feature: RoomFeature = toml::from_str(toml).unwrap();
        let state = feature.states.get("idle").unwrap();
        assert_eq!(state.description.text.as_deref(), Some("A dusty button."));
    }

    #[test]
    fn state_defaults_items_script_and_next_state_when_omitted() {
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
    }

    #[test]
    fn alt_verbs_defaults_to_empty_when_omitted() {
        let toml = r#"
name = "Button"
default_state = "idle"

[states.idle]
description = "A dusty button."
"#;
        let feature: RoomFeature = toml::from_str(toml).unwrap();
        assert!(feature.alt_verbs.is_empty());
    }

    #[test]
    fn alternate_names_defaults_to_empty_when_omitted() {
        let toml = r#"
name = "Button"
default_state = "idle"

[states.idle]
description = "A dusty button."
"#;
        let feature: RoomFeature = toml::from_str(toml).unwrap();
        assert!(feature.alternate_names.is_empty());
    }

    #[test]
    fn matches_name_is_case_insensitive() {
        let feature = chest();
        assert!(feature.matches_name("oak chest"));
        assert!(feature.matches_name("OAK CHEST"));
        assert!(!feature.matches_name("chest"));
    }

    #[test]
    fn matches_alternate_name_is_case_insensitive() {
        let feature = chest();
        assert!(feature.matches_alternate_name("chest"));
        assert!(feature.matches_alternate_name("CHEST"));
        assert!(!feature.matches_alternate_name("oak chest"));
    }

    #[test]
    fn allows_verb_always_allows_interact() {
        let mut feature = chest();
        feature.alt_verbs = vec![];
        assert!(feature.allows_verb("interact"));
        assert!(feature.allows_verb("INTERACT"));
    }

    #[test]
    fn allows_verb_matches_declared_alt_verb_case_insensitively() {
        let feature = chest();
        assert!(feature.allows_verb("open"));
        assert!(feature.allows_verb("OPEN"));
    }

    #[test]
    fn allows_verb_rejects_undeclared_verb() {
        let feature = chest();
        assert!(!feature.allows_verb("push"));
    }

    #[test]
    fn default_state_resolves_the_named_state() {
        let feature = chest();
        let state = feature.default_state().unwrap();
        assert_eq!(state.description.text.as_deref(), Some("A closed chest."));
    }

    #[test]
    fn default_state_is_none_when_default_state_is_unknown() {
        let mut feature = chest();
        feature.default_state = "missing".to_string();
        assert!(feature.default_state().is_none());
    }

    #[test]
    fn validate_accepts_a_well_formed_feature() {
        assert_eq!(chest().validate(), Ok(()));
    }

    #[test]
    fn validate_rejects_unknown_default_state() {
        let mut feature = chest();
        feature.default_state = "missing".to_string();
        assert_eq!(
            feature.validate(),
            Err(FeatureValidationError::UnknownDefaultState {
                feature: "chest".to_string(),
                state: "missing".to_string(),
            })
        );
    }

    #[test]
    fn validate_rejects_dangling_interact_next_state() {
        let mut feature = chest();
        feature
            .states
            .get_mut("closed")
            .unwrap()
            .interact_next_state = Some("missing".to_string());
        assert_eq!(
            feature.validate(),
            Err(FeatureValidationError::UnknownNextState {
                feature: "chest".to_string(),
                from: "closed".to_string(),
                next: "missing".to_string(),
            })
        );
    }

    fn named_feature(name: &str, alternate_names: &[&str]) -> RoomFeature {
        let mut feature = chest();
        feature.name = name.to_string();
        feature.alternate_names = alternate_names.iter().map(|s| s.to_string()).collect();
        feature
    }

    #[test]
    fn select_by_name_prefers_primary_name_over_alias() {
        let candidates = vec![
            named_feature("Club", &[]),
            named_feature("Spiked Bat", &["club"]),
        ];
        let selected = select_by_name(candidates, "club", |f| f);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].name, "Club");
    }

    #[test]
    fn select_by_name_returns_all_alias_matches_when_no_primary_name_matches() {
        let candidates = vec![
            named_feature("Oak Chest", &["stick"]),
            named_feature("Iron Chest", &["stick"]),
        ];
        let selected = select_by_name(candidates, "stick", |f| f);
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn select_by_name_matches_nothing_when_target_is_unrelated() {
        let candidates = vec![named_feature("Oak Chest", &["chest"])];
        let selected = select_by_name(candidates, "torch", |f| f);
        assert!(selected.is_empty());
    }
}
