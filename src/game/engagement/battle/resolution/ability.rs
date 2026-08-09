use std::collections::HashMap;
use std::sync::Arc;

use crate::game::GameState;
use crate::game::character::Character;
use crate::game::config::AttributeConfig;
use crate::game::engagement::TurnOrder;
use crate::game::engagement::battle::{BattleMessage, QueuedAbility};
use crate::game::narration::{TextResolver, VariableMap, effect_text};

use super::effect::resolve_effects;

/// Resolves all queued ability casts from a `ResolveAbilities` phase completion: orders casters
/// by speed, applies each ability's effects to its target, and emits cast messages. Only call
/// this when the current battle tick has just completed the `ResolveAbilities` phase.
pub(in crate::game::engagement::battle) async fn apply_battle_effects(
    game_state: &Arc<GameState>,
    resolution_queue: Vec<QueuedAbility>,
    entity_names: &HashMap<i64, String>,
    messages: &mut Vec<BattleMessage>,
) {
    let mut entities = game_state.active_characters.write().await;
    let speed_sorted_casters = speed_sort_casters(
        &resolution_queue
            .iter()
            .map(|qa| qa.caster_id)
            .collect::<Vec<_>>(),
        &entities,
        &game_state.attribute_config,
    );

    let mut target_effects = HashMap::new();
    let mut by_caster: HashMap<i64, Vec<QueuedAbility>> = HashMap::new();
    for qa in resolution_queue {
        by_caster.entry(qa.caster_id).or_default().push(qa);
    }

    for &caster_id in &speed_sorted_casters {
        for qa in by_caster.remove(&caster_id).unwrap_or_default() {
            messages.extend(ability_cast_messages(&qa, entity_names));
            target_effects
                .entry(qa.target_id)
                .or_insert_with(Vec::new)
                .extend(qa.ability.effects.clone());
        }
    }
    for abilities in by_caster.into_values() {
        for qa in abilities {
            messages.extend(ability_cast_messages(&qa, entity_names));
            target_effects
                .entry(qa.target_id)
                .or_insert_with(Vec::new)
                .extend(qa.ability.effects.clone());
        }
    }

    for (target_id, effects) in target_effects {
        let applied = resolve_effects(target_id, effects, &mut entities);
        messages.extend(applied);
    }
}

fn speed_sort_casters(
    caster_ids: &[i64],
    entities: &HashMap<i64, Character>,
    config: &AttributeConfig,
) -> Vec<i64> {
    let entity_refs: Vec<&Character> = caster_ids
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .filter_map(|&id| entities.get(&id))
        .collect();
    TurnOrder::new_from_entities(&entity_refs, config)
        .order()
        .to_vec()
}

