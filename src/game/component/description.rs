use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Description {
    pub text: Option<String>,
    pub theme: Option<String>,
}

impl Description {
    pub fn new(text: Option<String>) -> Self {
        Self { text, theme: None }
    }
}

impl<'de> Deserialize<'de> for Description {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Text(String),
            Full {
                #[serde(default)]
                text: Option<String>,
                #[serde(default)]
                theme: Option<String>,
            },
        }

        Ok(match Option::<Repr>::deserialize(deserializer)? {
            None => Description::default(),
            Some(Repr::Text(text)) => Description {
                text: Some(text),
                theme: None,
            },
            Some(Repr::Full { text, theme }) => Description { text, theme },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_bare_string_as_text() {
        #[derive(Deserialize)]
        struct Wrapper {
            description: Description,
        }
        let wrapper: Wrapper = toml::from_str(r#"description = "A room.""#).unwrap();
        assert_eq!(wrapper.description.text.as_deref(), Some("A room."));
        assert_eq!(wrapper.description.theme, None);
    }

    #[test]
    fn deserializes_table_with_text_and_theme() {
        let toml = r#"
        [description]
        text = "A room."
        theme = "eerie"
        "#;
        #[derive(Deserialize)]
        struct Wrapper {
            description: Description,
        }
        let wrapper: Wrapper = toml::from_str(toml).unwrap();
        assert_eq!(wrapper.description.text.as_deref(), Some("A room."));
        assert_eq!(wrapper.description.theme.as_deref(), Some("eerie"));
    }

    #[test]
    fn deserializes_empty_table_as_default() {
        let toml = "[description]";
        #[derive(Deserialize)]
        struct Wrapper {
            description: Description,
        }
        let wrapper: Wrapper = toml::from_str(toml).unwrap();
        assert_eq!(wrapper.description.text, None);
        assert_eq!(wrapper.description.theme, None);
    }

    #[test]
    fn deserializes_json_null_as_default() {
        #[derive(Deserialize)]
        struct Wrapper {
            description: Description,
        }
        let wrapper: Wrapper = serde_json::from_str(r#"{"description": null}"#).unwrap();
        assert_eq!(wrapper.description, Description::default());
    }

    #[test]
    fn new_sets_no_theme() {
        let description = Description::new(Some("A room.".to_string()));
        assert_eq!(description.text.as_deref(), Some("A room."));
        assert_eq!(description.theme, None);
    }
}
