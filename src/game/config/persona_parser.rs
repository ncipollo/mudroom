use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Clone)]
pub struct PersonaFrontMatter {
    pub name: Option<String>,
    pub role: Option<String>,
    pub extra: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompareOp {
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    Equal,
}

#[derive(Debug, Clone)]
pub enum PersonaCondition {
    Trust {
        op: CompareOp,
        value: f64,
    },
    Attribute {
        name: String,
        op: CompareOp,
        value: f64,
    },
}

#[derive(Debug, Clone)]
pub struct PersonaSection {
    pub title: String,
    pub conditions: Vec<PersonaCondition>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct PersonaFile {
    pub front_matter: PersonaFrontMatter,
    pub preamble: String,
    pub sections: Vec<PersonaSection>,
}

#[derive(Debug, Clone)]
pub struct PersonaContext {
    pub trust: f64,
    pub attributes: HashMap<String, f64>,
}

impl PersonaCondition {
    pub fn evaluate(&self, context: &PersonaContext) -> bool {
        match self {
            PersonaCondition::Trust { op, value } => apply_op(op, context.trust, *value),
            PersonaCondition::Attribute { name, op, value } => {
                let attr = context.attributes.get(name).copied().unwrap_or(0.0);
                apply_op(op, attr, *value)
            }
        }
    }
}

fn apply_op(op: &CompareOp, lhs: f64, rhs: f64) -> bool {
    match op {
        CompareOp::GreaterThan => lhs > rhs,
        CompareOp::GreaterThanOrEqual => lhs >= rhs,
        CompareOp::LessThan => lhs < rhs,
        CompareOp::LessThanOrEqual => lhs <= rhs,
        CompareOp::Equal => (lhs - rhs).abs() < f64::EPSILON,
    }
}

impl PersonaFile {
    pub fn to_instructions(&self, context: &PersonaContext) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if !self.preamble.is_empty() {
            parts.push(&self.preamble);
        }
        for section in &self.sections {
            let all_match = section.conditions.iter().all(|c| c.evaluate(context));
            if all_match && !section.text.is_empty() {
                parts.push(&section.text);
            }
        }
        parts.join("\n\n")
    }
}

pub fn parse_persona_markdown(content: &str) -> Result<PersonaFile, Box<dyn Error>> {
    let (front_matter_str, body) = split_front_matter(content);
    let front_matter = parse_front_matter(front_matter_str)?;
    let blocks = collect_blocks(body);
    let (preamble, sections) = build_sections(&blocks)?;
    Ok(PersonaFile {
        front_matter,
        preamble,
        sections,
    })
}

fn split_front_matter(content: &str) -> (&str, &str) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return ("", content);
    }
    let after_open = &trimmed[3..];
    // Find the closing ---
    if let Some(close) = after_open.find("\n---") {
        let fm = &after_open[..close];
        let body = &after_open[close + 4..]; // skip \n---
        // Skip optional newline after closing delimiter
        let body = body.trim_start_matches('\n');
        (fm.trim(), body)
    } else {
        ("", content)
    }
}

fn parse_front_matter(yaml: &str) -> Result<PersonaFrontMatter, Box<dyn Error>> {
    if yaml.is_empty() {
        return Ok(PersonaFrontMatter {
            name: None,
            role: None,
            extra: HashMap::new(),
        });
    }
    let value: serde_yml::Value = serde_yml::from_str(yaml)?;
    let mut name = None;
    let mut role = None;
    let mut extra = HashMap::new();
    if let serde_yml::Value::Mapping(map) = value {
        for (k, v) in map {
            let key = yaml_value_to_string(&k);
            let val = yaml_value_to_string(&v);
            match key.as_str() {
                "name" => name = Some(val),
                "role" => role = Some(val),
                _ => {
                    extra.insert(key, val);
                }
            }
        }
    }
    Ok(PersonaFrontMatter { name, role, extra })
}

fn yaml_value_to_string(v: &serde_yml::Value) -> String {
    match v {
        serde_yml::Value::String(s) => s.clone(),
        serde_yml::Value::Number(n) => n.to_string(),
        serde_yml::Value::Bool(b) => b.to_string(),
        serde_yml::Value::Null => String::new(),
        _ => String::new(),
    }
}

#[derive(Debug, Clone)]
enum Block {
    Text(String),
    Heading(u8, String),
    Code { lang: String, content: String },
}

