use crate::game::component::AttributeCategory;
use crate::game::config::AttributeConfig;
use crate::game::entity::Entity;

/// Tracks the turn order for an engagement. Sorted by entity ID ascending when using `new`;
/// use `new_from_entities` to sort by Speed attribute descending.
pub struct TurnOrder {
    order: Vec<i64>,
    current_index: usize,
}

impl TurnOrder {
    pub fn new(entity_ids: &[i64]) -> Self {
        let mut order = entity_ids.to_vec();
        order.sort();
        Self {
            order,
            current_index: 0,
        }
    }

    /// Build a turn order from entities sorted by Speed attribute descending. Entities with
    /// higher total speed go first; ties are broken by ascending entity ID.
    pub fn new_from_entities(entities: &[&Entity], config: &AttributeConfig) -> Self {
        let speed_def_ids: Vec<&str> = config
            .attributes
            .iter()
            .filter(|def| def.attribute_category == AttributeCategory::Speed)
            .map(|def| def.id.as_str())
            .collect();

        let mut entity_speeds: Vec<(i64, i64)> = entities
            .iter()
            .map(|entity| {
                let speed: i64 = speed_def_ids
                    .iter()
                    .filter_map(|&def_id| entity.attributes.get(def_id))
                    .map(|attr| attr.current_value)
                    .sum();
                (entity.id, speed)
            })
            .collect();

        entity_speeds.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

        Self {
            order: entity_speeds.into_iter().map(|(id, _)| id).collect(),
            current_index: 0,
        }
    }

    pub fn current(&self) -> Option<i64> {
        self.order.get(self.current_index).copied()
    }

    pub fn advance(&mut self) {
        if self.order.is_empty() {
            return;
        }
        self.current_index = (self.current_index + 1) % self.order.len();
    }

    pub fn order(&self) -> &[i64] {
        &self.order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sorts_by_entity_id_ascending() {
        let turn_order = TurnOrder::new(&[30, 10, 20]);
        assert_eq!(turn_order.order(), &[10, 20, 30]);
    }

    #[test]
    fn current_returns_first_entity() {
        let turn_order = TurnOrder::new(&[5, 3, 1]);
        assert_eq!(turn_order.current(), Some(1));
    }

    #[test]
    fn advance_moves_to_next() {
        let mut turn_order = TurnOrder::new(&[1, 2, 3]);
        assert_eq!(turn_order.current(), Some(1));
        turn_order.advance();
        assert_eq!(turn_order.current(), Some(2));
        turn_order.advance();
        assert_eq!(turn_order.current(), Some(3));
    }

    #[test]
    fn advance_wraps_around() {
        let mut turn_order = TurnOrder::new(&[1, 2]);
        turn_order.advance();
        turn_order.advance();
        assert_eq!(turn_order.current(), Some(1));
    }

    #[test]
    fn current_returns_none_when_empty() {
        let turn_order = TurnOrder::new(&[]);
        assert_eq!(turn_order.current(), None);
    }

    #[test]
    fn advance_noop_when_empty() {
        let mut turn_order = TurnOrder::new(&[]);
        turn_order.advance(); // should not panic
        assert_eq!(turn_order.current(), None);
    }

    #[test]
    fn new_from_entities_sorts_by_speed_descending() {
        use crate::game::component::location::Location;
        use crate::game::component::{
            Attribute, AttributeCategory, AttributeDefinition, AttributeType,
        };
        use crate::game::entity::{Entity, EntityType};

        let config = AttributeConfig {
            attributes: vec![AttributeDefinition {
                id: "speed".to_string(),
                title: "Speed".to_string(),
                description: "Quickness.".to_string(),
                min_value: 0,
                max_value: 100,
                attribute_type: AttributeType::Stat,
                attribute_category: AttributeCategory::Speed,
            }],
        };

        let loc = Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        };

        let mut slow = Entity::new(1, EntityType::Enemy, loc.clone());
        slow.attributes.insert(
            "speed".to_string(),
            Attribute::new("speed".to_string(), 0, 100, 5),
        );

        let mut fast = Entity::new(2, EntityType::Enemy, loc.clone());
        fast.attributes.insert(
            "speed".to_string(),
            Attribute::new("speed".to_string(), 0, 100, 20),
        );

        let mut medium = Entity::new(3, EntityType::Enemy, loc);
        medium.attributes.insert(
            "speed".to_string(),
            Attribute::new("speed".to_string(), 0, 100, 10),
        );

        let order = TurnOrder::new_from_entities(&[&slow, &fast, &medium], &config);
        assert_eq!(order.order(), &[2, 3, 1]);
    }

    #[test]
    fn new_from_entities_tie_breaks_by_entity_id_ascending() {
        use crate::game::component::location::Location;
        use crate::game::component::{
            Attribute, AttributeCategory, AttributeDefinition, AttributeType,
        };
        use crate::game::entity::{Entity, EntityType};

        let config = AttributeConfig {
            attributes: vec![AttributeDefinition {
                id: "speed".to_string(),
                title: "Speed".to_string(),
                description: "Quickness.".to_string(),
                min_value: 0,
                max_value: 100,
                attribute_type: AttributeType::Stat,
                attribute_category: AttributeCategory::Speed,
            }],
        };

        let loc = Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        };

        let mut e5 = Entity::new(5, EntityType::Enemy, loc.clone());
        e5.attributes.insert(
            "speed".to_string(),
            Attribute::new("speed".to_string(), 0, 100, 10),
        );

        let mut e3 = Entity::new(3, EntityType::Enemy, loc);
        e3.attributes.insert(
            "speed".to_string(),
            Attribute::new("speed".to_string(), 0, 100, 10),
        );

        let order = TurnOrder::new_from_entities(&[&e5, &e3], &config);
        assert_eq!(order.order(), &[3, 5]);
    }

    #[test]
    fn new_from_entities_falls_back_to_entity_id_when_no_speed_attr() {
        use crate::game::component::location::Location;
        use crate::game::entity::{Entity, EntityType};

        let config = AttributeConfig { attributes: vec![] };

        let loc = Location {
            world_id: "w".to_string(),
            dungeon_id: "d".to_string(),
            room_id: "r".to_string(),
        };

        let e3 = Entity::new(3, EntityType::Enemy, loc.clone());
        let e1 = Entity::new(1, EntityType::Enemy, loc);

        let order = TurnOrder::new_from_entities(&[&e3, &e1], &config);
        assert_eq!(order.order(), &[1, 3]);
    }
}
