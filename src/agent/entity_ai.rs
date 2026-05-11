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

#[derive(Debug, Clone)]
pub enum AgentRole {
    Player,
    Agent,
}

#[derive(Debug, Clone)]
pub struct AgentMessage {
    pub role: AgentRole,
    pub content: String,
}

#[derive(Debug, Clone, Default)]
pub struct AgentConversationContext {
    pub instructions: String,
    pub history: Vec<AgentMessage>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentConversationState {
    pub contexts: HashMap<i64, AgentConversationContext>,
}

#[derive(Debug, Clone, Default)]
pub struct EntityAI {
    pub simple_conversation_state: Option<SimpleConversationState>,
    pub agent_conversation_state: Option<AgentConversationState>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_conversation_state_insert_and_retrieve() {
        let mut state = AgentConversationState::default();
        state.contexts.insert(
            1,
            AgentConversationContext {
                instructions: "Be helpful.".to_string(),
                history: vec![AgentMessage {
                    role: AgentRole::Player,
                    content: "Hello".to_string(),
                }],
            },
        );
        assert!(state.contexts.contains_key(&1));
        assert_eq!(state.contexts[&1].instructions, "Be helpful.");
        assert_eq!(state.contexts[&1].history.len(), 1);
    }

    #[test]
    fn agent_conversation_state_remove_clears_context() {
        let mut state = AgentConversationState::default();
        state
            .contexts
            .insert(2, AgentConversationContext::default());
        state.contexts.remove(&2);
        assert!(!state.contexts.contains_key(&2));
    }
}
