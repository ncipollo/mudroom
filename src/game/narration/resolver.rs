use super::variable_map::VariableMap;

pub struct TextResolver;

impl TextResolver {
    pub fn resolve(text: &str, vars: &VariableMap) -> String {
        let mut result = String::with_capacity(text.len());
        let mut remaining = text;

        while let Some(open) = remaining.find("{{") {
            result.push_str(&remaining[..open]);
            let after_open = &remaining[open + 2..];
            if let Some(close) = after_open.find("}}") {
                let key = &after_open[..close];
                if let Some(value) = vars.get(key) {
                    result.push_str(value);
                } else {
                    result.push_str("{{");
                    result.push_str(key);
                    result.push_str("}}");
                }
                remaining = &after_open[close + 2..];
            } else {
                result.push_str("{{");
                remaining = after_open;
            }
        }
        result.push_str(remaining);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> VariableMap {
        VariableMap::new()
            .insert("character", "Alice")
            .insert("target", "Bob")
    }

    #[test]
    fn resolves_known_vars() {
        let result = TextResolver::resolve("{{character}} attacks {{target}}!", &vars());
        assert_eq!(result, "Alice attacks Bob!");
    }

    #[test]
    fn unknown_var_left_as_literal() {
        let result = TextResolver::resolve("{{character}} hits for {{effect}}", &vars());
        assert_eq!(result, "Alice hits for {{effect}}");
    }

    #[test]
    fn empty_text_returns_empty() {
        let result = TextResolver::resolve("", &vars());
        assert_eq!(result, "");
    }

    #[test]
    fn no_placeholders_returned_unchanged() {
        let result = TextResolver::resolve("no vars here", &vars());
        assert_eq!(result, "no vars here");
    }

    #[test]
    fn unclosed_brace_left_as_is() {
        let result = TextResolver::resolve("{{character}} and {{unclosed", &vars());
        assert_eq!(result, "Alice and {{unclosed");
    }
}
