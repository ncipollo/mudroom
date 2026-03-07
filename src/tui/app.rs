use crate::network::NetworkEvent;
use crate::network::event::PlayerInfo;

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    PlayerSelect,
    Game,
}

pub struct App {
    pub should_quit: bool,
    pub messages: Vec<String>,
    pub input: String,
    pub scroll_offset: usize,
    pub mode: AppMode,
    pub server_url: Option<String>,
    pub client_id: Option<String>,
    pub players: Vec<PlayerInfo>,
    pub selected_index: usize,
    pub creating_player: bool,
    pub player_name_input: String,
}

impl App {
    pub fn new() -> Self {
        Self {
            should_quit: false,
            messages: vec![
                "Welcome to mudroom.".to_string(),
                "Type commands and press Enter.".to_string(),
            ],
            input: String::new(),
            scroll_offset: 0,
            mode: AppMode::Game,
            server_url: None,
            client_id: None,
            players: Vec::new(),
            selected_index: 0,
            creating_player: false,
            player_name_input: String::new(),
        }
    }

    pub fn with_player_select(server_url: String, client_id: String) -> Self {
        Self {
            should_quit: false,
            messages: Vec::new(),
            input: String::new(),
            scroll_offset: 0,
            mode: AppMode::PlayerSelect,
            server_url: Some(server_url),
            client_id: Some(client_id),
            players: Vec::new(),
            selected_index: 0,
            creating_player: false,
            player_name_input: String::new(),
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset += 1;
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn select_next(&mut self) {
        let total = self.players.len() + 1; // +1 for "Create New Player"
        if total > 0 {
            self.selected_index = (self.selected_index + 1) % total;
        }
    }

    pub fn select_prev(&mut self) {
        let total = self.players.len() + 1;
        if total > 0 {
            self.selected_index = self.selected_index.saturating_sub(1);
            if self.selected_index == 0 && self.selected_index == total {
                self.selected_index = total - 1;
            }
        }
    }

    pub fn start_create(&mut self) {
        self.creating_player = true;
        self.player_name_input.clear();
    }

    pub fn cancel_create(&mut self) {
        self.creating_player = false;
        self.player_name_input.clear();
    }

    pub fn handle_network_event(&mut self, event: NetworkEvent) {
        match event {
            NetworkEvent::StartSession { session_id } => {
                self.messages.push(format!("Session started: {session_id}"))
            }
            NetworkEvent::EndSession { session_id } => {
                self.messages.push(format!("Session ended: {session_id}"))
            }
            NetworkEvent::Ping => self.messages.push("[ping received]".to_string()),
            NetworkEvent::Pong => self.messages.push("[pong received]".to_string()),
            NetworkEvent::PlayerSelected { player_name, .. } => {
                self.messages.push(format!("Playing as: {player_name}"))
            }
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
