// ── Expression helper tests ──────────────────────────────────────────────────

#[test]
fn parse_helper_contains() {
    use crate::expression::ExpressionHelper;
    use crate::expression::parse_helper;
    assert_eq!(parse_helper("contains"), Some(ExpressionHelper::Contains));
}

#[test]
fn parse_helper_starts_with() {
    use crate::expression::ExpressionHelper;
    use crate::expression::parse_helper;
    assert_eq!(
        parse_helper("starts_with"),
        Some(ExpressionHelper::StartsWith)
    );
}

#[test]
fn parse_helper_ends_with() {
    use crate::expression::ExpressionHelper;
    use crate::expression::parse_helper;
    assert_eq!(parse_helper("ends_with"), Some(ExpressionHelper::EndsWith));
}

#[test]
fn parse_helper_has() {
    use crate::expression::ExpressionHelper;
    use crate::expression::parse_helper;
    assert_eq!(parse_helper("has"), Some(ExpressionHelper::Has));
}

#[test]
fn parse_helper_exists() {
    use crate::expression::ExpressionHelper;
    use crate::expression::parse_helper;
    assert_eq!(parse_helper("exists"), Some(ExpressionHelper::Exists));
}

#[test]
fn parse_helper_length() {
    use crate::expression::ExpressionHelper;
    use crate::expression::parse_helper;
    assert_eq!(parse_helper("length"), Some(ExpressionHelper::Length));
}

#[test]
fn parse_helper_empty() {
    use crate::expression::ExpressionHelper;
    use crate::expression::parse_helper;
    assert_eq!(parse_helper("empty"), Some(ExpressionHelper::Empty));
}

#[test]
fn parse_helper_append() {
    use crate::expression::ExpressionHelper;
    use crate::expression::parse_helper;
    assert_eq!(parse_helper("append"), Some(ExpressionHelper::Append));
}

#[test]
fn parse_helper_append_if() {
    use crate::expression::ExpressionHelper;
    use crate::expression::parse_helper;
    assert_eq!(parse_helper("append_if"), Some(ExpressionHelper::AppendIf));
}

#[test]
fn parse_helper_merge() {
    use crate::expression::ExpressionHelper;
    use crate::expression::parse_helper;
    assert_eq!(parse_helper("merge"), Some(ExpressionHelper::Merge));
}

#[test]
fn parse_helper_sum() {
    use crate::expression::ExpressionHelper;
    use crate::expression::parse_helper;
    assert_eq!(parse_helper("sum"), Some(ExpressionHelper::Sum));
}

#[test]
fn parse_helper_count() {
    use crate::expression::ExpressionHelper;
    use crate::expression::parse_helper;
    assert_eq!(parse_helper("count"), Some(ExpressionHelper::Count));
}

#[test]
fn parse_helper_unique() {
    use crate::expression::ExpressionHelper;
    use crate::expression::parse_helper;
    assert_eq!(parse_helper("unique"), Some(ExpressionHelper::Unique));
}

#[test]
fn parse_helper_unknown_returns_none() {
    use crate::expression::parse_helper;
    assert_eq!(parse_helper("unknown_helper"), None);
}

// ── ExpressionLiteral tests ─────────────────────────────────────────────────

#[test]
fn expression_literal_null() {
    use crate::expression::ExpressionLiteral;
    let lit = ExpressionLiteral::Null;
    assert!(matches!(lit, ExpressionLiteral::Null));
}

#[test]
fn expression_literal_bool_true() {
    use crate::expression::ExpressionLiteral;
    let lit = ExpressionLiteral::Bool(true);
    assert!(matches!(lit, ExpressionLiteral::Bool(true)));
}

#[test]
fn expression_literal_bool_false() {
    use crate::expression::ExpressionLiteral;
    let lit = ExpressionLiteral::Bool(false);
    assert!(matches!(lit, ExpressionLiteral::Bool(false)));
}

#[test]
fn expression_literal_i64() {
    use crate::expression::ExpressionLiteral;
    let lit = ExpressionLiteral::I64(42);
    assert!(matches!(lit, ExpressionLiteral::I64(42)));
}

#[test]
fn expression_literal_text() {
    use crate::expression::ExpressionLiteral;
    let lit = ExpressionLiteral::Text(Box::from("hello"));
    if let ExpressionLiteral::Text(s) = lit {
        assert_eq!(s.as_ref(), "hello");
    } else {
        panic!("expected text literal");
    }
}

// ── WorkflowDigest tests ────────────────────────────────────────────────────

#[test]
fn workflow_digest_from_bytes_creates_digest() {
    use vb_core::WorkflowDigest;
    let _digest = WorkflowDigest::from_bytes([0u8; 32]);
    // Just verify it can be created without panicking
}

// ── CompileError code() tests ────────────────────────────────────────────────

#[test]
fn compile_error_source_too_large_code() {
    let error = CompileError::SourceTooLarge {
        actual: 100,
        limit: 50,
    };
    assert_eq!(error.code().as_str(), "PAYLOAD_TOO_LARGE");
}

#[test]
fn compile_error_empty_source_code() {
    let error = CompileError::EmptySource;
    assert_eq!(error.code().as_str(), "MISSING_REQUIRED_FIELD");
}

#[test]
fn compile_error_unknown_reference_root_code() {
    let error = CompileError::UnknownReferenceRoot {
        reference: Box::from("$unknown.value"),
        root: Box::from("unknown"),
    };
    assert_eq!(error.code().as_str(), "UNKNOWN_REFERENCE");
}

// ── YamlLimits tests ────────────────────────────────────────────────────────

#[test]
fn yaml_limits_default() {
    use crate::YamlLimits;
    let limits = YamlLimits::default();
    assert_eq!(limits.max_source_bytes, 1_048_576);
    assert_eq!(limits.max_depth, 64);
    assert_eq!(limits.max_nodes, 100_000);
}

#[test]
fn yaml_limits_custom() {
    use crate::YamlLimits;
    let limits = YamlLimits {
        max_source_bytes: 100,
        max_depth: 10,
        max_nodes: 50,
        max_sequence_len: 20,
        max_mapping_entries: 15,
        max_scalar_bytes: 200,
    };
    assert_eq!(limits.max_source_bytes, 100);
    assert_eq!(limits.max_depth, 10);
}

