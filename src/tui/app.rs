mod battle_state;
mod conversation_state;
mod message;
mod network_event_handler;
mod reveal;

pub use battle_state::{BattleFocus, BattleLogEntry, BattleState, QueuedAbilityInfo};
pub use conversation_state::ConversationState;
pub use message::AppMessage;

use std::collections::VecDeque;

use crate::network::event::{ClassInfo, PlayerInfo};
use crate::tui::components::scroll::ScrollState;
use crate::tui::components::theme::MessageTheme;
use crate::tui::components::typewriter::{DEFAULT_REVEAL_RATE, TypewriterState};

#[derive(Debug, Clone, PartialEq)]
pub enum GameMode {
    PlayerSelect,
    Game,
    StandardConversation,
    AgentConversation,
    Battle,
}

#[derive(Debug, Clone, Default)]
pub struct ConnectionState {
    pub server_url: Option<String>,
    pub client_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ClassSelectState {
    pub classes: Vec<ClassInfo>,
    pub selected_index: usize,
    pub active: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PlayerSelectState {
    pub players: Vec<PlayerInfo>,
    pub selected_index: usize,
    pub creating_player: bool,
    pub player_name_input: String,
    pub class_select: ClassSelectState,
}

pub struct App {
    pub should_quit: bool,
    pub messages: Vec<AppMessage>,
    pub input: String,
    pub log_scroll: ScrollState,
    pub mode: GameMode,
    pub connection: ConnectionState,
    pub player_select: PlayerSelectState,
    pub conversation: ConversationState,
    pub battle: Option<BattleState>,
    pub current_player_id: Option<i64>,
    pub current_entity_id: Option<i64>,
    pub streaming_message_index: Option<usize>,
    pub agent_responding: bool,
    pub reveal: Option<TypewriterState>,
    pub reveal_queue: VecDeque<usize>,
    pub reveal_rate: f64,
    pub debug: bool,
    pub theme: MessageTheme,
}

impl App {
    pub fn new(debug: bool) -> Self {
        let theme = MessageTheme;
        Self {
            should_quit: false,
            messages: vec![
                AppMessage::system("Welcome to mudroom.", &theme),
                AppMessage::system("Type commands and press Enter.", &theme),
            ],
            input: String::new(),
            log_scroll: ScrollState::default(),
            mode: GameMode::Game,
            connection: ConnectionState::default(),
            player_select: PlayerSelectState::default(),
            conversation: ConversationState::default(),
            battle: None,
            current_player_id: None,
            current_entity_id: None,
            streaming_message_index: None,
            agent_responding: false,
            reveal: None,
            reveal_queue: VecDeque::new(),
            reveal_rate: DEFAULT_REVEAL_RATE,
            debug,
            theme: MessageTheme,
        }
    }

    pub fn with_player_select(server_url: String, client_id: String, debug: bool) -> Self {
        Self {
            should_quit: false,
            messages: Vec::<AppMessage>::new(),
            input: String::new(),
            log_scroll: ScrollState::default(),
            mode: GameMode::PlayerSelect,
            connection: ConnectionState {
                server_url: Some(server_url),
                client_id: Some(client_id),
            },
            player_select: PlayerSelectState::default(),
            conversation: ConversationState::default(),
            battle: None,
            current_player_id: None,
            current_entity_id: None,
            streaming_message_index: None,
            agent_responding: false,
            reveal: None,
            reveal_queue: VecDeque::new(),
            reveal_rate: DEFAULT_REVEAL_RATE,
            debug,
            theme: MessageTheme,
        }
    }

    pub fn select_next(&mut self) {
        let total = self.player_select.players.len() + 1; // +1 for "Create New Player"
        if total > 0 {
            self.player_select.selected_index = (self.player_select.selected_index + 1) % total;
        }
    }

    pub fn select_prev(&mut self) {
        let total = self.player_select.players.len() + 1;
        if total > 0 {
            self.player_select.selected_index = self.player_select.selected_index.saturating_sub(1);
            if self.player_select.selected_index == 0 && self.player_select.selected_index == total
            {
                self.player_select.selected_index = total - 1;
            }
        }
    }

    pub fn start_create(&mut self) {
        self.player_select.creating_player = true;
        self.player_select.player_name_input.clear();
    }

    pub fn cancel_create(&mut self) {
        self.player_select.creating_player = false;
        self.player_select.player_name_input.clear();
    }

    pub fn start_class_select(&mut self, classes: Vec<ClassInfo>) {
        self.player_select.class_select.classes = classes;
        self.player_select.class_select.selected_index = 0;
        self.player_select.class_select.active = true;
        self.player_select.creating_player = false;
    }

    pub fn cancel_class_select(&mut self) {
        self.player_select.class_select.active = false;
        self.player_select.creating_player = true;
    }

    pub fn class_select_next(&mut self) {
        let total = self.player_select.class_select.classes.len();
        if total > 0 {
            self.player_select.class_select.selected_index =
                (self.player_select.class_select.selected_index + 1) % total;
        }
    }

    pub fn class_select_prev(&mut self) {
        let total = self.player_select.class_select.classes.len();
        if total > 0 {
            self.player_select.class_select.selected_index =
                if self.player_select.class_select.selected_index == 0 {
                    total - 1
                } else {
                    self.player_select.class_select.selected_index - 1
                };
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new(false)
    }
}
