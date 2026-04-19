use std::collections::HashMap;

use crate::game::config::DialogLine;

#[derive(Debug, Clone, Default)]
pub struct ConversationContext {
    pub current_dialog: Option<DialogLine>,
}

#[derive(Debug, Clone, Default)]
pub struct SimpleConversationState {
    pub contexts: HashMap<i64, ConversationContext>,
}

#[derive(Debug, Clone, Default)]
pub struct EntityAI {
    pub simple_conversation_state: Option<SimpleConversationState>,
}
