use super::{get_context, get_span, match_identifiers};

#[test]
fn test_single_line_context() -> Result<(), String> {
    let lines = vec!["pub fn Widget() {}".to_string()];
    let ctx = get_context(&lines, 1)?;
    assert_eq!(ctx, "pub fn Widget() {}");
    Ok(())
}

#[test]
fn test_first_line_window() -> Result<(), String> {
    let lines: Vec<String> = (1..=5).map(|i| format!("line {}", i)).collect();
    let ctx = get_context(&lines, 1)?;
    // Window for line 1 (0-based index 0): [0.saturating_sub(3)..min(5, 0+6)] = [0..5]
    assert_eq!(ctx, "line 1 line 2 line 3 line 4 line 5");
    Ok(())
}

#[test]
fn test_middle_line_window() -> Result<(), String> {
    let lines: Vec<String> = (1..=10).map(|i| format!("line {}", i)).collect();
    let ctx = get_context(&lines, 5)?;
    // Window for line 5 (0-based index 4): [4-3..min(10, 4+6)] = [1..10]
    assert_eq!(
        ctx,
        "line 2 line 3 line 4 line 5 line 6 line 7 line 8 line 9 line 10"
    );
    Ok(())
}

#[test]
fn test_last_line_window() -> Result<(), String> {
    let lines: Vec<String> = (1..=5).map(|i| format!("line {}", i)).collect();
    let ctx = get_context(&lines, 5)?;
    // Window for line 5 (0-based index 4): [4-3..min(5, 4+6)] = [1..5]
    assert_eq!(ctx, "line 2 line 3 line 4 line 5");
    Ok(())
}

#[test]
fn test_context_zero_line() {
    let lines = vec!["line 1".to_string()];
    let result = get_context(&lines, 0);
    assert!(result.is_err());
}

#[test]
fn test_context_outside_line() {
    let lines = vec!["line 1".to_string()];
    let result = get_context(&lines, 2);
    assert!(result.is_err());
}

#[test]
fn test_span_valid() -> Result<(), String> {
    let lines = vec![
        "line 1".to_string(),
        "line 2".to_string(),
        "line 3".to_string(),
    ];
    let span = get_span(&lines, 1, 2)?;
    assert_eq!(span, "line 1\nline 2");
    Ok(())
}

#[test]
fn test_span_last_line() -> Result<(), String> {
    let lines = vec!["line 1".to_string(), "line 2".to_string()];
    let span = get_span(&lines, 2, 2)?;
    assert_eq!(span, "line 2");
    Ok(())
}

#[test]
fn test_span_zero_start() {
    let lines = vec!["line 1".to_string()];
    let result = get_span(&lines, 0, 1);
    assert!(result.is_err());
}

#[test]
fn test_span_reversed() {
    let lines = vec!["line 1".to_string(), "line 2".to_string()];
    let result = get_span(&lines, 2, 1);
    assert!(result.is_err());
}

#[test]
fn test_span_outside() {
    let lines = vec!["line 1".to_string()];
    let result = get_span(&lines, 1, 3);
    assert!(result.is_err());
}

#[test]
fn test_span_overflow() {
    let lines = vec!["line 1".to_string()];
    let result = get_span(&lines, 1, u64::MAX);
    assert!(result.is_err());
}

#[test]
fn test_match_identifiers_basic() {
    let span = "pub fn Widget() {}";
    let ctx = "// Widget from crates/a.rs:1";
    assert!(match_identifiers(span, ctx, &[]));
}

#[test]
fn test_match_identifiers_case() {
    let span = "pub fn WIDGET() {}";
    let ctx = "// widget from crates/a.rs:1";
    assert!(match_identifiers(span, ctx, &[]));
}

#[test]
fn test_match_identifiers_stop() {
    let span = "pub fn types() {}";
    let ctx = "// types from crates/a.rs:1";
    assert!(!match_identifiers(span, ctx, &["types"]));
}

#[test]
fn test_match_identifiers_mirrored() {
    let span = "pub fn Foo() {}";
    let ctx = "// MirrorFoo from crates/a.rs:1";
    assert!(match_identifiers(span, ctx, &[]));
}

#[test]
fn test_match_identifiers_underscore_component() {
    // components() splits on _; Foo matches Foo_Bar via component expansion
    let span = "pub fn Foo() {}";
    let ctx = "// Foo_Bar from crates/a.rs:1";
    assert!(match_identifiers(span, ctx, &[]));
}

#[test]
fn test_match_identifiers_underscore_no_concatenation() {
    // components() never concatenates; FooBar does NOT match Foo_Bar
    let span = "pub fn FooBar() {}";
    let ctx = "// Foo_Bar from crates/a.rs:1";
    assert!(!match_identifiers(span, ctx, &[]));
}

#[test]
fn test_match_identifiers_numeric_prefix() {
    // IDENT_RE [A-Za-z_][A-Za-z0-9_]* scans for letter-start identifiers;
    // "123alpha" yields "alpha" (letter-start), no standalone alpha in context
    let span = "pub fn alpha() {}";
    let ctx = "// see 123alpha from crates/a.rs:1";
    assert!(match_identifiers(span, ctx, &[]));
}

#[test]
fn test_match_identifiers_numeric_prefix_mismatch() {
    // "123alpha" yields "alpha"; beta does not match
    let span = "pub fn beta() {}";
    let ctx = "// see 123alpha from crates/a.rs:1";
    assert!(!match_identifiers(span, ctx, &[]));
}

#[test]
fn test_match_identifiers_numeric_only_no_match() {
    // Numeric-only tokens are not identifiers; 123 vs 123 does not match
    let span = "123";
    let ctx = "123";
    assert!(!match_identifiers(span, ctx, &[]));
}

#[test]
fn test_match_identifiers_mirror_underscore_order() {
    // Mirror suffix stripped first, then components expanded:
    // MirrorFoo_Bar → Foo_Bar → {Foo, Bar}; Foo matches
    let span = "pub fn Foo() {}";
    let ctx = "// MirrorFoo_Bar from crates/a.rs:1";
    assert!(match_identifiers(span, ctx, &[]));
}

#[test]
fn test_match_identifiers_mirror_underscore_no_concatenation() {
    // Even after Mirror stripping, FooBar does not match Foo_Bar
    let span = "pub fn FooBar() {}";
    let ctx = "// MirrorFoo_Bar from crates/a.rs:1";
    assert!(!match_identifiers(span, ctx, &[]));
}

#[test]
fn test_match_identifiers_mirror_underscore_no_match() {
    // MirrorFoo_Bar expands to {Foo, Bar}; Baz matches neither
    let span = "pub fn Baz() {}";
    let ctx = "// MirrorFoo_Bar from crates/a.rs:1";
    assert!(!match_identifiers(span, ctx, &[]));
}

#[test]
fn test_match_identifiers_short_token_casefold() {
    // Short full token "X" matches Mirror-expanded "x" via casefold
    assert!(match_identifiers("X", "Mirrorx", &[]));
}

#[test]
fn test_match_identifiers_short_underscore_full_token() {
    // Full token "X_Y" matches Mirror-expanded "x_y" via casefold
    assert!(match_identifiers("X_Y", "Mirrorx_y", &[]));
}

#[test]
fn test_match_identifiers_stopped_span_component() {
    // "types" is STOP; span component "types" does not create a hit
    assert!(!match_identifiers("abc_types", "Mirrortypes", &["types"]));
}

#[test]
fn test_match_identifiers_positive_stop_full_token() {
    assert!(match_identifiers("types", "Mirrortypes", &["types"]));
}
