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
    pub standard: Option<String>,
    #[serde(default)]
    pub checked: Vec<CheckedDescription>,
}

impl Description {
    pub fn new(standard: Option<String>) -> Self {
        Self {
            standard,
            checked: Vec::new(),
        }
    }
}
