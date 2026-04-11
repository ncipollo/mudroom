use crate::game::engagement::EngagementType;
use crate::game::engagement::TurnAction;

pub struct ResolvedAction {
    pub engagement_id: i64,
    pub engagement_type: EngagementType,
    pub entity_ids: Vec<i64>,
    pub entity_id: i64,
    pub action: Option<TurnAction>,
}
