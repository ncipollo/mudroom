use crate::game::config::character_config::{DialogLine, PlayerResponse};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::error::Error;

/// Parses a markdown dialog tree into a `DialogLine`: headings become player
/// choices (nesting = sub-choices), `**alt-N**` paragraphs are alternates,
/// and everything else is greeting/reply text.
pub fn parse_dialog_markdown(content: &str) -> Result<DialogLine, Box<dyn Error>> {
    let parser = Parser::new(content);
    let events: Vec<Event> = parser.collect();
    let blocks = collect_blocks(&events);
    build_dialog_from_blocks(&blocks, 1)
}

/// A coarse block extracted from the markdown event stream.
#[derive(Debug, Clone)]
enum Block {
    Text(String),
    Alt,
    Heading(u8, String),
}

/// Collects the event stream into high-level blocks, splitting a paragraph that starts
/// with a bold `**alt-N**` marker into a separate `Alt` block plus its trailing text.
fn collect_blocks(events: &[Event]) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut i = 0;

    while i < events.len() {
        match &events[i] {
            Event::Start(Tag::Paragraph) => {
                if is_alt_start(events, i) {
                    let (alt_text, consumed) = read_alt_paragraph(events, i);
                    i += consumed;
                    blocks.push(Block::Alt);
                    if !alt_text.is_empty() {
                        blocks.push(Block::Text(alt_text));
                    }
                } else {
                    let (text, consumed) = read_paragraph(events, i);
                    i += consumed;
                    blocks.push(Block::Text(text));
                }
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let (text, consumed) = read_heading(events, i);
                let depth = heading_depth(*level);
                i += consumed;
                blocks.push(Block::Heading(depth, text));
            }
            _ => {
                i += 1;
            }
        }
    }

    blocks
}

