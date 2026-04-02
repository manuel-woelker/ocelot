/// Normalizes observed and expected output before comparison.
pub fn normalize_validation_text(text: &str) -> String {
    text.replace("\r\n", "\n").trim_end_matches('\n').to_owned()
}

#[cfg(test)]
mod tests {
    use super::normalize_validation_text;

    #[test]
    fn normalizes_line_endings_and_trailing_newlines() {
        assert_eq!(
            normalize_validation_text("first\r\nsecond\r\n"),
            "first\nsecond"
        );
    }
}
