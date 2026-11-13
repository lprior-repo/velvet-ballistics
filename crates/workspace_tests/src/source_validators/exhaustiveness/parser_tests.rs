use super::*;

#[test]
fn legacy_control_whitespace_does_not_hide_enum_variants() {
    for blank in ["\u{1c}", "\u{1d}", "\u{1e}", "\u{1f}"] {
        let body = format!("First,\n{blank}Second,\n");
        assert_eq!(
            extract_enum_variants(&body).expect("variants"),
            vec!["First".to_string(), "Second".to_string()],
            "blank={blank:?}"
        );
    }
}

#[test]
fn legacy_line_separators_do_not_hide_enum_variants() {
    for separator in [
        "\n", "\r", "\r\n", "\u{b}", "\u{c}", "\u{1c}", "\u{1d}", "\u{1e}", "\u{85}", "\u{2028}",
        "\u{2029}",
    ] {
        let body = format!("First,{separator}Second,");
        assert_eq!(
            extract_enum_variants(&body).expect("variants"),
            vec!["First".to_string(), "Second".to_string()],
            "separator={separator:?}"
        );
    }
}

#[test]
fn strip_line_comments() {
    let input = "let x = 1; // comment";
    let result = strip_comments(input).expect("strip ok");
    assert_eq!(result, "let x = 1; ");
}

#[test]
fn strip_block_comments() {
    let input = "let x = 1; /* block */ let y = 2;";
    let result = strip_comments(input).expect("strip ok");
    assert_eq!(result, "let x = 1;  let y = 2;");
}

#[test]
fn test_extract_enum_body() {
    let text = "pub enum TestError { VariantOne, VariantTwo }";
    let body = extract_enum_body(text, "TestError")
        .expect("extract ok")
        .expect("enum found");
    assert!(body.contains("VariantOne"));
    assert!(body.contains("VariantTwo"));
}

#[test]
fn extract_variants_from_body() {
    let body = "VariantOne,\n    VariantTwo,\n    VariantThree\n";
    let variants = extract_enum_variants(body).expect("extract ok");
    assert_eq!(variants.len(), 3);
    assert!(variants.contains(&"VariantOne".to_owned()));
    assert!(variants.contains(&"VariantTwo".to_owned()));
    assert!(variants.contains(&"VariantThree".to_owned()));
}

#[test]
fn find_mentioned_variants_in_body() {
    let body = "match err { Test::VarOne => {}, Test::VarTwo => {} }";
    let mentioned = find_mentioned_variants(body, "Test").expect("find ok");
    assert_eq!(mentioned.len(), 2);
    assert!(mentioned.contains(&"VarOne".to_owned()));
    assert!(mentioned.contains(&"VarTwo".to_owned()));
}

#[test]
fn graceful_wildcard_empty_braces() {
    assert!(has_graceful_wildcard("match x { _ => {} }").expect("wildcard check ok"));
}

#[test]
fn graceful_wildcard_ok_empty_tuple() {
    assert!(has_graceful_wildcard("match x { _ => Ok(()) }").expect("wildcard check ok"));
}

#[test]
fn graceful_wildcard_empty_tuple() {
    assert!(has_graceful_wildcard("match x { _ => () }").expect("wildcard check ok"));
}

#[test]
fn no_graceful_wildcard() {
    assert!(!has_graceful_wildcard("match x { A => {}, B => {} }").expect("wildcard check ok"));
}

#[test]
fn extract_enum_variants_with_data() {
    let body = "A,\n    B { x: u32, y: String },\n    C(u8),\n    D,\n";
    let variants = extract_enum_variants(body).expect("extract ok");
    assert_eq!(variants.len(), 4);
    assert!(variants.contains(&"A".to_owned()));
    assert!(variants.contains(&"B".to_owned()));
    assert!(variants.contains(&"C".to_owned()));
    assert!(variants.contains(&"D".to_owned()));
}

