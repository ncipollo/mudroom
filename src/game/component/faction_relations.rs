use std::collections::HashMap;

use serde::{Deserialize, Serialize};

const PLAYER_FACTION_ID: &str = "player";
const ENEMY_FACTION_ID: &str = "enemy";

static NON_INTERACTIVE: FactionRelation = FactionRelation::NonInteractive;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactionRelation {
    Hostile,
    Unfriendly,
    Friendly,
    #[default]
    NonInteractive,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FactionRelations {
    #[serde(default)]
    pub factions: HashMap<String, FactionRelation>,
}

impl FactionRelations {
    pub fn default_for_enemy() -> Self {
        let mut factions = HashMap::new();
        factions.insert(PLAYER_FACTION_ID.to_string(), FactionRelation::Hostile);
        Self { factions }
    }

    pub fn default_for_player() -> Self {
        let mut factions = HashMap::new();
        factions.insert(PLAYER_FACTION_ID.to_string(), FactionRelation::Friendly);
        factions.insert(ENEMY_FACTION_ID.to_string(), FactionRelation::Hostile);
        Self { factions }
    }

    pub fn player_relation(&self) -> &FactionRelation {
        self.factions
            .get(PLAYER_FACTION_ID)
            .unwrap_or(&NON_INTERACTIVE)
    }

    pub fn enemy_relation(&self) -> &FactionRelation {
        self.factions
            .get(ENEMY_FACTION_ID)
            .unwrap_or(&NON_INTERACTIVE)
    }

    pub fn relation_for(&self, faction_id: &str) -> &FactionRelation {
        self.factions.get(faction_id).unwrap_or(&NON_INTERACTIVE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faction_relation_default_is_non_interactive() {
        assert_eq!(FactionRelation::default(), FactionRelation::NonInteractive);
    }

    #[test]
    fn faction_relation_serde_round_trip() {
        let variants = [
            FactionRelation::Hostile,
            FactionRelation::Unfriendly,
            FactionRelation::Friendly,
            FactionRelation::NonInteractive,
        ];
        for variant in &variants {
            let json = serde_json::to_string(variant).unwrap();
            let restored: FactionRelation = serde_json::from_str(&json).unwrap();
            assert_eq!(&restored, variant);
        }
    }

    #[test]
    fn faction_relation_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&FactionRelation::NonInteractive).unwrap(),
            "\"non_interactive\""
        );
        assert_eq!(
            serde_json::to_string(&FactionRelation::Hostile).unwrap(),
            "\"hostile\""
        );
    }

    #[test]
    fn faction_relations_default_is_empty() {
        let relations = FactionRelations::default();
        assert!(relations.factions.is_empty());
    }

    #[test]
    fn default_for_enemy_has_player_hostile() {
        let relations = FactionRelations::default_for_enemy();
        assert_eq!(relations.player_relation(), &FactionRelation::Hostile);
        assert_eq!(relations.enemy_relation(), &FactionRelation::NonInteractive);
    }

    #[test]
    fn default_for_player_has_player_friendly_enemy_hostile() {
        let relations = FactionRelations::default_for_player();
        assert_eq!(relations.player_relation(), &FactionRelation::Friendly);
        assert_eq!(relations.enemy_relation(), &FactionRelation::Hostile);
    }

    #[test]
    fn player_relation_returns_non_interactive_when_absent() {
        let relations = FactionRelations::default();
        assert_eq!(
            relations.player_relation(),
            &FactionRelation::NonInteractive
        );
    }

    #[test]
    fn enemy_relation_returns_non_interactive_when_absent() {
        let relations = FactionRelations::default();
        assert_eq!(relations.enemy_relation(), &FactionRelation::NonInteractive);
    }

    #[test]
    fn relation_for_returns_mapped_value() {
        let mut relations = FactionRelations::default();
        relations
            .factions
            .insert("bandits".to_string(), FactionRelation::Unfriendly);
        assert_eq!(
            relations.relation_for("bandits"),
            &FactionRelation::Unfriendly
        );
    }

    #[test]
    fn relation_for_returns_non_interactive_for_unknown_faction() {
        let relations = FactionRelations::default_for_enemy();
        assert_eq!(
            relations.relation_for("bandits"),
            &FactionRelation::NonInteractive
        );
    }

    #[test]
    fn faction_relations_toml_round_trip() {
        let toml = r#"
[factions]
player = "hostile"
enemy = "friendly"
bandits = "unfriendly"
"#;
        let relations: FactionRelations = toml::from_str(toml).unwrap();
        assert_eq!(relations.player_relation(), &FactionRelation::Hostile);
        assert_eq!(relations.enemy_relation(), &FactionRelation::Friendly);
        assert_eq!(relations.factions["bandits"], FactionRelation::Unfriendly);
    }
}
