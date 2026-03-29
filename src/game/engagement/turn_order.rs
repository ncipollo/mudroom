/// Tracks the turn order for an engagement. Currently stubbed to sort entity ids ascending.
/// In the future, turn order will be determined by entity attributes.
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
}