#[test]
fn attributed_struct_variant_fields_not_reported() {
    let body = "#[error(\"full\")]\nFull,\n#[error(\"size mismatch\")]\nSizeMismatch { expected: EventSeq, actual: RunId },\n#[error(\"version error\")]\nVersionError(u16),\n";
    let variants = extract_enum_variants(body).expect("extract ok");
    assert_eq!(variants.len(), 3);
    assert!(variants.contains(&"Full".to_owned()));
    assert!(variants.contains(&"SizeMismatch".to_owned()));
    assert!(variants.contains(&"VersionError".to_owned()));
    // Field types must not appear as variants
    assert!(!variants.contains(&"EventSeq".to_owned()));
    assert!(!variants.contains(&"RunId".to_owned()));
    assert!(!variants.contains(&"u16".to_owned()));
}

#[test]
fn nested_braces_in_enum_body() {
    let text = "pub enum TestError { VariantOne, VariantTwo { data: String } }";
    let body = extract_enum_body(text, "TestError")
        .expect("extract ok")
        .expect("enum found");
    // Should extract the entire enum body including nested braces
    assert!(body.contains("VariantOne"));
    assert!(body.contains("VariantTwo"));
    assert!(body.contains("data: String"));
}

#[test]
fn nested_braces_in_function_body() {
    let text = "fn foo() { if true { return; } else { panic!(); } }";
    let body = extract_function_body(text, "foo")
        .expect("extract ok")
        .expect("function found");
    // Should extract the entire function body including nested braces
    assert!(body.contains("if true"));
    assert!(body.contains("return;"));
    assert!(body.contains("panic!();"));
}

#[test]
fn missing_closing_brace_error() {
    let text = "pub enum TestError { VariantOne";
    let result = extract_enum_body(text, "TestError");
    // Truncated definitions must fail with an explicit unmatched-brace error
    assert!(result.is_err());
}

#[test]
fn no_opening_brace_error() {
    let text = "pub enum TestError VariantOne";
    let result = extract_enum_body(text, "TestError");
    assert!(result.is_ok());
    let body = result.expect("ok");
    assert!(body.is_none());
}

#[test]
fn generic_function_name() {
    let text = "fn process<T>(x: T) { let y = x; }";
    let body = extract_function_body(text, "process")
        .expect("extract ok")
        .expect("function found");
    assert!(body.contains("let y"));
}

#[test]
fn legacy_generic_spacing_rejected() {
    // Legacy contract parity: original Python checker rejects space
    // between generic closing > and opening (. Not a general Rust
    // grammar claim; this checker intentionally preserves that
    // original strict pattern.
    let text = "fn process<T> (x: T) { let y = x; }";
    let body = extract_function_body(text, "process").expect("extract ok");
    assert!(
        body.is_none(),
        "generic with space before paren must be rejected"
    );

    // Positive control: no-gap generic still extracts
    let text2 = "fn process<T>(x: T) { let y = x; }";
    let body2 = extract_function_body(text2, "process")
        .expect("extract ok")
        .expect("function found");
    assert_eq!(body2, " let y = x; ");
}

#[test]
fn similarly_prefixed_wrong_function_name() {
    let text = "fn process(x) { let y = x; }\nfn process_all(xs) { }";
    // Should find `process`, not `process_all`
    let body = extract_function_body(text, "process")
        .expect("extract ok")
        .expect("function found");
    assert!(body.contains("let y"));
    assert!(!body.contains("xs"));
}

#[test]
fn only_prefixed_name_when_requesting_base() {
    let text = "fn process_all(xs) { }";
    // Should not find `process` when only `process_all` exists
    let body = extract_function_body(text, "process").expect("extract ok");
    assert!(body.is_none());
}

#[test]
fn notfn_wrong_prefix_when_requesting_fn() {
    let text = "notfn foo() { }";
    // Should not find `foo` when prefix is `notfn`
    let body = extract_function_body(text, "foo").expect("extract ok");
    assert!(body.is_none());
}

#[test]
fn unicode_in_enum_body() {
    let text = "pub enum TestError { VariantOne /* café */ }";
    let body = extract_enum_body(text, "TestError")
        .expect("extract ok")
        .expect("enum found");
    assert!(body.contains("VariantOne"));
}

#[test]
fn unicode_in_function_body() {
    let text = "fn foo() { let msg = \"hello 🌍\"; }";
    let body = extract_function_body(text, "foo")
        .expect("extract ok")
        .expect("function found");
    assert!(body.contains("hello"));
}

