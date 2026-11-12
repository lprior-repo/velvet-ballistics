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
    lower.contains("#[ignore")
        || lower.contains("cfg_attr") && lower.contains("ignore")
        || lower.contains("return;")
        || lower.contains(" skipped")
        || lower.contains(" skip")
        || lower.contains("ignored")
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
