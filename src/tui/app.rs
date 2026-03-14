use crate::network::NetworkEvent;
use crate::network::event::PlayerInfo;

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    PlayerSelect,
    Game,
}

#[derive(Debug, Clone, Default)]
pub struct ConnectionState {
    pub server_url: Option<String>,
    pub client_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PlayerSelectState {
    pub players: Vec<PlayerInfo>,
    pub selected_index: usize,
    pub creating_player: bool,
    pub player_name_input: String,
}

pub struct App {
    pub should_quit: bool,
    pub messages: Vec<String>,
    pub input: String,
    pub scroll_offset: usize,
    pub mode: AppMode,
    pub connection: ConnectionState,
    pub player_select: PlayerSelectState,
    pub current_player_id: Option<i64>,
    pub debug: bool,
}

impl App {
    pub fn new(debug: bool) -> Self {
        Self {
            should_quit: false,
            messages: vec![
                "Welcome to mudroom.".to_string(),
                "Type commands and press Enter.".to_string(),
            ],
            input: String::new(),
            scroll_offset: 0,
            mode: AppMode::Game,
            connection: ConnectionState::default(),
            player_select: PlayerSelectState::default(),
            current_player_id: None,
            debug,
        }
    }

    pub fn with_player_select(server_url: String, client_id: String, debug: bool) -> Self {
        Self {
            should_quit: false,
            messages: Vec::new(),
            input: String::new(),
            scroll_offset: 0,
            mode: AppMode::PlayerSelect,
            connection: ConnectionState {
                server_url: Some(server_url),
                client_id: Some(client_id),
            },
            player_select: PlayerSelectState::default(),
            current_player_id: None,
            debug,
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

    pub fn handle_network_event(&mut self, event: NetworkEvent) {
        match event {
            NetworkEvent::StartSession { session_id } => {
                self.messages.push(format!("Session started: {session_id}"))
            }
            NetworkEvent::EndSession { session_id } => {
                self.messages.push(format!("Session ended: {session_id}"))
            }
            NetworkEvent::Ping => {
                if self.debug {
                    self.messages.push("[ping received]".to_string());
                }
            }
            NetworkEvent::Pong => {
                if self.debug {
                    self.messages.push("[pong received]".to_string());
                }
            }
            NetworkEvent::PlayerSelected {
                player_name,
                player_id,
                ..
            } => {
                self.current_player_id = Some(player_id);
                self.messages.push(format!("Playing as: {player_name}"));
            }
            NetworkEvent::Message { player_id, content } => {
                if Some(player_id) == self.current_player_id {
                    self.messages.push(content);
                }
            }
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new(false)
    }
}
