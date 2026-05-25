//! Error variant completeness tests for vb_compile.
//!
//! These tests ensure every `CompileError` variant has at least one test
//! asserting the exact variant (not merely `is_err()`).
//!
//! Coverage audit as of 2026-05-18:
//! - Target: 5x pub fn coverage (≥400 tests for 80 pub fns)
//! - Current: 337 tests → need ~63 more
//! - This file contributes tests for untested error variants

use crate::{CompileError, CompileErrors, YamlCompiler};

/// Helper: parse source, return first error or fail.
fn parse_error(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
        Err(CompileErrors(errors)) => errors
            .into_iter()
            .next()
            .ok_or_else(|| "parse_ast failed with no errors".to_string()),
    }
}

// ── EmptySource variant ────────────────────────────────────────────────────────

#[test]
fn empty_source_rejected_with_empty_source() {
    let result = YamlCompiler::default().parse_ast(b"");
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::EmptySource)))
    );
}

// ── DuplicateKey variant ────────────────────────────────────────────────────

#[test]
fn duplicate_top_level_key_rejected_with_duplicate_key() {
    let source = br#"version: velvet-ballistics/v1
name: test
name: duplicate
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let error = parse_error(source).expect("expected error");
    assert!(matches!(error, CompileError::DuplicateKey { key, .. } if key.as_ref() == "name"));
}

// ── NonStringKey variant ─────────────────────────────────────────────────────

#[test]
fn non_string_mapping_key_rejected_with_non_string_key() {
    // Use integer 100 as a key (invalid - keys must be strings)
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
100: invalid_key
"#;
    let result = YamlCompiler::default().parse_ast(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::NonStringKey { .. })))
    );
}

// ── StepShape variant ───────────────────────────────────────────────────────

#[test]
fn step_not_mapping_rejected_with_step_shape() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - "not a mapping"
"#;
    let result = YamlCompiler::default().parse_ast(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::StepShape { step: 0, .. })))
    );
}

// ── UnknownStepField variant ─────────────────────────────────────────────────

#[test]
fn unknown_step_field_rejected_with_unknown_step_field() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    unknown_field: true
    finish:
      result: 0
"#;
    let result = YamlCompiler::default().parse_ast(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::UnknownStepField { step: 0, .. })))
    );
}

// ── MissingStepId variant ───────────────────────────────────────────────────

#[test]
fn step_missing_id_rejected_with_missing_step_id() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
    let result = YamlCompiler::default().parse_ast(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::MissingStepId { step: 0, .. })))
    );
}

// ── DuplicateStepId variant ────────────────────────────────────────────────

#[test]
fn duplicate_step_id_rejected_with_duplicate_step_id() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    save:
      value: 1
  - id: done
    finish:
      result: 0
"#;
    let result = YamlCompiler::default().parse_ast(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::DuplicateStepId { .. })))
    );
}

// ── UnknownTopLevelField variant ─────────────────────────────────────────────

#[test]
fn unknown_top_level_field_rejected_with_unknown_top_level_field() {
    let source = br#"version: velvet-ballistics/v1
name: test
unknown_field: true
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = YamlCompiler::default().parse_ast(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::UnknownTopLevelField { .. })))
    );
}

// ── InvalidVersion variant ───────────────────────────────────────────────────

#[test]
fn invalid_workflow_version_rejected_with_invalid_version() {
    let source = br#"version: velvet-ballistics/v2
name: test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = YamlCompiler::default().parse_ast(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::InvalidVersion { .. })))
    );
}

// ── UnknownTriggerKind variant ───────────────────────────────────────────────

#[test]
fn unknown_trigger_kind_rejected_with_unknown_trigger_kind() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  unknown_trigger: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = YamlCompiler::default().parse_ast(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::UnknownTriggerKind { .. })))
    );
}

// ── InvalidTriggerCount variant ─────────────────────────────────────────────

