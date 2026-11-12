pub(super) fn contains_case_insensitive(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

pub(super) fn sentence_containing<'a>(text: &'a str, needle: &str) -> Option<&'a str> {
    text.split_inclusive(['.', '\n'])
        .map(str::trim)
        .find(|sentence| contains_case_insensitive(sentence, needle))
}
