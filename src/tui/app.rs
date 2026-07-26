mod battle_state;
mod conversation_state;
mod network_event_handler;

pub use battle_state::{BattleFocus, BattleState, QueuedAbilityInfo};
pub use conversation_state::ConversationState;

use crate::network::event::{ClassInfo, PlayerInfo};
use crate::tui::components::theme::{MessageKind, MessageTheme};

#[derive(Debug, Clone)]
pub struct AppMessage {
    pub text: String,
    pub kind: MessageKind,
}

impl AppMessage {
    pub fn normal(text: impl Into<String>) -> Self {
        Self::with_kind(text, MessageKind::Narration)
    }

    pub fn command(text: impl Into<String>) -> Self {
        Self::with_kind(text, MessageKind::PlayerCommand)
    }

    pub fn system(text: impl Into<String>) -> Self {
        Self::with_kind(text, MessageKind::System)
    }

    pub fn debug(text: impl Into<String>) -> Self {
        Self::with_kind(text, MessageKind::Debug)
    }

    fn with_kind(text: impl Into<String>, kind: MessageKind) -> Self {
        Self {
            text: text.into(),
            kind,
        }
    }
}

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
    pub scroll_offset: usize,
    pub mode: GameMode,
    pub connection: ConnectionState,
    pub player_select: PlayerSelectState,
    pub conversation: ConversationState,
    pub battle: Option<BattleState>,
    pub current_player_id: Option<i64>,
    pub current_entity_id: Option<i64>,
    pub streaming_message_index: Option<usize>,
    pub agent_responding: bool,
    pub debug: bool,
    pub theme: MessageTheme,
}

impl App {
    pub fn new(debug: bool) -> Self {
        Self {
            should_quit: false,
            messages: vec![
                AppMessage::system("Welcome to mudroom."),
                AppMessage::system("Type commands and press Enter."),
            ],
            input: String::new(),
            scroll_offset: 0,
            mode: GameMode::Game,
            connection: ConnectionState::default(),
            player_select: PlayerSelectState::default(),
            conversation: ConversationState::default(),
            battle: None,
            current_player_id: None,
            current_entity_id: None,
            streaming_message_index: None,
            agent_responding: false,
            debug,
            theme: MessageTheme,
        }
    }

    pub fn with_player_select(server_url: String, client_id: String, debug: bool) -> Self {
        Self {
            should_quit: false,
            messages: Vec::<AppMessage>::new(),
            input: String::new(),
            scroll_offset: 0,
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
            debug,
            theme: MessageTheme,
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset += 1;
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
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
