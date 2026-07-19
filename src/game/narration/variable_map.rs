use std::collections::HashMap;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct VariableMap(HashMap<String, String>);

impl VariableMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.0.insert(key.into(), value.into());
        self
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_inserts_and_retrieves() {
        let vars = VariableMap::new()
            .insert("entity", "Alice")
            .insert("target", "Bob");
        assert_eq!(vars.get("entity"), Some("Alice"));
        assert_eq!(vars.get("target"), Some("Bob"));
    }

    #[test]
    fn missing_key_returns_none() {
        let vars = VariableMap::new();
        assert_eq!(vars.get("unknown"), None);
    }
}
