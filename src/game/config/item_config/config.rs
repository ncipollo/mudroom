use serde::{Deserialize, Serialize};

use crate::game::component::description::Description;
use crate::game::component::effect::Effect;
use crate::game::component::modifier::Modifier;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemUseType {
    Used,
    Passive,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum UseEffect {
    StatBoost(Modifier),
    ApplyEffect(Effect),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributeBonus {
    pub attribute_id: String,
    pub amount: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EquippedBonuses {
    #[serde(default)]
    pub attributes: Vec<AttributeBonus>,
    #[serde(default)]
    pub equipped: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemDefinition {
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub description: Description,
    pub use_type: ItemUseType,
    pub item_type: String,
    #[serde(default)]
    pub equipped_bonuses: EquippedBonuses,
    #[serde(default)]
    pub use_effects: Vec<UseEffect>,
    /// Extra names the item can be referred to by (e.g. `["bat"]` for a Spiked Bat).
    /// Matched case-insensitively by `take` and `look at` alongside the primary name.
    #[serde(default)]
    pub alternate_names: Vec<String>,
}

impl ItemDefinition {
    /// Whether `target` matches this item's primary display name, case-insensitively.
    pub fn matches_name(&self, target: &str) -> bool {
        self.name.eq_ignore_ascii_case(target)
    }

    /// Whether `target` matches one of this item's alternate names, case-insensitively.
    pub fn matches_alternate_name(&self, target: &str) -> bool {
        self.alternate_names
            .iter()
            .any(|alt| alt.eq_ignore_ascii_case(target))
    }
}

/// Filters `candidates` down to those whose resolved [`ItemDefinition`] matches `target` by
/// name. Primary-name matches (case-insensitive) win: alternate-name matches are only used
/// when nothing matched by primary name, so an alias collision never makes a real name
/// ambiguous. `definition` resolves a candidate to its definition (returning `None` drops it).
pub fn select_by_name<'d, T>(
    candidates: impl IntoIterator<Item = T>,
    target: &str,
    definition: impl Fn(&T) -> Option<&'d ItemDefinition>,
) -> Vec<T> {
    let mut name_matches = Vec::new();
    let mut alternate_matches = Vec::new();
    for candidate in candidates {
        let Some(def) = definition(&candidate) else {
            continue;
        };
        if def.matches_name(target) {
            name_matches.push(candidate);
        } else if def.matches_alternate_name(target) {
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
    use crate::game::component::effect::{EffectDescription, EffectScope, EffectType, TriggerInfo};
    use crate::game::component::modifier::Operator;

    #[test]
    fn item_use_type_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&ItemUseType::Used).unwrap(),
            r#""used""#
        );
        assert_eq!(
            serde_json::to_string(&ItemUseType::Passive).unwrap(),
            r#""passive""#
        );
    }

    #[test]
    fn use_effect_stat_boost_serde_round_trip() {
        let use_effect = UseEffect::StatBoost(Modifier {
            attribute_id: "strength".to_string(),
            operator: Operator::Add,
            amount: 3,
        });
        let json = serde_json::to_string(&use_effect).unwrap();
        let restored: UseEffect = serde_json::from_str(&json).unwrap();
        assert_eq!(use_effect, restored);
    }

    #[test]
    fn use_effect_apply_effect_serde_round_trip() {
        let use_effect = UseEffect::ApplyEffect(Effect {
            name: "heal_hp".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: 20,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription::default(),
            scope: EffectScope::default(),
        });
        let json = serde_json::to_string(&use_effect).unwrap();
        let restored: UseEffect = serde_json::from_str(&json).unwrap();
        assert_eq!(use_effect, restored);
    }

    #[test]
    fn attribute_bonus_serde_round_trip() {
        let bonus = AttributeBonus {
            attribute_id: "max_health".to_string(),
            amount: 10,
        };
        let json = serde_json::to_string(&bonus).unwrap();
        let restored: AttributeBonus = serde_json::from_str(&json).unwrap();
        assert_eq!(bonus, restored);
    }

    #[test]
    fn item_definition_serde_round_trip() {
        let def = ItemDefinition {
            id: "leather_vest".to_string(),
            name: "Leather Vest".to_string(),
            description: Description::new(Some("A simple protective vest.".to_string())),
            use_type: ItemUseType::Passive,
            item_type: "armor".to_string(),
            equipped_bonuses: EquippedBonuses {
                attributes: vec![AttributeBonus {
                    attribute_id: "defense".to_string(),
                    amount: 5,
                }],
                equipped: vec![],
            },
            use_effects: vec![],
            alternate_names: vec!["vest".to_string()],
        };
        let json = serde_json::to_string(&def).unwrap();
        let restored: ItemDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, def);
    }

    #[test]
    fn item_definition_missing_optional_fields_deserializes() {
        let json = r#"{
            "name": "Health Tonic",
            "description": null,
            "use_type": "used",
            "item_type": "medicine"
        }"#;
        let def: ItemDefinition = serde_json::from_str(json).unwrap();
        assert!(def.id.is_empty());
        assert!(def.equipped_bonuses.attributes.is_empty());
        assert!(def.equipped_bonuses.equipped.is_empty());
        assert!(def.use_effects.is_empty());
        assert!(def.alternate_names.is_empty());
    }

    #[test]
    fn item_definition_parses_alternate_names_from_toml() {
        let toml = r#"
name = "Spiked Bat"
use_type = "passive"
item_type = "weapon"
alternate_names = ["bat", "club"]
"#;
        let def: ItemDefinition = toml::from_str(toml).unwrap();
        assert_eq!(def.alternate_names, vec!["bat", "club"]);
    }

    #[test]
    fn matches_name_is_case_insensitive() {
        let def = ItemDefinition {
            id: "spiked_bat".to_string(),
            name: "Spiked Bat".to_string(),
            description: Description::default(),
            use_type: ItemUseType::Passive,
            item_type: "weapon".to_string(),
            equipped_bonuses: EquippedBonuses::default(),
            use_effects: vec![],
            alternate_names: vec!["bat".to_string()],
        };
        assert!(def.matches_name("spiked bat"));
        assert!(def.matches_name("SPIKED BAT"));
        assert!(!def.matches_name("bat"));
        assert!(def.matches_alternate_name("BAT"));
        assert!(!def.matches_alternate_name("spiked bat"));
    }

    fn named(name: &str, alternate_names: &[&str]) -> ItemDefinition {
        ItemDefinition {
            id: name.to_lowercase().replace(' ', "_"),
            name: name.to_string(),
            description: Description::default(),
            use_type: ItemUseType::Passive,
            item_type: "weapon".to_string(),
            equipped_bonuses: EquippedBonuses::default(),
            use_effects: vec![],
            alternate_names: alternate_names.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn select_by_name_prefers_primary_name_over_alias() {
        let defs = [named("Club", &[]), named("Spiked Bat", &["club"])];
        let selected = select_by_name(defs.iter(), "club", |d| Some(*d));
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].name, "Club");
    }

    #[test]
    fn select_by_name_returns_all_alias_matches_when_no_primary_name_matches() {
        let defs = [
            named("Spiked Bat", &["stick"]),
            named("Gnarled Club", &["stick"]),
        ];
        let selected = select_by_name(defs.iter(), "STICK", |d| Some(*d));
        assert_eq!(selected.len(), 2);
    }

    #[test]
    fn select_by_name_drops_candidates_without_a_definition() {
        let defs = [named("Spiked Bat", &["bat"])];
        let selected = select_by_name(0..3, "bat", |i| defs.get(*i));
        assert_eq!(selected.len(), 1);
    }
}
