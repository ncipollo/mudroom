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
fn parse_look_variants() {
    assert!(matches!(parse("l"), Command::Look(None)));
    assert!(matches!(parse("L"), Command::Look(None)));
    assert!(matches!(parse("look"), Command::Look(None)));
    assert!(matches!(parse("Look"), Command::Look(None)));
}

#[test]
fn parse_look_at_variants() {
    for input in ["look at sword", "look sword", "l sword"] {
        if let Command::Look(Some(target)) = parse(input) {
            assert_eq!(target, "sword");
        } else {
            panic!("expected Look(Some(...)) for {input}");
        }
    }
}

#[test]
fn parse_help_variants() {
    assert!(matches!(parse("help"), Command::Help));
    assert!(matches!(parse("Help"), Command::Help));
}

#[test]
fn parse_attack_variant() {
    assert!(matches!(parse("attack"), Command::Attack));
    assert!(matches!(parse("Attack"), Command::Attack));
}

#[test]
fn parse_speak_variants() {
    assert!(matches!(parse("speak"), Command::Speak(None)));
    assert!(matches!(parse("Speak"), Command::Speak(None)));
    assert!(matches!(parse("talk"), Command::Speak(None)));
    assert!(matches!(parse("Talk"), Command::Speak(None)));
    assert!(matches!(parse("say"), Command::Speak(None)));
    assert!(matches!(parse("Say"), Command::Speak(None)));
}

#[test]
fn parse_speak_with_message() {
    for (input, expected) in [
        ("speak Hello there!", "Hello there!"),
        ("talk Hi", "Hi"),
        ("say Yo", "Yo"),
    ] {
        if let Command::Speak(Some(msg)) = parse(input) {
            assert_eq!(msg, expected);
        } else {
            panic!("expected Speak(Some(...)) for {input}");
        }
    }
}

#[test]
fn parse_take() {
    if let Command::Take(target) = parse("take sword") {
        assert_eq!(target, "sword");
    } else {
        panic!("expected Take(...)");
    }
}

#[test]
fn parse_take_preserves_target_case() {
    for (input, expected) in [("take BAT", "BAT"), ("Take Spiked Bat", "Spiked Bat")] {
        if let Command::Take(target) = parse(input) {
            assert_eq!(target, expected);
        } else {
            panic!("expected Take(...) for {input}");
        }
    }
}

#[test]
fn parse_removed_single_letter_shortcuts() {
    assert!(matches!(parse("t"), Command::Unknown));
    assert!(matches!(parse("h"), Command::Unknown));
    assert!(matches!(parse("a"), Command::Unknown));
}

#[test]
fn parse_unknown() {
    assert!(matches!(parse("foo"), Command::Unknown));
    assert!(matches!(parse(""), Command::Unknown));
}

#[test]
fn parse_interact_explicit_verb() {
    if let Command::Interact { verb, target } = parse("interact chest") {
        assert_eq!(verb, "interact");
        assert_eq!(target, "chest");
    } else {
        panic!("expected Interact(...)");
    }
}

#[test]
fn parse_interact_alt_verb() {
    if let Command::Interact { verb, target } = parse("open chest") {
        assert_eq!(verb, "open");
        assert_eq!(target, "chest");
    } else {
        panic!("expected Interact(...)");
    }
}

#[test]
fn parse_interact_lowercases_verb_but_preserves_target_case() {
    if let Command::Interact { verb, target } = parse("Open Oak Chest") {
        assert_eq!(verb, "open");
        assert_eq!(target, "Oak Chest");
    } else {
        panic!("expected Interact(...)");
    }
}

#[test]
fn parse_interact_is_the_fallback_for_any_unrecognized_verb_and_target() {
    if let Command::Interact { verb, target } = parse("go north") {
        assert_eq!(verb, "go");
        assert_eq!(target, "north");
    } else {
        panic!("expected Interact(...)");
    }
}
