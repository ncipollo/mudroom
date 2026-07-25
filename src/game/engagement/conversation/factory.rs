use std::collections::HashMap;

use crate::game::engagement::{AttributeSnapshots, Engagement, EngagementType, TurnOrder};

pub fn new_conversation(id: i64, player_entity_id: i64, npc_entity_id: i64) -> Engagement {
    let turn_order = TurnOrder::new(&[player_entity_id]);
    Engagement {
        id,
        engagement_type: EngagementType::Conversation,
        room_id: None,
        entity_ids: vec![player_entity_id, npc_entity_id],
        turn_order,
        pending_actions: HashMap::new(),
        ticks_on_current_turn: 0,
        battle: None,
        attribute_snapshots: AttributeSnapshots::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_conversation_sets_room_id_to_none() {
        let eng = new_conversation(2, 10, 20);
        assert_eq!(eng.room_id, None);
    }

    #[test]
    fn new_conversation_only_player_in_turn_order() {
        let eng = new_conversation(2, 10, 20);
        assert_eq!(eng.turn_order.order(), &[10]);
    }

    #[test]
    fn new_conversation_both_entities_tracked() {
        let eng = new_conversation(2, 10, 20);
        assert!(eng.entity_ids.contains(&10));
        assert!(eng.entity_ids.contains(&20));
    }
}
