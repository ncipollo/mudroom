use std::collections::{HashMap, VecDeque};

use tokio::sync::RwLock;

use crate::game::component::Interaction;

pub struct Mailbox {
    queue: VecDeque<Interaction>,
}

impl Mailbox {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, interaction: Interaction) {
        self.queue.push_back(interaction);
    }

    pub fn drain(&mut self) -> Vec<Interaction> {
        self.queue.drain(..).collect()
    }
}

impl Default for Mailbox {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Mailboxes {
    inner: RwLock<HashMap<i64, Mailbox>>,
}

impl Mailboxes {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub async fn push(&self, entity_id: i64, interaction: Interaction) {
        let mut map = self.inner.write().await;
        map.entry(entity_id).or_default().push(interaction);
    }

    pub async fn drain(&self, entity_id: i64) -> Vec<Interaction> {
        let mut map = self.inner.write().await;
        map.get_mut(&entity_id)
            .map(|mb| mb.drain())
            .unwrap_or_default()
    }

    pub async fn remove(&self, entity_id: i64) {
        let mut map = self.inner.write().await;
        map.remove(&entity_id);
    }
}

impl Default for Mailboxes {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::component::interaction::{Direction, Movement};

    #[test]
    fn mailbox_push_and_drain() {
        let mut mb = Mailbox::new();
        mb.push(Interaction::Movement(Movement::TryDirection(
            Direction::North,
        )));
        mb.push(Interaction::Movement(Movement::TryDirection(
            Direction::South,
        )));
        let drained = mb.drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(
            drained[0],
            Interaction::Movement(Movement::TryDirection(Direction::North))
        );
        assert_eq!(
            drained[1],
            Interaction::Movement(Movement::TryDirection(Direction::South))
        );
    }

    #[test]
    fn mailbox_drain_empty() {
        let mut mb = Mailbox::new();
        assert!(mb.drain().is_empty());
    }

    #[tokio::test]
    async fn mailboxes_push_and_drain() {
        let mailboxes = Mailboxes::new();
        mailboxes
            .push(
                1,
                Interaction::Movement(Movement::TryDirection(Direction::East)),
            )
            .await;
        mailboxes
            .push(
                1,
                Interaction::Movement(Movement::TryDirection(Direction::West)),
            )
            .await;
        let drained = mailboxes.drain(1).await;
        assert_eq!(drained.len(), 2);
        assert!(mailboxes.drain(1).await.is_empty());
    }

    #[tokio::test]
    async fn mailboxes_drain_missing_entity_returns_empty() {
        let mailboxes = Mailboxes::new();
        assert!(mailboxes.drain(99).await.is_empty());
    }

    #[tokio::test]
    async fn mailboxes_remove() {
        let mailboxes = Mailboxes::new();
        mailboxes
            .push(
                1,
                Interaction::Movement(Movement::TryDirection(Direction::North)),
            )
            .await;
        mailboxes.remove(1).await;
        assert!(mailboxes.drain(1).await.is_empty());
    }
}
