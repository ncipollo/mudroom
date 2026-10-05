use std::sync::Arc;

use crate::game::GameState;
use crate::game::component::Ability;
use crate::game::component::effect::Effect;
use crate::game::entity::character::Character;
use crate::game::script;

/// Resolves `ability`'s effective effects for this cast: the static `effects` when it has no
/// script, or each effect run through its script (see `script::execution::run_on_effect`) when
/// it does. Any failure along the way — no caster, a cache/compile error, or a script
/// execution error — is logged and falls back to the static effects, so a single bad script
/// degrades gracefully instead of breaking battle resolution.
pub(super) async fn resolved_effects(
    game_state: &Arc<GameState>,
    caster: Option<&Character>,
    ability: &Ability,
) -> Vec<Effect> {
    let Some(script_name) = &ability.script else {
        return ability.effects.clone();
    };
    match caster {
        Some(caster) => scripted_effects(game_state, script_name, caster, ability).await,
        None => {
            tracing::warn!(
                ability_id = %ability.id,
                "Scripted ability cast with no caster in play; using static effects"
            );
            ability.effects.clone()
        }
    }
}

async fn scripted_effects(
    game_state: &Arc<GameState>,
    script_name: &str,
    caster: &Character,
    ability: &Ability,
) -> Vec<Effect> {
    match script::cache::load_source(&game_state.scripts, script_name).await {
        Ok(source) => ability
            .effects
            .iter()
            .map(|effect| run_or_fallback(&source, caster, effect))
            .collect(),
        Err(err) => {
            tracing::warn!(
                ability_id = %ability.id,
                %err,
                "Failed to load ability script; using static effects"
            );
            ability.effects.clone()
        }
    }
}

fn run_or_fallback(source: &str, caster: &Character, effect: &Effect) -> Effect {
    match script::execution::run_on_effect(source, caster, effect) {
        Ok(mutated) => mutated,
        Err(err) => {
            tracing::warn!(%err, "Ability script execution failed; using static effect");
            effect.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use tempfile::TempDir;

    use super::*;
    use crate::game::component::effect::{EffectDescription, EffectScope, EffectType, TriggerInfo};
    use crate::game::component::{AbilityRole, Attribute, Description, Location};
    use crate::game::engagement::EngagementType;
    use crate::game::entity::character::CharacterType;
    use crate::game::script::Script;

    fn test_location() -> Location {
        Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        }
    }

    fn caster_with_strength(strength: i64) -> Character {
        let mut character = Character::new(1, CharacterType::Player, test_location());
        character.attributes.insert(
            "strength".to_string(),
            Attribute::new("strength".to_string(), 1, 20, strength),
        );
        character
    }

    fn damage_ability(script_name: Option<&str>) -> Ability {
        Ability {
            id: "painful_smash".to_string(),
            name: "Painful Smash".to_string(),
            description: Description::default(),
            effects: vec![Effect {
                name: "smash_damage".to_string(),
                effect_type: EffectType::AttributeUpdate {
                    attribute_id: "hp".to_string(),
                    value: -18,
                },
                trigger_info: TriggerInfo::Once,
                description: EffectDescription::default(),
                scope: EffectScope::default(),
            }],
            costs: vec![],
            script: script_name.map(|s| s.to_string()),
            engagement_types: vec![EngagementType::Battle],
            role: AbilityRole::Attack,
            targets: vec![],
            action_text: None,
        }
    }

    #[tokio::test]
    async fn unscripted_ability_returns_static_effects_unchanged() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let caster = caster_with_strength(14);
        let ability = damage_ability(None);
        let effects = resolved_effects(&game_state, Some(&caster), &ability).await;
        assert_eq!(effects, ability.effects);
    }

    #[tokio::test]
    async fn scripted_ability_scales_effect_from_caster_strength() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("damage_bonus.js");
        std::fs::write(
            &path,
            "effect.effect_type.value = -(character.attributes.strength.current_value * 2) | 0;",
        )
        .unwrap();

        let game_state = Arc::new(GameState::load(None).unwrap());
        let mut scripts = HashMap::new();
        scripts.insert("abilities/damage_bonus".to_string(), Script::new(path));
        *game_state.scripts.write().await = scripts;

        let caster = caster_with_strength(14);
        let ability = damage_ability(Some("abilities/damage_bonus"));
        let effects = resolved_effects(&game_state, Some(&caster), &ability).await;

        assert_eq!(effects.len(), 1);
        assert_eq!(
            effects[0].effect_type,
            EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: -28,
            }
        );
    }

    #[tokio::test]
    async fn scripted_ability_falls_back_to_static_effects_when_caster_missing() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let ability = damage_ability(Some("abilities/damage_bonus"));
        let effects = resolved_effects(&game_state, None, &ability).await;
        assert_eq!(effects, ability.effects);
    }

    #[tokio::test]
    async fn scripted_ability_falls_back_to_static_effects_when_script_unregistered() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let caster = caster_with_strength(14);
        let ability = damage_ability(Some("abilities/does_not_exist"));
        let effects = resolved_effects(&game_state, Some(&caster), &ability).await;
        assert_eq!(effects, ability.effects);
    }
}
