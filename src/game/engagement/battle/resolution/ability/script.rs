use std::sync::Arc;

use crate::game::GameState;
use crate::game::component::Ability;
use crate::game::component::effect::Effect;
use crate::game::entity::character::Character;
use crate::game::script;
use crate::game::script::cache::ScriptCacheError;

/// Resolves `ability`'s effective effects for this cast: the static `effects` when it has
/// neither `inline_script` nor `script`, or each effect run through whichever script applies
/// (see [`script_source`]) when it does. Any failure along the way — no caster, a cache/compile
/// error, or a script execution error — is logged and falls back to the static effects, so a
/// single bad script degrades gracefully instead of breaking battle resolution.
pub(super) async fn resolved_effects(
    game_state: &Arc<GameState>,
    caster: Option<&Character>,
    ability: &Ability,
) -> Vec<Effect> {
    let Some(caster) = caster else {
        return effects_for_missing_caster(ability);
    };
    apply_source(game_state, caster, ability).await
}

fn effects_for_missing_caster(ability: &Ability) -> Vec<Effect> {
    if ability.inline_script.is_some() || ability.script.is_some() {
        tracing::warn!(
            ability_id = %ability.id,
            "Scripted ability cast with no caster in play; using static effects"
        );
    }
    ability.effects.clone()
}

async fn apply_source(
    game_state: &Arc<GameState>,
    caster: &Character,
    ability: &Ability,
) -> Vec<Effect> {
    match script_source(game_state, ability).await {
        Some(Ok(source)) => {
            let mut effects = Vec::with_capacity(ability.effects.len());
            for effect in &ability.effects {
                effects.push(run_or_fallback(&source, caster, effect).await);
            }
            effects
        }
        Some(Err(err)) => {
            tracing::warn!(
                ability_id = %ability.id,
                %err,
                "Failed to load ability script; using static effects"
            );
            ability.effects.clone()
        }
        None => ability.effects.clone(),
    }
}

/// Picks the script source to run, preferring `inline_script` over a `script` file reference
/// when both are set. Returns `None` when neither is configured (nothing to resolve); the
/// inner `Result` only ever fails for the `script` (file-cache) path.
async fn script_source(
    game_state: &Arc<GameState>,
    ability: &Ability,
) -> Option<Result<String, ScriptCacheError>> {
    if let Some(inline) = &ability.inline_script {
        return Some(Ok(inline.clone()));
    }
    let script_name = ability.script.as_ref()?;
    Some(script::cache::load_source(&game_state.scripts, script_name).await)
}

async fn run_or_fallback(source: &str, caster: &Character, effect: &Effect) -> Effect {
    match script::execution::run_on_effect(source, caster, effect).await {
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

    fn damage_ability(script_name: Option<&str>, inline_script: Option<&str>) -> Ability {
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
            inline_script: inline_script.map(|s| s.to_string()),
            engagement_types: vec![EngagementType::Battle],
            role: AbilityRole::Attack,
            targets: vec![],
            action_text: None,
        }
    }

    const SCALE_SCRIPT: &str =
        "effect.effect_type.value = -(character.attributes.strength.current_value * 2) | 0;";

    fn register_script(path: std::path::PathBuf) -> std::collections::HashMap<String, Script> {
        let mut scripts = HashMap::new();
        scripts.insert("abilities/damage_bonus".to_string(), Script::new(path));
        scripts
    }

    #[tokio::test]
    async fn unscripted_ability_returns_static_effects_unchanged() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let caster = caster_with_strength(14);
        let ability = damage_ability(None, None);
        let effects = resolved_effects(&game_state, Some(&caster), &ability).await;
        assert_eq!(effects, ability.effects);
    }

    #[tokio::test]
    async fn scripted_ability_scales_effect_from_caster_strength() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("damage_bonus.js");
        std::fs::write(&path, SCALE_SCRIPT).unwrap();

        let game_state = Arc::new(GameState::load(None).unwrap());
        *game_state.scripts.write().await = register_script(path);

        let caster = caster_with_strength(14);
        let ability = damage_ability(Some("abilities/damage_bonus"), None);
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
        let ability = damage_ability(Some("abilities/damage_bonus"), None);
        let effects = resolved_effects(&game_state, None, &ability).await;
        assert_eq!(effects, ability.effects);
    }

    #[tokio::test]
    async fn scripted_ability_falls_back_to_static_effects_when_script_unregistered() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let caster = caster_with_strength(14);
        let ability = damage_ability(Some("abilities/does_not_exist"), None);
        let effects = resolved_effects(&game_state, Some(&caster), &ability).await;
        assert_eq!(effects, ability.effects);
    }

    #[tokio::test]
    async fn inline_script_scales_effect_without_touching_file_cache() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let caster = caster_with_strength(14);
        let ability = damage_ability(None, Some(SCALE_SCRIPT));
        let effects = resolved_effects(&game_state, Some(&caster), &ability).await;

        assert_eq!(effects.len(), 1);
        assert_eq!(
            effects[0].effect_type,
            EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: -28,
            }
        );
        assert!(game_state.scripts.read().await.is_empty());
    }

    #[tokio::test]
    async fn inline_script_takes_precedence_over_script_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("damage_bonus.js");
        std::fs::write(&path, "effect.effect_type.value = -1;").unwrap();

        let game_state = Arc::new(GameState::load(None).unwrap());
        *game_state.scripts.write().await = register_script(path);

        let caster = caster_with_strength(14);
        let ability = damage_ability(Some("abilities/damage_bonus"), Some(SCALE_SCRIPT));
        let effects = resolved_effects(&game_state, Some(&caster), &ability).await;

        // The file script would set -1; the inline script scales with strength instead.
        assert_eq!(
            effects[0].effect_type,
            EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value: -28,
            }
        );
    }

    #[tokio::test]
    async fn inline_script_falls_back_to_static_effects_when_caster_missing() {
        let game_state = Arc::new(GameState::load(None).unwrap());
        let ability = damage_ability(None, Some(SCALE_SCRIPT));
        let effects = resolved_effects(&game_state, None, &ability).await;
        assert_eq!(effects, ability.effects);
    }
}
