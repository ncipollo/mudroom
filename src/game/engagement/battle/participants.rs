use std::collections::HashMap;

use crate::game::engagement::EngagementType;

use super::collection::Battles;

/// Add an entity to an existing battle engagement under the given faction.
/// Returns false if the engagement does not exist or has no battle.
pub async fn add_entity(
    battles: &Battles,
    engagement_id: i64,
    faction: &str,
    entity_id: i64,
) -> bool {
    let mut map = battles.map.write().await;
    let Some(engagement) = map.get_mut(&engagement_id) else {
        return false;
    };
    let Some(battle) = &mut engagement.battle else {
        return false;
    };
    battle.add_entity(faction, entity_id);
    if !engagement.entity_ids.contains(&entity_id) {
        engagement.entity_ids.push(entity_id);
    }
    true
}

/// Remove an entity from whichever battle it currently belongs to.
/// Returns `Some((engagement_id, surviving_faction_count))` or `None` if not in any battle.
pub async fn remove_entity(battles: &Battles, entity_id: i64) -> Option<(i64, usize)> {
    let mut map = battles.map.write().await;
    for engagement in map.values_mut() {
        if engagement.engagement_type != EngagementType::Battle {
            continue;
        }
        if !engagement.entity_ids.contains(&entity_id) {
            continue;
        }
        if let Some(battle) = &mut engagement.battle {
            battle.remove_entity(entity_id);
            engagement.entity_ids.retain(|&id| id != entity_id);
            let count = battle.surviving_faction_count();
            return Some((engagement.id, count));
        }
    }
    None
}

/// Return the factions and participant map for a battle engagement, or `None`.
pub async fn snapshot(
    battles: &Battles,
    engagement_id: i64,
) -> Option<(Vec<String>, HashMap<String, Vec<i64>>)> {
    let map = battles.map.read().await;
    let engagement = map.get(&engagement_id)?;
    let battle = engagement.battle.as_ref()?;
    Some((battle.factions.clone(), battle.participants.clone()))
}
