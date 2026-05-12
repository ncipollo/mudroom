#[derive(Debug, Clone, Default)]
pub struct ConversationState {
    pub options: Vec<String>,
    pub selected_index: usize,
}

impl ConversationState {
    pub fn select_next(&mut self) {
        let len = self.options.len();
        if len > 0 {
            self.selected_index = (self.selected_index + 1) % len;
        }
    }

    pub fn select_prev(&mut self) {
        let len = self.options.len();
        if len > 0 {
            self.selected_index = (self.selected_index + len - 1) % len;
        }
    }

    pub fn selected_choice(&self) -> Option<String> {
        if self.options.is_empty() {
            None
        } else {
            Some((self.selected_index + 1).to_string())
        }
    }

    pub fn reset(&mut self) {
        self.options.clear();
        self.selected_index = 0;
    }
}