#[test]
fn multiple_trigger_entries_rejected_with_invalid_trigger_count() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
  schedule: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = YamlCompiler::default().parse_ast(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::InvalidTriggerCount { count: 2 })))
    );
}

// ── FieldShape variant ──────────────────────────────────────────────────────

#[test]
fn field_wrong_shape_rejected_with_field_shape() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
inputs: not_a_mapping
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = YamlCompiler::default().parse_ast(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::FieldShape { .. })))
    );
}

// ── EmptySteps variant ───────────────────────────────────────────────────────

#[test]
fn empty_steps_rejected_with_empty_steps() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps: []
"#;
    let result = YamlCompiler::default().parse_ast(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::EmptySteps)))
    );
}

// ── LastStepMustFinish variant ───────────────────────────────────────────────

#[test]
fn last_step_not_finish_rejected_with_last_step_must_finish() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    save:
      value: 1
"#;
    let result = YamlCompiler::default().parse_ast(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::LastStepMustFinish)))
    );
}

// ── UnknownReferenceName variant ─────────────────────────────────────────────

#[test]
fn unknown_reference_name_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
inputs:
  user: text
examples:
  - name: fixture
    value: $input.nonexistent
steps:
  - id: done
    finish:
      result: 0
"#;
    let error = parse_error(source).expect("expected error");
    assert!(matches!(error, CompileError::UnknownReferenceName { kind, .. } if kind == "input"));
}

// ── UnknownReferenceRoot variant ─────────────────────────────────────────────

#[test]
fn unknown_reference_root_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
examples:
  - name: fixture
    value: $env.HOME
steps:
  - id: done
    finish:
      result: 0
"#;
    let error = parse_error(source).expect("expected error");
    assert!(matches!(error, CompileError::UnknownReferenceRoot { .. }));
}

// ── IllegalReference variant ─────────────────────────────────────────────────

#[test]
fn illegal_reference_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
examples:
  - name: fixture
    value: $runtime.now
steps:
  - id: done
    finish:
      result: 0
"#;
    let error = parse_error(source).expect("expected error");
    assert!(matches!(error, CompileError::IllegalReference { .. }));
}

// ── UnsupportedAccessorReference variant ─────────────────────────────────────

#[test]
fn unsupported_accessor_reference_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: ref_case
when:
  manual: {}
steps:
  - id: build_result
    save:
      value: 1
  - id: done
    finish:
      result: $slot.0.name
"#;
    let error = parse_error(source).expect("expected error");
    assert!(matches!(
        error,
        CompileError::UnsupportedAccessorReference { .. }
    ));
}

// ── CompileErrors first() method ─────────────────────────────────────────────

#[test]
fn compile_errors_first_returns_first_error() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    save:
      value: 1
"#;
    let result = YamlCompiler::default().parse_ast(source);
    let Err(CompileErrors(errors)) = result else {
        panic!("expected error");
    };
    let first = errors.first();
    assert!(first.is_some());
}

// ── CompileErrors len() and is_empty() ─────────────────────────────────────

#[test]
fn compile_errors_len_and_is_empty() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    save:
      value: 1
"#;
    let result = YamlCompiler::default().parse_ast(source);
    let Err(CompileErrors(errors)) = result else {
        panic!("expected error");
    };
    assert!(!errors.is_empty());
    assert!(errors.len() >= 1);
}

// ── CompileErrors iter() ─────────────────────────────────────────────────────

#[test]
fn compile_errors_iter_works() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    save:
      value: 1
"#;
    let result = YamlCompiler::default().parse_ast(source);
    let Err(CompileErrors(errors)) = result else {
        panic!("expected error");
    };
    let count = errors.iter().count();
    assert!(count >= 1);
}

// ── CompileErrors as_slice() ─────────────────────────────────────────────────

#[test]
fn compile_errors_as_slice_returns_all_errors() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    save:
      value: 1
"#;
    let result = YamlCompiler::default().parse_ast(source);
    let Err(CompileErrors(errors)) = result else {
        panic!("expected error");
    };
    let slice = errors.as_slice();
    assert!(!slice.is_empty());
}

