use crate::game::Direction;

pub enum Command {
    Move(Direction),
    Look,
    Help,
    Talk,
    Choose(String),
    #[allow(dead_code)]
    Enter(String),
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
        "l" | "look" => Command::Look,
        "h" | "help" => Command::Help,
        "talk" => Command::Talk,
        _ => {
            if lower.chars().all(|c| c.is_ascii_digit()) && !lower.is_empty() {
                Command::Choose(lower)
            } else if let Some(target) = lower.strip_prefix("enter ") {
                Command::Enter(target.to_string())
            } else {
                Command::Unknown
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_north_variants() {
        assert!(matches!(parse("n"), Command::Move(Direction::North)));
        assert!(matches!(parse("N"), Command::Move(Direction::North)));
        assert!(matches!(parse("north"), Command::Move(Direction::North)));
        assert!(matches!(parse("North"), Command::Move(Direction::North)));
    }

    #[test]
    fn parse_south_variants() {
        assert!(matches!(parse("s"), Command::Move(Direction::South)));
        assert!(matches!(parse("south"), Command::Move(Direction::South)));
    }

    #[test]
    fn parse_east_variants() {
        assert!(matches!(parse("e"), Command::Move(Direction::East)));
        assert!(matches!(parse("east"), Command::Move(Direction::East)));
    }

    #[test]
    fn parse_west_variants() {
        assert!(matches!(parse("w"), Command::Move(Direction::West)));
        assert!(matches!(parse("west"), Command::Move(Direction::West)));
    }

    #[test]
    fn parse_enter() {
        assert!(matches!(parse("enter tavern"), Command::Enter(_)));
        if let Command::Enter(target) = parse("enter tavern") {
            assert_eq!(target, "tavern");
        }
    }

    #[test]
    fn parse_help_variants() {
        assert!(matches!(parse("h"), Command::Help));
        assert!(matches!(parse("H"), Command::Help));
        assert!(matches!(parse("help"), Command::Help));
        assert!(matches!(parse("Help"), Command::Help));
    }

    #[test]
    fn parse_unknown() {
        assert!(matches!(parse("foo"), Command::Unknown));
        assert!(matches!(parse(""), Command::Unknown));
        assert!(matches!(parse("go north"), Command::Unknown));
    }
}
