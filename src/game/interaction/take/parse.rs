/// Parsed shape of a `take` command's target string.
pub(super) enum TakeTarget<'a> {
    Named(&'a str),
    FromFeature { item: &'a str, feature: &'a str },
    AllFromFeature { feature: &'a str },
}

const FROM_SEPARATOR: &str = " from ";

/// Splits `take <item> from <feature>` / `take all from <feature>` grammar out of the
/// opaque target string sent by the client, falling back to `Named` (today's plain
/// `take <item>` behavior) whenever the grammar doesn't apply. Case-insensitive on both the
/// `from` keyword and the `all` keyword.
///
/// Uses `to_ascii_lowercase` rather than `to_lowercase` to find the separator: it only
/// rewrites bytes in the ASCII range in place, so byte offsets found on the lowercased copy
/// stay valid on the original string — full Unicode lowercasing can change byte length.
pub(super) fn parse_take_target(target: &str) -> TakeTarget<'_> {
    let trimmed = target.trim();
    let Some(sep_idx) = trimmed.to_ascii_lowercase().find(FROM_SEPARATOR) else {
        return TakeTarget::Named(trimmed);
    };

    let item = trimmed[..sep_idx].trim();
    let feature = trimmed[sep_idx + FROM_SEPARATOR.len()..].trim();
    if feature.is_empty() || item.is_empty() {
        return TakeTarget::Named(trimmed);
    }

    if item.eq_ignore_ascii_case("all") {
        TakeTarget::AllFromFeature { feature }
    } else {
        TakeTarget::FromFeature { item, feature }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_named(target: &str, expected: &str) {
        match parse_take_target(target) {
            TakeTarget::Named(name) => assert_eq!(name, expected),
            _ => panic!("expected Named(\"{expected}\") for {target:?}"),
        }
    }

    #[test]
    fn parses_plain_target_as_named() {
        assert_named("torch", "torch");
    }

    #[test]
    fn parses_item_from_feature_with_mixed_case_separator() {
        match parse_take_target("Torch FROM Oak Chest") {
            TakeTarget::FromFeature { item, feature } => {
                assert_eq!(item, "Torch");
                assert_eq!(feature, "Oak Chest");
            }
            _ => panic!("expected FromFeature"),
        }
    }

    #[test]
    fn parses_all_from_feature_case_insensitively() {
        match parse_take_target("ALL from Oak Chest") {
            TakeTarget::AllFromFeature { feature } => assert_eq!(feature, "Oak Chest"),
            _ => panic!("expected AllFromFeature"),
        }
    }

    #[test]
    fn treats_all_without_from_as_a_literal_item_name() {
        assert_named("all", "all");
    }

    #[test]
    fn falls_back_to_named_when_feature_part_is_empty() {
        assert_named("torch from", "torch from");
    }

    #[test]
    fn falls_back_to_named_when_item_part_is_empty() {
        assert_named("from oak chest", "from oak chest");
    }
}
