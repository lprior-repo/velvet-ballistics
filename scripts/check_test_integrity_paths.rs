// ---------------------------------------------------------------------------
// Path classification
// ---------------------------------------------------------------------------

pub(crate) fn is_test_path(path: &str) -> bool {
    path.ends_with(".rs")
        && (is_module_test_path(path)
            || [
                "/tests/",
                "/benches/",
                "/examples/",
                "/fuzz/",
                "workspace_tests",
            ]
            .iter()
            .any(|part| format!("/{path}").contains(part)))
}

pub(crate) fn is_behavior_test_path(path: &str) -> bool {
    path.ends_with(".rs")
        && (is_module_test_path(path)
            || ["/tests/", "workspace_tests"]
                .iter()
                .any(|part| format!("/{path}").contains(part)))
}

fn is_module_test_path(path: &str) -> bool {
    if !path.ends_with(".rs") {
        return false;
    }
    let Some(after_src) = path_after_src(path) else {
        return false;
    };
    is_src_tests_rs_path(after_src) || is_src_tests_child_path(after_src)
}

fn path_after_src(path: &str) -> Option<&str> {
    if let Some(after_src) = path.strip_prefix("src/") {
        return Some(after_src);
    }
    path.split_once("/src/")
        .map(|(_prefix, after_src)| after_src)
}

fn is_src_tests_rs_path(after_src: &str) -> bool {
    after_src == "tests.rs" || after_src.ends_with("/tests.rs")
}

fn is_src_tests_child_path(after_src: &str) -> bool {
    let child = if let Some(child) = after_src.strip_prefix("tests/") {
        child
    } else if let Some((_before_tests, child)) = after_src.rsplit_once("/tests/") {
        child
    } else {
        return false;
    };
    !child.is_empty() && child.ends_with(".rs") && !child.contains('/')
}

// ---------------------------------------------------------------------------
// Text signal detection
// ---------------------------------------------------------------------------

pub(crate) fn has_exact_assertion(text: &str) -> bool {
    [
        "assert_eq!(",
        "assert_ne!(",
        "assert_matches!(",
        "assert_json_",
        "insta::assert_",
        "snapshot!(",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

pub(crate) fn has_weak_assertion(text: &str) -> bool {
    text.contains("assert!(")
        && [
            ".is_ok(",
            ".is_err(",
            ".is_some(",
            ".is_none(",
            ".is_empty(",
        ]
        .iter()
        .any(|needle| text.contains(needle))
}

pub(crate) fn has_test_decl(text: &str) -> bool {
    text.contains("#[test")
        || text.contains("#[tokio::test")
        || text.contains("fn test_")
        || text.contains("_test(")
}

pub(crate) fn has_ignore_or_skip(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if lower.contains("#[ignore")
        || lower.contains("cfg_attr") && lower.contains("ignore")
        || lower.contains("return;")
    {
        return true;
    }
    let tokens = signal_tokens(&lower);
    let natural_comment = (lower.trim_start().starts_with("//")
        || lower.trim_start().starts_with("/*"))
        && !lower.contains('"')
        && !lower.contains('\'')
        && !lower.contains("*/");
    tokens.windows(2).any(|pair| {
        pair.first().map(String::as_str) == Some("return")
            && pair.get(1).map(String::as_str) == Some(";")
    }) || tokens.iter().enumerate().any(|(index, token)| {
        let previous = index.checked_sub(1).and_then(|i| tokens.get(i));
        matches!(token.as_str(), "skip" | "skipped")
            && previous.map(String::as_str) != Some("fn")
            && !method_dot(&tokens, index)
            && tokens.get(index.saturating_add(1)..).is_some_and(|tail| {
                let continued = tail.is_empty()
                    || tail.first().map(String::as_str) == Some("/")
                        && matches!(tail.get(1).map(String::as_str), Some("/" | "*"));
                skip_call(tail)
                    || continued
                        && !natural_comment
                        && !matches!(previous.map(String::as_str), Some("let" | "mut" | "ref"))
            })
    }) || tokens
        .split(|token| token == "#")
        .skip(1)
        .any(skip_attribute)
}

fn method_dot(tokens: &[String], index: usize) -> bool {
    let previous = index.checked_sub(1).and_then(|i| tokens.get(i));
    let before = index.checked_sub(2).and_then(|i| tokens.get(i));
    previous.map(String::as_str) == Some(".")
        && !matches!(before.map(String::as_str), Some("." | "<spaced-dot>"))
        && tokens.get(index.saturating_add(1)).map(String::as_str) != Some("!")
}

fn skip_call(tail: &[String]) -> bool {
    matches!(tail.first().map(String::as_str), Some("!" | "("))
        || tail.iter().take(2).map(String::as_str).eq([":", ":"])
}

fn skip_attribute(tokens: &[String]) -> bool {
    let mut words = tokens.iter().map(|word| word.trim_start_matches("r#"));
    let first = match words.next() {
        Some("!") => words.next(),
        first => first,
    };
    if first != Some("[") {
        return false;
    }
    match words.next() {
        Some("ignore" | "skip") => matches!(words.next(), Some("]" | "=" | "(")),
        Some("cfg_attr") => {
            let mut groups = Vec::new();
            let mut previous = "cfg_attr";
            words.take_while(|word| *word != "]").any(|word| {
                let marker = matches!(word, "ignore" | "skip")
                    && previous == ","
                    && groups.last().copied() == Some("cfg_attr");
                match word {
                    "(" => groups.push(previous),
                    ")" => groups.truncate(groups.len().saturating_sub(1)),
                    _ => {}
                }
                previous = word;
                marker
            })
        }
        _ => false,
    }
}

// Quotes and comments remain raw tokens so they cannot hide explicit markers.
fn signal_tokens(text: &str) -> Vec<String> {
    let mut chars = text.chars().peekable();
    std::iter::from_fn(|| {
        let ch = chars.next()?;
        let token = match ch {
            ch if ch.is_whitespace() => None,
            '.' if chars.peek().is_some_and(|ch| ch.is_whitespace()) => {
                Some("<spaced-dot>".to_owned())
            }
            ch if ch.is_alphanumeric() || ch == '_' => {
                let word: String = std::iter::once(ch)
                    .chain(std::iter::from_fn(|| {
                        chars.next_if(|ch| ch.is_alphanumeric() || *ch == '_')
                    }))
                    .collect();
                let mut tail = chars.clone();
                if word == "r"
                    && tail.next_if_eq(&'#').is_some()
                    && tail
                        .peek()
                        .is_some_and(|ch| ch.is_alphabetic() || *ch == '_')
                {
                    let name: String = std::iter::from_fn(|| {
                        tail.next_if(|ch| ch.is_alphanumeric() || *ch == '_')
                    })
                    .collect();
                    chars = tail;
                    Some(format!("r#{name}"))
                } else {
                    Some(word)
                }
            }
            ch => Some(ch.to_string()),
        };
        Some(token)
    })
    .flatten()
    .collect()
}

pub(crate) fn has_compile_only(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "no_run",
        "compile_only",
        "compile-only",
        "smoke only",
        "compile smoke",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}
