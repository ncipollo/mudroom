use serde::{Deserialize, Serialize};

use super::check::Check;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckedDescription {
    pub check: Check,
    pub description: String,
}

impl CheckedDescription {
    pub fn new(check: Check, description: String) -> Self {
        Self { check, description }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Description {
    pub basic: Option<String>,
    pub checked: Vec<CheckedDescription>,
}

impl Description {
    pub fn new(basic: Option<String>) -> Self {
        Self {
            basic,
            checked: Vec::new(),
        }
    }
}
