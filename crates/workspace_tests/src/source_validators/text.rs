pub(crate) fn legacy_whitespace(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '\u{1c}' | '\u{1d}' | '\u{1e}' | '\u{1f}')
}