fn ability_cast_messages(
    qa: &QueuedAbility,
    entity_names: &HashMap<i64, String>,
) -> Vec<BattleMessage> {
    let caster_name = entity_names
        .get(&qa.caster_id)
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string());
    let target_name = entity_names
        .get(&qa.target_id)
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string());

    let effect_lines: Vec<BattleMessage> = qa
        .ability
        .effects
        .iter()
        .map(effect_text)
        .filter(|s| !s.is_empty())
        .map(BattleMessage::EffectText)
        .collect();

    let cast_message = if let Some(action_text) = &qa.ability.action_text {
        let vars = VariableMap::new()
            .insert("character", &caster_name)
            .insert("target", &target_name);
        BattleMessage::Meta(TextResolver::resolve(action_text, &vars))
    } else {
        BattleMessage::AbilityCast {
            caster_name,
            target_name,
            ability_name: qa.ability.name.clone(),
        }
    };

    std::iter::once(cast_message).chain(effect_lines).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::Ability;
    use crate::game::component::AbilityRole;
    use crate::game::component::Description;
    use crate::game::component::effect::{
        Effect, EffectDescription, EffectScope, EffectType, TriggerInfo,
    };
    use crate::game::engagement::EngagementType;

    fn make_ability(action_text: Option<&str>) -> Ability {
        Ability {
            id: "test".to_string(),
            name: "Test Strike".to_string(),
            description: Description::default(),
            effects: vec![],
            engagement_types: vec![EngagementType::Battle],
            costs: vec![],
            modifiers: vec![],
            role: AbilityRole::Attack,
            targets: vec![],
            action_text: action_text.map(|s| s.to_string()),
        }
    }

    fn make_names() -> HashMap<i64, String> {
        let mut m = HashMap::new();
        m.insert(1, "Alice".to_string());
        m.insert(2, "Bob".to_string());
        m
    }

    fn hp_effect(value: i64, text: Option<&str>) -> Effect {
        Effect {
            name: "damage".to_string(),
            effect_type: EffectType::AttributeUpdate {
                attribute_id: "hp".to_string(),
                value,
            },
            trigger_info: TriggerInfo::Once,
            description: EffectDescription {
                text: text.map(|s| s.to_string()),
                ..Default::default()
            },
            scope: EffectScope::default(),
        }
    }

    #[test]
    fn no_action_text_no_effects_returns_ability_cast() {
        let qa = QueuedAbility {
            caster_id: 1,
            ability: make_ability(None),
            target_id: 2,
        };
        let msgs = ability_cast_messages(&qa, &make_names());
        assert_eq!(
            msgs,
            vec![BattleMessage::AbilityCast {
                caster_name: "Alice".to_string(),
                target_name: "Bob".to_string(),
                ability_name: "Test Strike".to_string(),
            }]
        );
    }

    #[test]
    fn no_action_text_with_effect_appends_indented_effect_line() {
        let mut ability = make_ability(None);
        ability.effects = vec![hp_effect(-5, Some("Defend for {{abs_value}}"))];
        let qa = QueuedAbility {
            caster_id: 1,
            ability,
            target_id: 2,
        };
        let msgs = ability_cast_messages(&qa, &make_names());
        assert_eq!(
            msgs,
            vec![
                BattleMessage::AbilityCast {
                    caster_name: "Alice".to_string(),
                    target_name: "Bob".to_string(),
                    ability_name: "Test Strike".to_string(),
                },
                BattleMessage::EffectText("Defend for 5".to_string()),
            ]
        );
    }

    #[test]
    fn with_action_text_no_effects_returns_single_meta() {
        let qa = QueuedAbility {
            caster_id: 1,
            ability: make_ability(Some("{{character}} strikes {{target}}!")),
            target_id: 2,
        };
        let msgs = ability_cast_messages(&qa, &make_names());
        assert_eq!(
            msgs,
            vec![BattleMessage::Meta("Alice strikes Bob!".to_string())]
        );
    }

    #[test]
    fn with_action_text_and_hp_effect_emits_separate_indented_line() {
        let mut ability = make_ability(Some("{{character}} hits {{target}}"));
        ability.effects = vec![hp_effect(-10, None)];
        let qa = QueuedAbility {
            caster_id: 1,
            ability,
            target_id: 2,
        };
        let msgs = ability_cast_messages(&qa, &make_names());
        assert_eq!(
            msgs,
            vec![
                BattleMessage::Meta("Alice hits Bob".to_string()),
                BattleMessage::EffectText("deals 10 damage".to_string()),
            ]
        );
    }

    #[test]
    fn with_action_text_and_custom_effect_text_uses_resolved_text() {
        let mut ability = make_ability(Some("{{character}} swings ax at {{target}}"));
        ability.effects = vec![hp_effect(-15, Some("Axe chops for {{abs_value}}"))];
        let qa = QueuedAbility {
            caster_id: 1,
            ability,
            target_id: 2,
        };
        let msgs = ability_cast_messages(&qa, &make_names());
        assert_eq!(
            msgs,
            vec![
                BattleMessage::Meta("Alice swings ax at Bob".to_string()),
                BattleMessage::EffectText("Axe chops for 15".to_string()),
            ]
        );
    }
}