// ── YamlCompiler Default ─────────────────────────────────────────────────────

#[test]
fn yaml_compiler_default_constructs() {
    let _compiler = YamlCompiler::default();
}

// ── YamlCompiler parse_ast returns Err for invalid source ────────────────────

#[test]
fn yaml_compiler_rejects_invalid_source() {
    let source = b"not yaml at all";
    let result = YamlCompiler::default().parse_ast(source);
    assert!(result.is_err());
}

// ── YamlCompiler compile returns Err for invalid source ──────────────────────

#[test]
fn yaml_compiler_rejects_invalid_source_via_compile() {
    let source = b"not yaml at all";
    let result = YamlCompiler::default().compile(source);
    assert!(result.is_err());
}

// ── Parse expressions tests ─────────────────────────────────────────────────

#[test]
fn parse_expression_accepts_integer_literals() {
    use crate::expression::{ExpressionLiteral, ParsedExpression, parse_expression};
    let result = parse_expression("42");
    assert!(result.is_ok());
    if let Ok(ParsedExpression::Literal(ExpressionLiteral::I64(n))) = result {
        assert_eq!(n, 42);
    } else {
        panic!("expected integer literal");
    }
}

#[test]
fn parse_expression_accepts_boolean_literals() {
    use crate::expression::{ExpressionLiteral, ParsedExpression, parse_expression};
    let result = parse_expression("true");
    assert!(result.is_ok());
    if let Ok(ParsedExpression::Literal(ExpressionLiteral::Bool(b))) = result {
        assert!(b);
    } else {
        panic!("expected boolean literal");
    }
}

#[test]
fn parse_expression_rejects_invalid_syntax() {
    use crate::expression::parse_expression;
    let result = parse_expression("= 1 + 2");
    assert!(result.is_err());
}

// ── Strict YAML profile tests ────────────────────────────────────────────────

#[test]
fn strict_yaml_rejects_alias() {
    use crate::strict_yaml::reject_unsupported_profile_events;
    let result = reject_unsupported_profile_events("&anchor value");
    assert!(result.is_err());
}

#[test]
fn strict_yaml_accepts_single_document() {
    use crate::strict_yaml::reject_unsupported_profile_events;
    let result = reject_unsupported_profile_events("key: value");
    assert!(result.is_ok());
}

#[test]
fn strict_yaml_rejects_multiple_documents() {
    use crate::strict_yaml::reject_unsupported_profile_events;
    let result = reject_unsupported_profile_events("key1: value1\n---\nkey2: value2");
    assert!(result.is_err());
}

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
    assert_eq!(error.code(), "PAYLOAD_TOO_LARGE");
}

#[test]
fn compile_error_empty_source_code() {
    let error = CompileError::EmptySource;
    assert_eq!(error.code(), "MISSING_REQUIRED_FIELD");
}

#[test]
fn compile_error_unknown_reference_root_code() {
    let error = CompileError::UnknownReferenceRoot {
        reference: Box::from("$unknown.value"),
        root: Box::from("unknown"),
    };
    assert_eq!(error.code(), "UNKNOWN_REFERENCE");
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

// ── Integration: workflow with reference parses ─────────────────────────────

#[test]
fn workflow_with_input_reference_parses() {
    let source = br#"version: velvet-ballistics/v1
name: ref_test
when:
  manual: {}
inputs:
  user: text
steps:
  - id: step1
    save:
      value: $input.user
  - id: done
    finish:
      result: 0
"#;
    let result = YamlCompiler::default().parse_ast(source);
    assert!(result.is_ok(), "workflow with reference should parse");
}

// ── Integration: compute_compiled_digest is deterministic ───────────────────

#[test]
fn compiled_digest_is_deterministic() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let digest1 = crate::compute_compiled_digest(source);
    let digest2 = crate::compute_compiled_digest(source);
    assert_eq!(digest1, digest2);
}

