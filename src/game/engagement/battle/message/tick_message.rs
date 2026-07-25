use std::collections::HashMap;

use crate::game::engagement::battle::{BattleMessage, QueuedAbility};

/// Assembles the full ordered message log for one battle tick: the phase-transition/effect
/// messages `turn.rs` already produced, plus pending-attack, ability-cast, and death messages
/// gathered for this tick.
pub(in crate::game::engagement::battle) fn assemble_tick_messages(
    base_messages: &[BattleMessage],
    pending_actions: &[QueuedAbility],
    cast_messages: &[BattleMessage],
    dead_ids: &[i64],
    entity_names: &HashMap<i64, String>,
) -> Vec<BattleMessage> {
    let death_messages: Vec<BattleMessage> = dead_ids
        .iter()
        .map(|&id| BattleMessage::EntityDied {
            name: entity_names
                .get(&id)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string()),
        })
        .collect();

    let pending_attack_messages: Vec<BattleMessage> = pending_actions
        .iter()
        .map(|qa| pending_attack_message(qa, entity_names))
        .collect();

    base_messages
        .iter()
        .chain(pending_attack_messages.iter())
        .chain(cast_messages.iter())
        .chain(death_messages.iter())
        .cloned()
        .collect()
}

fn pending_attack_message(
    qa: &QueuedAbility,
    entity_names: &HashMap<i64, String>,
) -> BattleMessage {
    BattleMessage::PendingAttack {
        caster_name: entity_names
            .get(&qa.caster_id)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string()),
        ability_name: qa.ability.name.clone(),
        target_name: entity_names
            .get(&qa.target_id)
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string()),
        target_id: qa.target_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> HashMap<i64, String> {
        let mut m = HashMap::new();
        m.insert(1, "Alice".to_string());
        m.insert(2, "Bob".to_string());
        m
    }

    #[test]
    fn pending_attack_message_looks_up_caster_and_target_names() {
        use crate::game::component::{Ability, AbilityRole};
        use crate::game::engagement::EngagementType;

        let qa = QueuedAbility {
            caster_id: 1,
            ability: Ability {
                id: "slash".to_string(),
                name: "Slash".to_string(),
                description: None,
                effects: vec![],
                engagement_types: vec![EngagementType::Battle],
                costs: vec![],
                modifiers: vec![],
                role: AbilityRole::Attack,
                targets: vec![],
                action_text: None,
            },
            target_id: 2,
        };

        let msg = pending_attack_message(&qa, &names());
        assert_eq!(
            msg,
            BattleMessage::PendingAttack {
                caster_name: "Alice".to_string(),
                ability_name: "Slash".to_string(),
                target_name: "Bob".to_string(),
                target_id: 2,
            }
        );
    }

    #[test]
    fn assemble_tick_messages_orders_base_pending_cast_then_death() {
        let base = vec![BattleMessage::Meta("phase change".to_string())];
        let cast = vec![BattleMessage::EffectText("deals 5 damage".to_string())];
        let dead_ids = vec![1];

        let assembled = assemble_tick_messages(&base, &[], &cast, &dead_ids, &names());

        assert_eq!(
            assembled,
            vec![
                BattleMessage::Meta("phase change".to_string()),
                BattleMessage::EffectText("deals 5 damage".to_string()),
                BattleMessage::EntityDied {
                    name: "Alice".to_string()
                },
            ]
        );
    }

    #[test]
    fn assemble_tick_messages_uses_unknown_for_missing_dead_entity_name() {
        let assembled = assemble_tick_messages(&[], &[], &[], &[99], &names());
        assert_eq!(
            assembled,
            vec![BattleMessage::EntityDied {
                name: "Unknown".to_string()
            }]
        );
    }
}
