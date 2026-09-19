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
}
