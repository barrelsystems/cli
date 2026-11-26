/// Anticipated wrong top-level commands, mapped to the real one -- muscle
/// memory from similarly-named verbs in other common command-line tools,
/// not a general fuzzy match. Grow this as real mistakes surface.
const COMMAND_HINTS: &[(&str, &str)] = &[
    ("publish", "barrel release create"),
    ("push", "barrel release create"),
    ("create", "barrel release create"),
    ("register", "barrel release create"),
];

/// Looks up a hint for an unrecognized top-level command the user actually
/// typed. Returns `None` for anything not in the table -- this is a lookup
/// of specific, anticipated mistakes, not general intent inference.
#[must_use]
pub fn command_hint(invalid: &str) -> Option<&'static str> {
    COMMAND_HINTS
        .iter()
        .find(|(wrong, _)| *wrong == invalid)
        .map(|(_, right)| *right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_hint_recognizes_anticipated_wrong_commands() {
        assert_eq!(command_hint("publish"), Some("barrel release create"));
        assert_eq!(command_hint("push"), Some("barrel release create"));
    }

    #[test]
    fn command_hint_returns_none_for_anything_not_anticipated() {
        assert_eq!(command_hint("frobnicate"), None);
    }
}
