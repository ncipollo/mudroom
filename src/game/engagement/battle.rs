pub mod abilities;
pub mod ai;
pub mod collection;
pub mod factory;
pub mod loot;
pub mod message;
pub mod participants;
pub mod phase;
pub mod queued_ability;
pub mod resolution;
pub mod state;
pub mod tick;
pub mod timer;

use std::collections::HashMap;

pub use abilities::{
    default_attack_ability, default_defend_ability, entity_innate_battle_abilities,
};
pub use ai::{BattleAiContext, run_battle_ai};
pub use collection::Battles;
pub use message::BattleMessage;
pub use phase::BattlePhase;
pub use queued_ability::QueuedAbility;
pub use state::BattleEngagement;
pub use tick::process_ticks;

pub struct BattleTick {
    pub engagement_id: i64,
    pub all_participant_ids: Vec<i64>,
    pub messages: Vec<BattleMessage>,
    pub turn_count: u64,
    pub resolution_queue: Vec<QueuedAbility>,
    pub pending_actions: Vec<QueuedAbility>,
    pub phase: BattlePhase,
    pub factions: Vec<String>,
    pub participants: HashMap<String, Vec<i64>>,
    pub ticks_in_phase: u64,
}
