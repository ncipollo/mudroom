use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionRelations {
    #[serde(default)]
    pub player: FactionRelation,
    #[serde(default)]
    pub monster: FactionRelation,
    #[serde(default)]
    pub factions: HashMap<String, FactionRelation>,
}

impl Default for FactionRelations {
    fn default() -> Self {
        Self {
            player: FactionRelation::NonInteractive,
            monster: FactionRelation::NonInteractive,
            factions: HashMap::new(),
        }
    }
}

impl FactionRelations {
    pub fn default_for_monster() -> Self {
        Self {
            player: FactionRelation::Hostile,
            monster: FactionRelation::NonInteractive,
            factions: HashMap::new(),
        }
    }

    pub fn default_for_player() -> Self {
        Self {
            player: FactionRelation::Friendly,
            monster: FactionRelation::Hostile,
            factions: HashMap::new(),
        }
    }

    pub fn relation_for(&self, faction_id: &str) -> &FactionRelation {
        if let Some(rel) = self.factions.get(faction_id) {
            return rel;
        }
        match faction_id {
            "player" => &self.player,
            "monster" => &self.monster,
            _ => &NON_INTERACTIVE,
        }
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
    fn faction_relations_default_is_all_non_interactive() {
        let relations = FactionRelations::default();
        assert_eq!(relations.player, FactionRelation::NonInteractive);
        assert_eq!(relations.monster, FactionRelation::NonInteractive);
        assert!(relations.factions.is_empty());
    }

    #[test]
    fn default_for_monster_has_player_hostile() {
        let relations = FactionRelations::default_for_monster();
        assert_eq!(relations.player, FactionRelation::Hostile);
        assert_eq!(relations.monster, FactionRelation::NonInteractive);
    }

    #[test]
    fn default_for_player_has_monster_hostile() {
        let relations = FactionRelations::default_for_player();
        assert_eq!(relations.player, FactionRelation::Friendly);
        assert_eq!(relations.monster, FactionRelation::Hostile);
    }

    #[test]
    fn relation_for_checks_factions_map_first() {
        let mut relations = FactionRelations::default_for_player();
        relations
            .factions
            .insert("player".to_string(), FactionRelation::Unfriendly);
        assert_eq!(
            relations.relation_for("player"),
            &FactionRelation::Unfriendly
        );
    }

    #[test]
    fn relation_for_falls_back_to_player_shortcut() {
        let relations = FactionRelations::default_for_monster();
        assert_eq!(relations.relation_for("player"), &FactionRelation::Hostile);
    }

    #[test]
    fn relation_for_falls_back_to_monster_shortcut() {
        let relations = FactionRelations::default_for_player();
        assert_eq!(relations.relation_for("monster"), &FactionRelation::Hostile);
    }

    #[test]
    fn relation_for_unknown_faction_returns_non_interactive() {
        let relations = FactionRelations::default_for_monster();
        assert_eq!(
            relations.relation_for("bandits"),
            &FactionRelation::NonInteractive
        );
    }

    #[test]
    fn relation_for_specific_faction_override() {
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
    fn faction_relations_toml_round_trip() {
        let toml = r#"
player = "hostile"
monster = "friendly"

[factions]
bandits = "unfriendly"
"#;
        let relations: FactionRelations = toml::from_str(toml).unwrap();
        assert_eq!(relations.player, FactionRelation::Hostile);
        assert_eq!(relations.monster, FactionRelation::Friendly);
        assert_eq!(relations.factions["bandits"], FactionRelation::Unfriendly);
    }
}
