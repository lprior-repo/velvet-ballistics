pub fn is_comment_line(line: &str) -> bool {
    line.trim_start_matches(|character: char| {
        character.is_whitespace() || matches!(character, '\u{001c}'..='\u{001f}')
    })
    .starts_with("//")
}

pub fn extract_citations(line: &str) -> Vec<(String, String)> {
    let mut citations = Vec::new();
    let mut consumed = 0;
    for (marker, _) in line.match_indices(".rs:") {
        if marker < consumed {
            continue;
        }
        let path_start = ascii_path_start(line, marker, consumed);
        if marker.saturating_sub(path_start) < 1 {
            continue;
        }
        let Some(spec_start) = marker.checked_add(4) else {
            continue;
        };
        let Some(first) = line.as_bytes().get(spec_start) else {
            continue;
        };
        if !first.is_ascii_digit() {
            continue;
        }
        let spec_end = line
            .as_bytes()
            .iter()
            .enumerate()
            .skip(spec_start)
            .take_while(|(_, byte)| byte.is_ascii_digit() || matches!(byte, b',' | b'-' | b' '))
            .map(|(index, _)| index)
            .last()
            .and_then(|last| last.checked_add(1));
        let Some(spec_end) = spec_end else {
            continue;
        };
        let path_end = marker.checked_add(3);
        let (Some(path_end), Some(path), Some(spec)) = (
            path_end,
            path_end.and_then(|end| line.get(path_start..end)),
            line.get(spec_start..spec_end),
        ) else {
            continue;
        };
        citations.push((path.to_string(), spec.to_string()));
        consumed = spec_end.max(path_end);
    }
    citations
}

fn ascii_path_start(line: &str, marker: usize, consumed: usize) -> usize {
    line.char_indices()
        .filter(|(index, _)| *index >= consumed && *index < marker)
        .rev()
        .take_while(|(_, character)| is_path_character(*character))
        .last()
        .map_or(marker, |(index, _)| index)
}

fn is_path_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '/' | '-')
}

#[cfg(test)]
mod tests {
    use super::{extract_citations, is_comment_line};

    #[test]
    fn comment_prefix_includes_python_extra_whitespace_only() {
        for character in ['\u{001c}', '\u{001d}', '\u{001e}', '\u{001f}'] {
            assert!(is_comment_line(&format!("{character}// cite")));
        }
        assert!(!is_comment_line("\u{200b}// cite"));
        assert!(!is_comment_line("code // cite"));
    }

    #[test]
    fn adjacent_matches_cannot_reuse_consumed_range_digits() {
        assert_eq!(
            extract_citations("// a.rs:1b.rs:1"),
            vec![
                ("a.rs".to_string(), "1".to_string()),
                ("b.rs".to_string(), "1".to_string())
            ]
        );
        assert_eq!(
            extract_citations("// a.rs:1 b.rs:1"),
            vec![
                ("a.rs".to_string(), "1 ".to_string()),
                ("b.rs".to_string(), "1".to_string())
            ]
        );
    }

    #[test]
    fn extracts_ascii_citation_with_literal_space_ranges() {
        assert_eq!(
            extract_citations("// See -_a.rs:1, 2-4"),
            vec![("-_a.rs".to_string(), "1, 2-4".to_string())]
        );
    }

    #[test]
    fn unicode_resets_path_candidate_without_empty_basename() {
        assert_eq!(
            extract_citations("// αfoo.rs:1 café.rs:2"),
            vec![("foo.rs".to_string(), "1 ".to_string())]
        );
    }

    #[test]
    fn only_ascii_digit_can_start_range_specification() {
        assert_eq!(
            extract_citations("// file.rs:٢ file.rs:2"),
            vec![("file.rs".to_string(), "2".to_string())]
        );
    }
}
