use crate::game::Direction;

pub enum Command {
    Move(Direction),
    Look(Option<String>),
    Help,
    Speak(Option<String>),
    Choose(String),
    Attack,
    Inventory,
    #[allow(dead_code)]
    Enter(String),
    Take(String),
    Interact {
        verb: String,
        target: String,
    },
    Unknown,
}

pub fn parse(input: &str) -> Command {
    let trimmed = input.trim();
    let lower = trimmed.to_lowercase();

    match lower.as_str() {
        "n" | "north" => Command::Move(Direction::North),
        "s" | "south" => Command::Move(Direction::South),
        "e" | "east" => Command::Move(Direction::East),
        "w" | "west" => Command::Move(Direction::West),
        "l" | "look" => Command::Look(None),
        "help" => Command::Help,
        "attack" => Command::Attack,
        "speak" | "talk" | "say" => Command::Speak(None),
        "i" | "inventory" => Command::Inventory,
        _ => parse_prefixed(trimmed, &lower),
    }
}

fn parse_prefixed(trimmed: &str, lower: &str) -> Command {
    if let Some(cmd) = parse_look_target(trimmed, lower) {
        cmd
    } else if let Some(cmd) = parse_speak_target(trimmed, lower) {
        cmd
    } else if let Some(target) = strip_arg(trimmed, lower, "take ") {
        Command::Take(target)
    } else if lower.chars().all(|c| c.is_ascii_digit()) && !lower.is_empty() {
        Command::Choose(lower.to_string())
    } else if let Some(target) = lower.strip_prefix("enter ") {
        Command::Enter(target.to_string())
    } else if let Some(cmd) = parse_interact(trimmed, lower) {
        cmd
    } else {
        Command::Unknown
    }
}

/// Falls back to a generic `<verb> <target>` reading for anything not already claimed by a
/// dedicated command above. The server, not this parser, knows which verbs a room feature
/// declares, so it decides whether `verb` is `interact` or a valid alternate verb.
fn parse_interact(trimmed: &str, lower: &str) -> Option<Command> {
    let space_idx = lower.find(' ')?;
    let verb = &lower[..space_idx];
    let target = trimmed[space_idx + 1..].trim();
    if verb.is_empty() || target.is_empty() {
        return None;
    }
    Some(Command::Interact {
        verb: verb.to_string(),
        target: target.to_string(),
    })
}

fn parse_look_target(trimmed: &str, lower: &str) -> Option<Command> {
    ["look at ", "look ", "l "]
        .into_iter()
        .find_map(|prefix| strip_arg(trimmed, lower, prefix))
        .map(|target| Command::Look(Some(target)))
}

fn parse_speak_target(trimmed: &str, lower: &str) -> Option<Command> {
    ["speak ", "talk ", "say "]
        .into_iter()
        .find_map(|prefix| strip_arg(trimmed, lower, prefix))
        .map(|target| Command::Speak(Some(target)))
}

fn strip_arg(trimmed: &str, lower: &str, prefix: &str) -> Option<String> {
    lower
        .starts_with(prefix)
        .then(|| trimmed[prefix.len()..].trim().to_string())
}

#[cfg(test)]
mod tests;
