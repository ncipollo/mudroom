use crate::network::NetworkEvent;

pub struct App {
    pub should_quit: bool,
    pub messages: Vec<String>,
    pub input: String,
    pub scroll_offset: usize,
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
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset += 1;
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
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
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