#[test]
fn unicode_before_enum() {
    let text = "// café\npub enum TestError { VariantOne }";
    let body = extract_enum_body(text, "TestError")
        .expect("extract ok")
        .expect("enum found");
    assert!(body.contains("VariantOne"));
}

#[test]
fn brace_step_depth_overflow() {
    // Simulate depth at u32::MAX — opening brace should fail
    let result = brace_step(u32::MAX, (0, '{'));
    assert!(matches!(result, ControlFlow::Break(Err(_))));
}

#[test]
fn brace_step_depth_underflow() {
    // Simulate depth at 0 — closing brace should fail
    let result = brace_step(0, (0, '}'));
    assert!(matches!(result, ControlFlow::Break(Err(_))));
}

#[test]
fn brace_step_valid_depth_change() {
    // Opening brace increases depth
    let result = brace_step(5, (0, '{'));
    assert!(matches!(result, ControlFlow::Continue(6)));

    // Closing brace decreases depth
    let result = brace_step(6, (0, '}'));
    assert!(matches!(result, ControlFlow::Continue(5)));

    // Non-brace char keeps depth the same
    let result = brace_step(5, (0, 'a'));
    assert!(matches!(result, ControlFlow::Continue(5)));
}

#[test]
fn duplicate_enum_variant_names_deduplicated() {
    // Enum with duplicate variant names — set semantics should produce unique variants
    let body = "Full,\n    Full,\n    Disconnected,\n    Disconnected,\n    Timeout\n";
    let variants = extract_enum_variants(body).expect("extract ok");
    // Should deduplicate to 3 unique variants
    assert_eq!(variants.len(), 3);
    assert!(variants.contains(&"Full".to_owned()));
    assert!(variants.contains(&"Disconnected".to_owned()));
    assert!(variants.contains(&"Timeout".to_owned()));
}

#[test]
fn exact_nested_enum_body_extraction() {
    // Verify exact complete body extraction, not just contains
    let text = "pub enum TestError { VariantOne, VariantTwo { data: String } } trailing";
    let body = extract_enum_body(text, "TestError")
        .expect("extract ok")
        .expect("enum found");
    // Exact match — no truncation, no extra content
    assert_eq!(body, " VariantOne, VariantTwo { data: String } ");
}

#[test]
fn exact_nested_function_body_extraction() {
    // Verify exact complete function body with nested braces
    let text = "fn foo() { if true { return; } else { panic!(); } } trailing";
    let body = extract_function_body(text, "foo")
        .expect("extract ok")
        .expect("function found");
    // Exact match — all nested braces included, no truncation
    assert_eq!(body, " if true { return; } else { panic!(); } ");
}

#[test]
fn multiline_struct_field_variant_extraction() {
    // Genuinely multiline struct variant with uppercase field types
    let body = "#[error(\"size mismatch\")]\n\
SizeMismatch {\n\
expected: EventSeq,\n\
actual: RunId,\n\
},\n\
#[error(\"version\")]\n\
VersionError(u16),\n";
    let variants = extract_enum_variants(body).expect("extract ok");
    assert_eq!(variants.len(), 2);
    assert!(variants.contains(&"SizeMismatch".to_owned()));
    assert!(variants.contains(&"VersionError".to_owned()));
    // Field types must not appear as variants
    assert!(!variants.contains(&"EventSeq".to_owned()));
    assert!(!variants.contains(&"RunId".to_owned()));
    assert!(!variants.contains(&"u16".to_owned()));
}

#[test]
fn find_mentioned_variants_deduplicates_repeated_mentions() {
    // Repeated qualified mentions interleaved with a different variant
    let body = "match err { Test::VarOne => {}, Test::VarTwo => {}, Test::VarOne => {}, Test::VarThree => {}, Test::VarTwo => {} }";
    let mentioned = find_mentioned_variants(body, "Test").expect("find ok");
    // Set semantics: exactly 3 unique variants despite 5 total mentions
    assert_eq!(mentioned.len(), 3);
    assert!(mentioned.contains(&"VarOne".to_owned()));
    assert!(mentioned.contains(&"VarTwo".to_owned()));
    assert!(mentioned.contains(&"VarThree".to_owned()));
}