fn collect_blocks(content: &str) -> Vec<Block> {
    let parser = Parser::new(content);
    let events: Vec<Event> = parser.collect();
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < events.len() {
        match &events[i] {
            Event::Start(Tag::Paragraph) => {
                let (text, consumed) = read_paragraph(&events, i);
                i += consumed;
                if !text.is_empty() {
                    blocks.push(Block::Text(text));
                }
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let (text, consumed) = read_heading(&events, i);
                let depth = heading_depth(*level);
                i += consumed;
                blocks.push(Block::Heading(depth, text));
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                    pulldown_cmark::CodeBlockKind::Indented => String::new(),
                };
                let (code, consumed) = read_code_block(&events, i);
                i += consumed;
                blocks.push(Block::Code {
                    lang,
                    content: code,
                });
            }
            _ => {
                i += 1;
            }
        }
    }

    blocks
}

fn read_paragraph(events: &[Event], start: usize) -> (String, usize) {
    let mut parts = Vec::new();
    let mut i = start + 1;
    loop {
        match events.get(i) {
            None | Some(Event::End(TagEnd::Paragraph)) => {
                i += 1;
                break;
            }
            Some(Event::Text(t)) => {
                parts.push(t.to_string());
                i += 1;
            }
            Some(Event::SoftBreak | Event::HardBreak) => {
                parts.push("\n".to_string());
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    (parts.join("").trim().to_string(), i - start)
}

fn read_heading(events: &[Event], start: usize) -> (String, usize) {
    let mut parts = Vec::new();
    let mut i = start + 1;
    loop {
        match events.get(i) {
            None | Some(Event::End(TagEnd::Heading(_))) => {
                i += 1;
                break;
            }
            Some(Event::Text(t)) => {
                parts.push(t.to_string());
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    (parts.join("").trim().to_string(), i - start)
}

fn read_code_block(events: &[Event], start: usize) -> (String, usize) {
    let mut parts = Vec::new();
    let mut i = start + 1;
    loop {
        match events.get(i) {
            None | Some(Event::End(TagEnd::CodeBlock)) => {
                i += 1;
                break;
            }
            Some(Event::Text(t)) => {
                parts.push(t.to_string());
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    (parts.join(""), i - start)
}

fn heading_depth(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn build_sections(blocks: &[Block]) -> Result<(String, Vec<PersonaSection>), Box<dyn Error>> {
    let mut preamble_parts: Vec<String> = Vec::new();
    let mut sections: Vec<PersonaSection> = Vec::new();
    let mut current_section: Option<(String, Vec<&Block>)> = None;

    for block in blocks {
        if let Block::Heading(1, title) = block {
            if let Some((sec_title, sec_blocks)) = current_section.take() {
                sections.push(build_section(sec_title, &sec_blocks)?);
            }
            current_section = Some((title.clone(), Vec::new()));
        } else if let Some((_, ref mut body)) = current_section {
            body.push(block);
        } else if let Block::Text(t) = block {
            preamble_parts.push(t.clone());
        }
    }

    if let Some((sec_title, sec_blocks)) = current_section {
        sections.push(build_section(sec_title, &sec_blocks)?);
    }

    let preamble = preamble_parts.join("\n\n");
    Ok((preamble, sections))
}

fn build_section(title: String, blocks: &[&Block]) -> Result<PersonaSection, Box<dyn Error>> {
    let mut conditions = Vec::new();
    let mut text_parts: Vec<String> = Vec::new();
    let mut iter = blocks.iter().peekable();

    // If the first block is a yml/yaml code block, parse it as conditions
    if let Some(Block::Code { lang, content }) = iter.peek().copied()
        && (lang == "yml" || lang == "yaml")
    {
        conditions = parse_conditions(content)?;
        iter.next();
    }

    for block in iter {
        if let Block::Text(t) = block {
            text_parts.push(t.clone());
        }
    }

    Ok(PersonaSection {
        title,
        conditions,
        text: text_parts.join("\n\n"),
    })
}

fn parse_conditions(yaml: &str) -> Result<Vec<PersonaCondition>, Box<dyn Error>> {
    let value: serde_yml::Value = serde_yml::from_str(yaml)?;
    let mut conditions = Vec::new();

    let conditions_map = match &value {
        serde_yml::Value::Mapping(m) => m
            .get("conditions")
            .and_then(|v| v.as_mapping())
            .cloned()
            .unwrap_or_default(),
        _ => return Ok(conditions),
    };

    for (key, val) in &conditions_map {
        let key_str = yaml_value_to_string(key);
        match key_str.as_str() {
            "trust" => {
                if let Some((op, value)) = parse_op_value(val) {
                    conditions.push(PersonaCondition::Trust { op, value });
                }
            }
            "attribute" => {
                if let serde_yml::Value::Mapping(m) = val {
                    let name = m.get("name").map(yaml_value_to_string).unwrap_or_default();
                    let op_str = m.get("op").map(yaml_value_to_string).unwrap_or_default();
                    let value = m.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    if let Some(op) = parse_op_str(&op_str) {
                        conditions.push(PersonaCondition::Attribute { name, op, value });
                    }
                }
            }
            _ => {}
        }
    }

    Ok(conditions)
}

fn parse_op_value(val: &serde_yml::Value) -> Option<(CompareOp, f64)> {
    let map = val.as_mapping()?;
    for (k, v) in map {
        let key = yaml_value_to_string(k);
        let value = v.as_f64()?;
        if let Some(op) = parse_op_str(&key) {
            return Some((op, value));
        }
    }
    None
}

fn parse_op_str(s: &str) -> Option<CompareOp> {
    match s {
        "gt" => Some(CompareOp::GreaterThan),
        "gte" => Some(CompareOp::GreaterThanOrEqual),
        "lt" => Some(CompareOp::LessThan),
        "lte" => Some(CompareOp::LessThanOrEqual),
        "eq" => Some(CompareOp::Equal),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL_EXAMPLE: &str = r#"---
name: Bramble
role: innkeeper
---

You are Bramble, a warm and weathered innkeeper at the Rusty Flagon tavern.

# Secrets

```yml
conditions:
  trust: { gt: 7 }
```

You guard a hidden cellar beneath the inn.

# Quest Hook

```yml
conditions:
  trust: { gte: 5 }
```

A shipment of rare ale was stolen three nights ago.

# Combat

```yml
conditions:
  attribute:
    name: threat_level
    op: gte
    value: 3
```

Bramble will reach for the club kept under the bar.

# Always Present
This section has no conditions.
"#;

    #[test]
    fn front_matter_name_and_role() {
        let pf = parse_persona_markdown(FULL_EXAMPLE).unwrap();
        assert_eq!(pf.front_matter.name.as_deref(), Some("Bramble"));
        assert_eq!(pf.front_matter.role.as_deref(), Some("innkeeper"));
        assert!(pf.front_matter.extra.is_empty());
    }

    #[test]
    fn front_matter_extra_keys() {
        let md = "---\nname: Zara\nalignment: neutral\n---\nHello.";
        let pf = parse_persona_markdown(md).unwrap();
        assert_eq!(pf.front_matter.name.as_deref(), Some("Zara"));
        assert_eq!(
            pf.front_matter.extra.get("alignment").map(|s| s.as_str()),
            Some("neutral")
        );
    }

    #[test]
    fn preamble_only() {
        let md = "---\nname: Bob\n---\nJust some preamble text.";
        let pf = parse_persona_markdown(md).unwrap();
        assert_eq!(pf.preamble, "Just some preamble text.");
        assert!(pf.sections.is_empty());
    }

    #[test]
    fn no_front_matter() {
        let md = "Hello there.";
        let pf = parse_persona_markdown(md).unwrap();
        assert!(pf.front_matter.name.is_none());
        assert_eq!(pf.preamble, "Hello there.");
    }

    #[test]
    fn unconditional_section_always_included() {
        let pf = parse_persona_markdown(FULL_EXAMPLE).unwrap();
        let always = pf
            .sections
            .iter()
            .find(|s| s.title == "Always Present")
            .unwrap();
        assert!(always.conditions.is_empty());
        let ctx = PersonaContext {
            trust: 0.0,
            attributes: HashMap::new(),
        };
        assert!(always.conditions.iter().all(|c| c.evaluate(&ctx)));
    }

    #[test]
    fn trust_gt_condition_included_when_high() {
        let pf = parse_persona_markdown(FULL_EXAMPLE).unwrap();
        let secrets = pf.sections.iter().find(|s| s.title == "Secrets").unwrap();
        assert_eq!(secrets.conditions.len(), 1);
        let ctx = PersonaContext {
            trust: 8.0,
            attributes: HashMap::new(),
        };
        assert!(secrets.conditions[0].evaluate(&ctx));
    }

    #[test]
    fn trust_gt_condition_excluded_when_low() {
        let pf = parse_persona_markdown(FULL_EXAMPLE).unwrap();
        let secrets = pf.sections.iter().find(|s| s.title == "Secrets").unwrap();
        let ctx = PersonaContext {
            trust: 7.0,
            attributes: HashMap::new(),
        };
        assert!(!secrets.conditions[0].evaluate(&ctx));
    }

    #[test]
    fn trust_gte_condition_included_at_boundary() {
        let pf = parse_persona_markdown(FULL_EXAMPLE).unwrap();
        let quest = pf
            .sections
            .iter()
            .find(|s| s.title == "Quest Hook")
            .unwrap();
        let ctx = PersonaContext {
            trust: 5.0,
            attributes: HashMap::new(),
        };
        assert!(quest.conditions[0].evaluate(&ctx));
    }

    #[test]
    fn attribute_condition_included_when_matches() {
        let pf = parse_persona_markdown(FULL_EXAMPLE).unwrap();
        let combat = pf.sections.iter().find(|s| s.title == "Combat").unwrap();
        let mut attrs = HashMap::new();
        attrs.insert("threat_level".to_string(), 3.0);
        let ctx = PersonaContext {
            trust: 0.0,
            attributes: attrs,
        };
        assert!(combat.conditions[0].evaluate(&ctx));
    }

    #[test]
    fn attribute_condition_excluded_when_missing() {
        let pf = parse_persona_markdown(FULL_EXAMPLE).unwrap();
        let combat = pf.sections.iter().find(|s| s.title == "Combat").unwrap();
        let ctx = PersonaContext {
            trust: 0.0,
            attributes: HashMap::new(),
        };
        assert!(!combat.conditions[0].evaluate(&ctx));
    }

    #[test]
    fn to_instructions_includes_preamble_and_matching_sections() {
        let pf = parse_persona_markdown(FULL_EXAMPLE).unwrap();
        let ctx = PersonaContext {
            trust: 8.0,
            attributes: HashMap::new(),
        };
        let instructions = pf.to_instructions(&ctx);
        assert!(instructions.contains("Bramble, a warm"));
        assert!(instructions.contains("hidden cellar"));
        assert!(
            instructions.contains("always present")
                || instructions
                    .to_lowercase()
                    .contains("this section has no conditions")
        );
    }

    #[test]
    fn to_instructions_excludes_unmet_sections() {
        let pf = parse_persona_markdown(FULL_EXAMPLE).unwrap();
        let ctx = PersonaContext {
            trust: 3.0,
            attributes: HashMap::new(),
        };
        let instructions = pf.to_instructions(&ctx);
        assert!(!instructions.contains("hidden cellar"));
        assert!(!instructions.contains("rare ale"));
        assert!(instructions.contains("Bramble, a warm"));
    }

    #[test]
    fn compare_op_less_than() {
        let cond = PersonaCondition::Trust {
            op: CompareOp::LessThan,
            value: 5.0,
        };
        let ctx = PersonaContext {
            trust: 4.0,
            attributes: HashMap::new(),
        };
        assert!(cond.evaluate(&ctx));
        let ctx2 = PersonaContext {
            trust: 5.0,
            attributes: HashMap::new(),
        };
        assert!(!cond.evaluate(&ctx2));
    }

    #[test]
    fn compare_op_less_than_or_equal() {
        let cond = PersonaCondition::Trust {
            op: CompareOp::LessThanOrEqual,
            value: 5.0,
        };
        let ctx = PersonaContext {
            trust: 5.0,
            attributes: HashMap::new(),
        };
        assert!(cond.evaluate(&ctx));
        let ctx2 = PersonaContext {
            trust: 6.0,
            attributes: HashMap::new(),
        };
        assert!(!cond.evaluate(&ctx2));
    }

    #[test]
    fn compare_op_equal() {
        let cond = PersonaCondition::Trust {
            op: CompareOp::Equal,
            value: 5.0,
        };
        let ctx = PersonaContext {
            trust: 5.0,
            attributes: HashMap::new(),
        };
        assert!(cond.evaluate(&ctx));
        let ctx2 = PersonaContext {
            trust: 5.1,
            attributes: HashMap::new(),
        };
        assert!(!cond.evaluate(&ctx2));
    }

    #[test]
    fn sections_parsed_in_order() {
        let pf = parse_persona_markdown(FULL_EXAMPLE).unwrap();
        let titles: Vec<&str> = pf.sections.iter().map(|s| s.title.as_str()).collect();
        assert_eq!(
            titles,
            ["Secrets", "Quest Hook", "Combat", "Always Present"]
        );
    }
}
