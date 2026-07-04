use std::collections::HashMap;

use crate::game::engagement::{Engagement, EngagementType, TurnOrder};

use super::BattleEngagement;

pub fn new_battle(
    id: i64,
    room_id: String,
    factions: Vec<String>,
    participants: HashMap<String, Vec<i64>>,
) -> Engagement {
    let entity_ids: Vec<i64> = participants.values().flatten().copied().collect();
    let turn_order = TurnOrder::new(&entity_ids);
    let battle = BattleEngagement::new(factions, participants);
    Engagement {
        id,
        engagement_type: EngagementType::Battle,
        room_id: Some(room_id),
        entity_ids,
        turn_order,
        pending_actions: HashMap::new(),
        ticks_on_current_turn: 0,
        battle: Some(battle),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_participants() -> (Vec<String>, HashMap<String, Vec<i64>>) {
        let mut participants = HashMap::new();
        participants.insert("player".to_string(), vec![1]);
        participants.insert("enemy".to_string(), vec![2]);
        (
            vec!["player".to_string(), "enemy".to_string()],
            participants,
        )
    }

    #[test]
    fn new_battle_sets_room_id_and_type() {
        let (factions, participants) = test_participants();
        let eng = new_battle(5, "room1".to_string(), factions, participants);
        assert_eq!(eng.room_id, Some("room1".to_string()));
        assert_eq!(eng.engagement_type, EngagementType::Battle);
        let mut ids = eng.entity_ids.clone();
        ids.sort();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn new_battle_initializes_battle_engagement() {
        let (factions, participants) = test_participants();
        let eng = new_battle(5, "room1".to_string(), factions, participants);
        assert!(eng.battle.is_some());
    }
}
