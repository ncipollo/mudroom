/// Builds the player-facing message for a `take all from <feature>` attempt.
pub(super) fn take_all_summary(
    feature_name: &str,
    taken: &[String],
    bag_filled_up: bool,
) -> String {
    if taken.is_empty() {
        return "Your bag is full.".to_string();
    }

    let list = join_with_and(taken);
    let mut message = format!("You take {list} from the {feature_name}.");
    if bag_filled_up {
        message.push_str(" Your bag is full, so you leave the rest.");
    }
    message
}

/// Joins item names into a natural-language, Oxford-comma list: `"the X"`, `"the X and the
/// Y"`, `"the X, the Y, and the Z"`.
fn join_with_and(names: &[String]) -> String {
    let phrased: Vec<String> = names.iter().map(|n| format!("the {n}")).collect();
    match phrased.as_slice() {
        [] => String::new(),
        [only] => only.clone(),
        [a, b] => format!("{a} and {b}"),
        _ => {
            let (last, rest) = phrased.split_last().unwrap();
            format!("{}, and {last}", rest.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_with_and_formats_a_single_item() {
        assert_eq!(join_with_and(&["Torch".to_string()]), "the Torch");
    }

    #[test]
    fn join_with_and_formats_a_pair() {
        assert_eq!(
            join_with_and(&["Torch".to_string(), "Key".to_string()]),
            "the Torch and the Key"
        );
    }

    #[test]
    fn join_with_and_formats_three_or_more_with_oxford_comma() {
        assert_eq!(
            join_with_and(&["Torch".to_string(), "Key".to_string(), "Rope".to_string()]),
            "the Torch, the Key, and the Rope"
        );
    }

    #[test]
    fn take_all_summary_when_nothing_was_taken() {
        assert_eq!(
            take_all_summary("Oak Chest", &[], false),
            "Your bag is full."
        );
    }

    #[test]
    fn take_all_summary_for_full_success() {
        assert_eq!(
            take_all_summary("Oak Chest", &["Torch".to_string()], false),
            "You take the Torch from the Oak Chest."
        );
    }

    #[test]
    fn take_all_summary_notes_the_bag_filled_up_mid_take() {
        assert_eq!(
            take_all_summary("Oak Chest", &["Torch".to_string()], true),
            "You take the Torch from the Oak Chest. Your bag is full, so you leave the rest."
        );
    }
}