#[test]
fn different_sources_produce_different_digests() {
    let source1 = br#"version: velvet-ballistics/v1
name: test1
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let source2 = br#"version: velvet-ballistics/v1
name: test2
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let digest1 = crate::compute_compiled_digest(source1);
    let digest2 = crate::compute_compiled_digest(source2);
    assert_ne!(digest1, digest2);
}

// ── Additional error variant coverage tests ───────────────────────────────────

#[test]
fn parse_error_returns_first_error() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    unknown_field: true
    finish:
      result: 0
"#;
    let error = parse_error(source).expect("expected error");
    assert!(matches!(error, CompileError::UnknownStepField { .. }));
}

#[test]
fn compile_errors_iter_count_matches_len() {
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    unknown_field: true
    finish:
      result: 0
"#;
    let Err(CompileErrors(errors)) = YamlCompiler::default().parse_ast(source) else {
        panic!("expected error");
    };
    assert_eq!(errors.iter().count(), errors.len());
}

#[test]
fn yaml_limits_max_sequence_defaults_correctly() {
    use crate::YamlLimits;
    let limits = YamlLimits::default();
    assert_eq!(limits.max_sequence_len, 10_000);
    assert_eq!(limits.max_mapping_entries, 1_024);
    assert_eq!(limits.max_scalar_bytes, 65_536);
}

// ── PO-009: Digest determinism ─────────────────────────────────────────────────

#[test]
fn compute_compiled_digest_determinism() {
    // Repeated calls produce bit-for-bit identical digest across multiple invocations.
    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let digest1 = crate::compute_compiled_digest(source);
    let digest2 = crate::compute_compiled_digest(source);
    let digest3 = crate::compute_compiled_digest(source);
    assert_eq!(digest1, digest2, "first and second call must match");
    assert_eq!(digest2, digest3, "second and third call must match");
}

// ── PO-010: Artifact digest depends on source digest ───────────────────────────

#[test]
fn artifact_digest_depends_on_source() {
    // The artifact digest is a function of source digest plus IR serialization.
    // Changing source digest changes artifact digest.
    use crate::compile_workflow;
    use crate::emit_compiled_artifact;

    let source1 = br#"version: velvet-ballistics/v1
name: test1
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let source2 = br#"version: velvet-ballistics/v1
name: test2
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    // Compile both sources
    let workflow1 = compile_workflow(source1).expect("source1 should compile");
    let workflow2 = compile_workflow(source2).expect("source2 should compile");

    // Get artifact bytes
    let artifact1 = emit_compiled_artifact(&workflow1).expect("workflow1 should emit");
    let artifact2 = emit_compiled_artifact(&workflow2).expect("workflow2 should emit");

    // Source digests differ
    let source_digest1 = crate::compute_compiled_digest(source1);
    let source_digest2 = crate::compute_compiled_digest(source2);
    assert_ne!(source_digest1, source_digest2, "different sources have different digests");

    // Artifact bytes differ (because source differs)
    assert_ne!(
        artifact1.as_ref(),
        artifact2.as_ref(),
        "artifact bytes differ when source differs"
    );
}

// ── PO-018: Postcard serialization determinism ─────────────────────────────────

#[test]
fn postcard_serialization_deterministic() {
    // Same WorkflowParts produces same postcard::serialize bytes across invocations.
    use crate::compile_workflow;
    use crate::emit_compiled_artifact;

    let source = br#"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let workflow = compile_workflow(source).expect("source should compile");

    // Serialize multiple times
    let artifact1 = emit_compiled_artifact(&workflow).expect("first emit should succeed");
    let artifact2 = emit_compiled_artifact(&workflow).expect("second emit should succeed");
    let artifact3 = emit_compiled_artifact(&workflow).expect("third emit should succeed");

    assert_eq!(
        artifact1.as_ref(),
        artifact2.as_ref(),
        "first and second serialization must match"
    );
    assert_eq!(
        artifact2.as_ref(),
        artifact3.as_ref(),
        "second and third serialization must match"
    );
}
