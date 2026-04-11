use crate::game::config::entity_config::{DialogLine, PlayerResponse};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::error::Error;

/// Parses a markdown dialog tree into a `DialogLine`.
///
/// Format:
/// - Text before any heading → greeting (+ alts via `**alt-N**` paragraphs)
/// - `# H1` → player response option text
/// - Text after `# H1` (before next heading) → NPC reply
/// - `## H2` within an H1 section → sub-player choices
/// - `**alt-N**` bold-only paragraphs act as alternate separators
pub fn parse_dialog_markdown(content: &str) -> Result<DialogLine, Box<dyn Error>> {
    let parser = Parser::new(content);
    let events: Vec<Event> = parser.collect();
    let blocks = collect_blocks(&events);
    build_dialog_from_blocks(&blocks, 1)
}

/// A coarse block extracted from the markdown event stream.
#[derive(Debug, Clone)]
enum Block {
    /// A paragraph of plain text (already trimmed and joined).
    Text(String),
    /// An alt separator paragraph (`**alt-N**`); the N is discarded.
    Alt,
    /// A heading at the given depth (1 = H1, 2 = H2, …) with its text.
    Heading(u8, String),
}

/// Collect the event stream into high-level blocks.
///
/// A paragraph beginning with a bold `**alt-N**` inline element is split into
/// an `Alt` separator block plus a `Text` block for any content that follows
/// on the same or subsequent lines (without a blank line in between).
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
///
/// Event sequence at `start`:
///   Start(Paragraph), Start(Strong), Text("alt-…"), End(Strong), …
fn is_alt_start(events: &[Event], start: usize) -> bool {
    // events[start] == Start(Paragraph)
    if !matches!(events.get(start + 1), Some(Event::Start(Tag::Strong))) {
        return false;
    }
    if let Some(Event::Text(t)) = events.get(start + 2) {
        t.starts_with("alt-") || t.starts_with("alt ")
    } else {
        false
    }
}

/// Read all inline events inside a paragraph, returning the joined text and
/// how many events were consumed (including the opening/closing tags).
fn read_paragraph(events: &[Event], start: usize) -> (String, usize) {
    let mut parts = Vec::new();
    let mut i = start + 1; // skip opening Start(Paragraph)
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
            // Skip bold/italic/etc. markers; their inner Text events carry content.
            _ => {
                i += 1;
            }
        }
    }
    (parts.join("").trim().to_string(), i - start)
}

/// Read all inline events inside a heading.
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

/// Read an alt paragraph: skip the bold alt marker (and optional soft break),
/// then collect the rest of the paragraph as the alternate text.
///
/// Returns (alt_content, events_consumed).
fn read_alt_paragraph(events: &[Event], start: usize) -> (String, usize) {
    // start → Start(Paragraph)
    // start+1 → Start(Strong)
    // start+2 → Text("alt-N")
    // start+3 → End(Strong)
    // start+4 → optional SoftBreak, then content…
    let mut i = start + 4; // skip Start(Paragraph), Start(Strong), Text, End(Strong)

    // Skip an optional soft/hard break immediately after the marker.
    if matches!(events.get(i), Some(Event::SoftBreak | Event::HardBreak)) {
        i += 1;
    }

    // Now collect remaining inline content until End(Paragraph).
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

/// Build a `DialogLine` from a slice of blocks, treating headings at
/// `choice_depth` as player response options.
///
/// `choice_depth` starts at 1 (H1 = top-level player choices).
fn build_dialog_from_blocks(
    blocks: &[Block],
    choice_depth: u8,
) -> Result<DialogLine, Box<dyn Error>> {
    // Split blocks into: prefix (before first heading at choice_depth) and sections.
    let mut prefix: Vec<&Block> = Vec::new();
    let mut sections: Vec<(String, Vec<&Block>)> = Vec::new(); // (heading text, following blocks)

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
                Block::Heading(d, _) if *d < choice_depth => {
                    // Higher-level heading closes everything — stop here.
                    break;
                }
                _ => {
                    if let Some(last) = sections.last_mut() {
                        last.1.push(block);
                    }
                }
            }
        }
    }

    let dialog_text = build_text_with_alts(&prefix);
    let mut responses = Vec::new();

    for (choice_text, body_blocks) in sections {
        // The body may itself contain sub-headings at choice_depth+1.
        // Split body into: NPC reply prefix and sub-choice sections.
        let sub_depth = choice_depth + 1;
        let first_sub = body_blocks
            .iter()
            .position(|b| matches!(b, Block::Heading(d, _) if *d == sub_depth));

        let reply = if body_blocks.is_empty() {
            None
        } else {
            let (npc_prefix, sub_blocks) = if let Some(pos) = first_sub {
                (&body_blocks[..pos], &body_blocks[pos..])
            } else {
                (&body_blocks[..], &[][..])
            };

            let npc_text = build_text_with_alts(npc_prefix);
            let owned_sub: Vec<Block> = sub_blocks.iter().map(|b| (*b).clone()).collect();
            let mut sub_dialog = build_dialog_from_blocks(&owned_sub, sub_depth)?;
            if npc_text.text.is_empty() && sub_dialog.text.is_empty() {
                // No actual NPC text — treat sub as the reply directly if there are responses
                if !sub_dialog.responses.is_empty() {
                    Some(Box::new(sub_dialog))
                } else {
                    None
                }
            } else if npc_text.text.is_empty() {
                // Fold into sub_dialog
                Some(Box::new(sub_dialog))
            } else {
                // NPC says npc_text, then presents sub-choices
                sub_dialog.text = npc_text.text;
                sub_dialog.alts = npc_text.alts;
                Some(Box::new(sub_dialog))
            }
        };

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

/// Intermediate container used while collecting text + alts from a prefix.
struct TextWithAlts {
    text: String,
    alts: Vec<String>,
}

/// Given a sequence of prefix blocks (Text and Alt interleaved), build the
/// primary text and alternates.
///
/// The first Text block (before any Alt) is the primary text. Subsequent Text
/// blocks after Alt markers become alternates. Multiple consecutive Text blocks
/// (without an intervening Alt) are joined with newlines.
fn build_text_with_alts(prefix: &[&Block]) -> TextWithAlts {
    // Group into runs separated by Alt markers.
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();

    for block in prefix {
        match block {
            Block::Text(t) => current.push(t.clone()),
            Block::Alt => {
                groups.push(current.clone());
                current = Vec::new();
            }
            Block::Heading(_, _) => {} // shouldn't appear here, skip
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