/// Returns true when a paragraph starts with a bold `**alt-…**` inline marker.
fn is_alt_start(events: &[Event], start: usize) -> bool {
    if !matches!(events.get(start + 1), Some(Event::Start(Tag::Strong))) {
        return false;
    }
    if let Some(Event::Text(t)) = events.get(start + 2) {
        t.starts_with("alt-") || t.starts_with("alt ")
    } else {
        false
    }
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
            Some(Event::Code(t)) => {
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

/// Reads an alt paragraph, skipping the bold `**alt-N**` marker (4 events: Start(Paragraph),
/// Start(Strong), Text, End(Strong)) and an optional break, then collecting the rest as the
/// alternate text. Returns (alt_content, events_consumed).
fn read_alt_paragraph(events: &[Event], start: usize) -> (String, usize) {
    let mut i = start + 4;
    if matches!(events.get(i), Some(Event::SoftBreak | Event::HardBreak)) {
        i += 1;
    }

    let mut parts = Vec::new();
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

/// Splits blocks into the prefix before the first `choice_depth` heading (the dialog/reply
/// text) and the sections that follow, keyed by their heading text. A heading shallower than
/// `choice_depth` closes the section list early.
fn split_into_sections(
    blocks: &[Block],
    choice_depth: u8,
) -> (Vec<&Block>, Vec<(String, Vec<&Block>)>) {
    let mut prefix: Vec<&Block> = Vec::new();
    let mut sections: Vec<(String, Vec<&Block>)> = Vec::new();

    for block in blocks {
        if sections.is_empty() {
            match block {
                Block::Heading(d, text) if *d == choice_depth => {
                    sections.push((text.clone(), Vec::new()));
                }
                _ => prefix.push(block),
            }
        } else {
            match block {
                Block::Heading(d, text) if *d == choice_depth => {
                    sections.push((text.clone(), Vec::new()));
                }
                Block::Heading(d, _) if *d < choice_depth => break,
                _ => {
                    if let Some(last) = sections.last_mut() {
                        last.1.push(block);
                    }
                }
            }
        }
    }

    (prefix, sections)
}

/// Builds a `DialogLine` from a slice of blocks, treating headings at `choice_depth` as player
/// response options (`choice_depth` starts at 1 for top-level H1 choices).
fn build_dialog_from_blocks(
    blocks: &[Block],
    choice_depth: u8,
) -> Result<DialogLine, Box<dyn Error>> {
    let (prefix, sections) = split_into_sections(blocks, choice_depth);
    let dialog_text = build_text_with_alts(&prefix);

    let mut responses = Vec::new();
    for (choice_text, body_blocks) in sections {
        let reply = build_reply(&body_blocks, choice_depth)?;
        responses.push(PlayerResponse {
            text: choice_text,
            reply,
        });
    }

    Ok(DialogLine {
        text: dialog_text.text,
        alts: dialog_text.alts,
        responses,
    })
}

/// Builds the NPC reply for one player choice: text before any `choice_depth + 1` sub-heading
/// becomes the reply's own text, and everything from that sub-heading on is parsed recursively
/// as its sub-choices.
fn build_reply(
    body_blocks: &[&Block],
    choice_depth: u8,
) -> Result<Option<Box<DialogLine>>, Box<dyn Error>> {
    if body_blocks.is_empty() {
        return Ok(None);
    }

    let sub_depth = choice_depth + 1;
    let first_sub = body_blocks
        .iter()
        .position(|b| matches!(b, Block::Heading(d, _) if *d == sub_depth));
    let (npc_prefix, sub_blocks) = match first_sub {
        Some(pos) => (&body_blocks[..pos], &body_blocks[pos..]),
        None => (body_blocks, &[][..]),
    };

    let npc_text = build_text_with_alts(npc_prefix);
    let owned_sub: Vec<Block> = sub_blocks.iter().map(|b| (**b).clone()).collect();
    let sub_dialog = build_dialog_from_blocks(&owned_sub, sub_depth)?;

    Ok(combine_npc_reply(npc_text, sub_dialog))
}

/// Merges NPC reply text into the sub-dialog it introduces: if there's reply text it wins
/// (overwriting the sub-dialog's own text/alts); otherwise the sub-dialog stands alone, unless
/// it too is empty, in which case there's no reply at all.
fn combine_npc_reply(
    npc_text: TextWithAlts,
    mut sub_dialog: DialogLine,
) -> Option<Box<DialogLine>> {
    if !npc_text.text.is_empty() {
        sub_dialog.text = npc_text.text;
        sub_dialog.alts = npc_text.alts;
        return Some(Box::new(sub_dialog));
    }
    if !sub_dialog.text.is_empty() || !sub_dialog.responses.is_empty() {
        return Some(Box::new(sub_dialog));
    }
    None
}

struct TextWithAlts {
    text: String,
    alts: Vec<String>,
}

/// Splits alt-separated prefix blocks into primary text (before the first
/// `Alt` marker) and alternates (each subsequent group), joining consecutive
/// `Text` blocks within a group with newlines.
fn build_text_with_alts(prefix: &[&Block]) -> TextWithAlts {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();

    for block in prefix {
        match block {
            Block::Text(t) => current.push(t.clone()),
            Block::Alt => {
                groups.push(current.clone());
                current = Vec::new();
            }
            Block::Heading(_, _) => {}
        }
    }
    if !current.is_empty() || groups.is_empty() {
        groups.push(current);
    }

    let mut iter = groups.into_iter();
    let primary = iter.next().unwrap_or_default().join("\n");
    let alts: Vec<String> = iter.map(|g| g.join("\n")).collect();

    TextWithAlts {
        text: primary,
        alts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greeting_only() {
        let md = "Hello there, traveller!";
        let dialog = parse_dialog_markdown(md).unwrap();
        assert_eq!(dialog.text, "Hello there, traveller!");
        assert!(dialog.alts.is_empty());
        assert!(dialog.responses.is_empty());
    }

    #[test]
    fn greeting_with_alts() {
        let md = "\
Welcome to the tavern!

**alt-1**
Glad you could make it.

**alt-2**
The ale is fresh today.
";
        let dialog = parse_dialog_markdown(md).unwrap();
        assert_eq!(dialog.text, "Welcome to the tavern!");
        assert_eq!(dialog.alts.len(), 2);
        assert_eq!(dialog.alts[0], "Glad you could make it.");
        assert_eq!(dialog.alts[1], "The ale is fresh today.");
        assert!(dialog.responses.is_empty());
    }

    #[test]
    fn single_player_choice_no_reply() {
        let md = "\
Greetings.

# Goodbye.
";
        let dialog = parse_dialog_markdown(md).unwrap();
        assert_eq!(dialog.text, "Greetings.");
        assert_eq!(dialog.responses.len(), 1);
        assert_eq!(dialog.responses[0].text, "Goodbye.");
        assert!(dialog.responses[0].reply.is_none());
    }

    #[test]
    fn single_player_choice_with_reply() {
        let md = "\
Welcome!

# I'd like a room.

That'll be 5 gold.
";
        let dialog = parse_dialog_markdown(md).unwrap();
        assert_eq!(dialog.text, "Welcome!");
        assert_eq!(dialog.responses.len(), 1);
        assert_eq!(dialog.responses[0].text, "I'd like a room.");
        let reply = dialog.responses[0].reply.as_ref().unwrap();
        assert_eq!(reply.text, "That'll be 5 gold.");
        assert!(reply.responses.is_empty());
    }

    #[test]
    fn multiple_top_level_choices() {
        let md = "\
How can I help?

# Just looking.

Take your time.

# I need supplies.

Right this way.
";
        let dialog = parse_dialog_markdown(md).unwrap();
        assert_eq!(dialog.responses.len(), 2);
        assert_eq!(dialog.responses[0].text, "Just looking.");
        assert_eq!(dialog.responses[1].text, "I need supplies.");
        assert_eq!(
            dialog.responses[0].reply.as_ref().unwrap().text,
            "Take your time."
        );
        assert_eq!(
            dialog.responses[1].reply.as_ref().unwrap().text,
            "Right this way."
        );
    }

    #[test]
    fn nested_sub_choices() {
        let md = "\
Welcome!

# I'd like a room.

That'll be 5 gold. Will you be staying?

## Yes, here you go.

Enjoy your stay!

## Never mind.

Come back anytime!
";
        let dialog = parse_dialog_markdown(md).unwrap();
        assert_eq!(dialog.responses.len(), 1);
        let reply = dialog.responses[0].reply.as_ref().unwrap();
        assert_eq!(reply.text, "That'll be 5 gold. Will you be staying?");
        assert_eq!(reply.responses.len(), 2);
        assert_eq!(reply.responses[0].text, "Yes, here you go.");
        assert_eq!(reply.responses[1].text, "Never mind.");
        assert_eq!(
            reply.responses[0].reply.as_ref().unwrap().text,
            "Enjoy your stay!"
        );
        assert_eq!(
            reply.responses[1].reply.as_ref().unwrap().text,
            "Come back anytime!"
        );
    }

    #[test]
    fn alts_in_npc_reply() {
        let md = "\
Hello!

# Tell me about yourself.

I am the innkeeper.

**alt-1**
I run this fine establishment.
";
        let dialog = parse_dialog_markdown(md).unwrap();
        let reply = dialog.responses[0].reply.as_ref().unwrap();
        assert_eq!(reply.text, "I am the innkeeper.");
        assert_eq!(reply.alts.len(), 1);
        assert_eq!(reply.alts[0], "I run this fine establishment.");
    }
}
