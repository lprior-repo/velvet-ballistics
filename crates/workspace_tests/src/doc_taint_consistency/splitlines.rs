//! Python-compatible str.splitlines() line breaking.
//!
//! Matches Python's universal-newline behavior: LF, CR, CRLF as one boundary,
//! plus VT (\u{b}), FF (\u{c}), FS (\u{1c}), GS (\u{1d}), RS (\u{1e}),
//! NEL (\u{85}), LS (\u{2028}), PS (\u{2029}). Empty input yields zero lines;
//! a trailing boundary adds no spurious line.

fn is_linebreak(ch: char) -> bool {
    matches!(
        ch,
        '\n' | '\r'
            | '\u{b}'
            | '\u{c}'
            | '\u{1c}'
            | '\u{1d}'
            | '\u{1e}'
            | '\u{85}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

pub(crate) fn python_splitlines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    let pieces = text.split_inclusive(|ch: char| is_linebreak(ch));

    let result = pieces.fold((Vec::new(), false), |(mut lines, pending_cr), piece| {
        if pending_cr && piece == "\n" {
            return (lines, false);
        }

        let line = piece.trim_end_matches(|ch: char| is_linebreak(ch));
        lines.push(line.to_string());

        (lines, piece.ends_with('\r'))
    });

    result.0
}

#[cfg(test)]
mod tests {
    use super::python_splitlines;

    #[test]
    fn empty_input_has_no_lines() {
        assert_eq!(python_splitlines(""), Vec::<String>::new());
    }

    #[test]
    fn single_line_no_break() {
        assert_eq!(python_splitlines("hello"), vec!["hello"]);
    }

    #[test]
    fn lf_breaks_into_two_lines() {
        assert_eq!(python_splitlines("a\nb"), vec!["a", "b"]);
    }

    #[test]
    fn crlf_is_one_boundary() {
        assert_eq!(python_splitlines("a\r\nb"), vec!["a", "b"]);
    }

    #[test]
    fn lone_cr_is_a_boundary() {
        assert_eq!(python_splitlines("a\rb"), vec!["a", "b"]);
    }

    #[test]
    fn trailing_newline_adds_no_spurious_line() {
        assert_eq!(python_splitlines("a\n"), vec!["a"]);
    }

    #[test]
    fn multiple_trailing_newlines_add_no_spurious_lines() {
        assert_eq!(python_splitlines("a\n\n"), vec!["a", ""]);
    }

    #[test]
    fn vt_is_a_boundary() {
        assert_eq!(python_splitlines("a\u{b}b"), vec!["a", "b"]);
    }

    #[test]
    fn ff_is_a_boundary() {
        assert_eq!(python_splitlines("a\u{c}b"), vec!["a", "b"]);
    }

    #[test]
    fn file_separator_is_a_boundary() {
        assert_eq!(python_splitlines("a\u{1c}b"), vec!["a", "b"]);
    }

    #[test]
    fn unicode_newline_boundaries() {
        assert_eq!(python_splitlines("a\u{2028}b"), vec!["a", "b"]);
        assert_eq!(python_splitlines("a\u{2029}b"), vec!["a", "b"]);
        assert_eq!(python_splitlines("a\u{85}b"), vec!["a", "b"]);
    }

    #[test]
    fn preserves_line_content() {
        let lines = python_splitlines("line one\nline two\nline three");
        assert_eq!(lines, vec!["line one", "line two", "line three"]);
    }

    #[test]
    fn consecutive_boundaries_produce_empty_lines() {
        assert_eq!(python_splitlines("a\n\n\nb"), vec!["a", "", "", "b"]);
    }

    #[test]
    fn mixed_crlf_and_lf() {
        assert_eq!(python_splitlines("a\r\nb\nc"), vec!["a", "b", "c"]);
    }
}
