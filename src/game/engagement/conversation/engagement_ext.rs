use std::collections::HashMap;

use crate::game::engagement::{Engagement, EngagementType, Engagements, TurnOrder};

impl Engagement {
    /// Create a conversation engagement where only the player takes turns. The NPC entity is
    /// tracked in entity_ids for lookup but does not participate in the turn order.
    pub fn new_conversation(id: i64, player_entity_id: i64, npc_entity_id: i64) -> Self {
        let turn_order = TurnOrder::new(&[player_entity_id]);
        Self {
            id,
            engagement_type: EngagementType::Conversation,
            room_id: None,
            entity_ids: vec![player_entity_id, npc_entity_id],
            turn_order,
            pending_actions: HashMap::new(),
            ticks_on_current_turn: 0,
            battle: None,
        }
    }
}

impl Engagements {
    pub async fn add_conversation(&self, player_entity_id: i64, npc_entity_id: i64) -> i64 {
        let id = self.alloc_id();
        let engagement = Engagement::new_conversation(id, player_entity_id, npc_entity_id);
        self.engagements_by_id.write().await.insert(id, engagement);
        id
    }

    pub async fn find_conversation_for_entity(&self, entity_id: i64) -> Option<(i64, Vec<i64>)> {
        self.engagements_by_id
            .read()
            .await
            .values()
            .find(|e| {
                e.engagement_type == EngagementType::Conversation
                    && e.entity_ids.contains(&entity_id)
            })
            .map(|e| (e.id, e.entity_ids.clone()))
    }

    pub async fn is_entity_in_conversation(&self, entity_id: i64) -> bool {
        self.engagements_by_id.read().await.values().any(|e| {
            e.engagement_type == EngagementType::Conversation && e.entity_ids.contains(&entity_id)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_conversation_sets_room_id_to_none() {
        let eng = Engagement::new_conversation(2, 10, 20);
        assert_eq!(eng.room_id, None);
    }
}
