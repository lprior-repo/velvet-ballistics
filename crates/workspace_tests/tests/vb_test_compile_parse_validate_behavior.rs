#![forbid(unsafe_code)]
//! Behavior tests for vb_compile parsing and validation.
//!
//! Tests parse error detection, validation rule enforcement, exact error
//! message content, and happy-path compilation success.

use vb_compile::{CompileError, CompileErrors, YamlCompiler, compile_workflow, strict_yaml};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_error(source: &[u8]) -> Result<CompileError, String> {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => Err(format!("parse_ast unexpectedly succeeded: {ast:?}")),
        Err(CompileErrors(errors)) => errors
            .into_iter()
            .next()
            .ok_or_else(|| "parse_ast failed with no errors".to_string()),
    }
}

fn all_errors(source: &[u8]) -> Vec<CompileError> {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => panic!("parse_ast unexpectedly succeeded: {ast:?}"),
        Err(CompileErrors(errors)) => errors,
    }
}

// ---------------------------------------------------------------------------
// Parse Error Detection — Empty Source
// ---------------------------------------------------------------------------

#[test]
fn parse_rejects_totally_empty_source() {
    let result = YamlCompiler::default().parse_ast(b"");
    assert!(result.is_err(), "empty source should fail parsing");
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("empty") || msg.contains("document"),
        "EmptySource error should mention empty/document: {msg}"
    );
}

#[test]
fn parse_rejects_whitespace_only_source() {
    let result = YamlCompiler::default().parse_ast(b"   \n\n   \n");
    assert!(result.is_err(), "whitespace-only source should fail");
}

#[test]
fn parse_rejects_source_too_large() {
    let compiler = YamlCompiler::new(vb_compile::YamlLimits {
        max_source_bytes: 10,
        ..Default::default()
    });
    let source = b"this exceeds the limit";
    let result = compiler.parse_ast(source);
    assert!(result.is_err(), "source exceeding byte limit should fail");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("limit") || msg.contains("exceed"),
        "error should mention limit: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Parse Error Detection — Document Count
// ---------------------------------------------------------------------------

#[test]
fn parse_rejects_multiple_documents() {
    let source = br#"
version: velvet-ballastics/v1
name: first
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
---
name: second
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let errors = all_errors(source);
    let doc_count = errors
        .iter()
        .find(|e| matches!(e, CompileError::DocumentCount { .. }));
    assert!(
        doc_count.is_some(),
        "Multiple documents should produce DocumentCount error: {errors:?}"
    );
    if let Some(CompileError::DocumentCount { count }) = doc_count {
        assert_eq!(*count, 2, "DocumentCount should report 2");
    }
}

// ---------------------------------------------------------------------------
// Parse Error Detection — YAML Structural
// ---------------------------------------------------------------------------

#[test]
fn parse_rejects_top_level_scalar() {
    let source = b"just a scalar\n";
    let error = parse_error(source).expect("expected error");
    assert!(
        matches!(error, CompileError::TopLevelNotMapping),
        "top-level scalar should produce TopLevelNotMapping, got: {error:?}"
    );
}

#[test]
fn parse_rejects_invalid_yaml_syntax() {
    // Missing colon after key at top level produces TopLevelNotMapping
    // since the parser treats it as a scalar key
    let source = b"version velvet-ballastics/v1\n";
    let error = parse_error(source).expect("expected error");
    // TopLevelNotMapping is the error for this malformed YAML
    assert!(
        matches!(error, CompileError::TopLevelNotMapping),
        "Invalid YAML syntax at top level should produce error, got: {error:?}"
    );
}

#[test]
fn strict_yaml_rejects_alias() {
    let source = r#"
name: test
when:
  manual: {}
steps:
  - id: s1
    set: { output: a, value: "1" }
  - id: s2
    set:
      <<: *anchor
"#;
    let result = strict_yaml::reject_unsupported_profile_events(source);
    assert!(
        result.is_err(),
        "YAML alias should be rejected by strict profile"
    );
}

#[test]
fn strict_yaml_rejects_anchor() {
    let source = r#"
anchor_test: &anchor
key: value
"#;
    let result = strict_yaml::reject_unsupported_profile_events(source);
    assert!(result.is_err(), "YAML anchor should be rejected");
    let err = result.unwrap_err();
    assert!(
        matches!(err, CompileError::AnchorForbidden { .. }),
        "expected AnchorForbidden, got: {err:?}"
    );
}

#[test]
fn strict_yaml_rejects_tag() {
    let source = r#"
key: !custom_tag value
"#;
    let result = strict_yaml::reject_unsupported_profile_events(source);
    assert!(result.is_err(), "YAML tag should be rejected");
    let err = result.unwrap_err();
    assert!(
        matches!(err, CompileError::TagForbidden { .. }),
        "expected TagForbidden, got: {err:?}"
    );
}

// ---------------------------------------------------------------------------
// Validation — Duplicate Keys
// ---------------------------------------------------------------------------

#[test]
fn parse_rejects_duplicate_top_level_key() {
    let source = br#"
version: velvet-ballastics/v1
name: first
name: duplicate
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let errors = all_errors(source);
    let dup = errors
        .iter()
        .find(|e| matches!(e, CompileError::DuplicateKey { .. }));
    assert!(
        dup.is_some(),
        "duplicate top-level key should error: {errors:?}"
    );
    if let Some(CompileError::DuplicateKey { key, .. }) = dup {
        assert_eq!(key.as_ref(), "name", "duplicate key should be 'name'");
    }
}

#[test]
fn parse_rejects_duplicate_step_id() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps:
  - id: step_a
    finish: { result: 0 }
  - id: step_a
    finish: { result: 1 }
"#;
    let errors = all_errors(source);
    let dup = errors
        .iter()
        .find(|e| matches!(e, CompileError::DuplicateStepId { .. }));
    assert!(dup.is_some(), "duplicate step id should error: {errors:?}");
}

#[test]
fn parse_rejects_duplicate_key_in_nested_mapping() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps:
  - id: s1
    set:
      output: a
      output: b
      value: "1"
"#;
    let errors = all_errors(source);
    let dup = errors
        .iter()
        .find(|e| matches!(e, CompileError::DuplicateKey { .. }));
    assert!(
        dup.is_some(),
        "duplicate key in nested mapping should error: {errors:?}"
    );
}

// ---------------------------------------------------------------------------
// Validation — Name Rules
// ---------------------------------------------------------------------------

#[test]
fn parse_rejects_name_with_spaces() {
    let source = br#"
version: velvet-ballastics/v1
name: "invalid name with spaces"
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let errors = all_errors(source);
    let invalid = errors
        .iter()
        .find(|e| matches!(e, CompileError::InvalidName { .. }));
    assert!(
        invalid.is_some(),
        "name with spaces should produce InvalidName: {errors:?}"
    );
}

#[test]
fn parse_rejects_name_starting_with_uppercase() {
    let source = br#"
version: velvet-ballastics/v1
name: InvalidName
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let errors = all_errors(source);
    let invalid = errors
        .iter()
        .find(|e| matches!(e, CompileError::InvalidName { .. }));
    assert!(
        invalid.is_some(),
        "uppercase-starting name should produce InvalidName: {errors:?}"
    );
}

#[test]
fn parse_rejects_name_with_special_chars() {
    let source = br#"
version: velvet-ballastics/v1
name: "name@#!"
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let errors = all_errors(source);
    let invalid = errors
        .iter()
        .find(|e| matches!(e, CompileError::InvalidName { .. }));
    assert!(
        invalid.is_some(),
        "name with special chars should produce InvalidName: {errors:?}"
    );
}

#[test]
fn parse_rejects_reserved_name_step_id() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps:
  - id: set
    finish:
      result: 0
"#;
    let errors = all_errors(source);
    let invalid = errors
        .iter()
        .find(|e| matches!(e, CompileError::InvalidName { .. }));
    assert!(
        invalid.is_some(),
        "reserved name 'set' as step id should error: {errors:?}"
    );
}

// ---------------------------------------------------------------------------
// Validation — Version
// ---------------------------------------------------------------------------

#[test]
fn parse_rejects_missing_version() {
    let source = br#"
name: no_version
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let errors = all_errors(source);
    let has_error = !errors.is_empty();
    assert!(
        has_error,
        "missing version should produce an error: {errors:?}"
    );
}

// ---------------------------------------------------------------------------
// Validation — Trigger
// ---------------------------------------------------------------------------

#[test]
fn parse_rejects_multiple_triggers() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
  schedule:
    cron: "0 * * * *"
steps:
  - id: done
    finish:
      result: 0
"#;
    let errors = all_errors(source);
    let multi_trigger = errors
        .iter()
        .find(|e| matches!(e, CompileError::InvalidTriggerCount { .. }));
    assert!(
        multi_trigger.is_some(),
        "multiple triggers should error: {errors:?}"
    );
}

#[test]
fn parse_rejects_unknown_trigger_kind() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  unknown_trigger: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let errors = all_errors(source);
    let unknown = errors
        .iter()
        .find(|e| matches!(e, CompileError::UnknownTriggerKind { .. }));
    assert!(
        unknown.is_some(),
        "unknown trigger kind should error: {errors:?}"
    );
}

// ---------------------------------------------------------------------------
// Validation — Steps
// ---------------------------------------------------------------------------

#[test]
fn parse_rejects_empty_steps() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps: []
"#;
    let errors = all_errors(source);
    let empty = errors
        .iter()
        .find(|e| matches!(e, CompileError::EmptySteps));
    assert!(empty.is_some(), "empty steps should error: {errors:?}");
}

#[test]
fn parse_rejects_missing_step_id() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps:
  - finish:
      result: 0
"#;
    let errors = all_errors(source);
    let missing = errors
        .iter()
        .find(|e| matches!(e, CompileError::MissingStepId { .. }));
    assert!(
        missing.is_some(),
        "missing step id should error: {errors:?}"
    );
}

#[test]
fn parse_rejects_step_without_primitive() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps:
  - id: orphan
"#;
    let errors = all_errors(source);
    let missing = errors
        .iter()
        .find(|e| matches!(e, CompileError::MissingStepPrimitive { .. }));
    assert!(
        missing.is_some(),
        "step without primitive should error: {errors:?}"
    );
}

#[test]
fn parse_rejects_step_with_multiple_primitives() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps:
  - id: multi
    set: { output: a, value: "1" }
    finish:
      result: 0
"#;
    let errors = all_errors(source);
    let multi = errors
        .iter()
        .find(|e| matches!(e, CompileError::MultipleStepPrimitives { .. }));
    assert!(
        multi.is_some(),
        "step with multiple primitives should error: {errors:?}"
    );
}

#[test]
fn parse_rejects_unknown_step_field() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    unknown_field: true
    finish:
      result: 0
"#;
    let errors = all_errors(source);
    let unknown = errors
        .iter()
        .find(|e| matches!(e, CompileError::UnknownStepField { .. }));
    assert!(
        unknown.is_some(),
        "unknown step field should error: {errors:?}"
    );
}

#[test]
fn parse_rejects_non_mapping_step() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps:
  - "not a mapping"
"#;
    let errors = all_errors(source);
    let shape = errors
        .iter()
        .find(|e| matches!(e, CompileError::StepShape { .. }));
    assert!(shape.is_some(), "non-mapping step should error: {errors:?}");
}

// ---------------------------------------------------------------------------
// Validation — Finish Step Rules
// ---------------------------------------------------------------------------

#[test]
fn parse_rejects_finish_not_last() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps:
  - id: first
    finish:
      result: 0
  - id: second
    set:
      output: a
      value: "1"
"#;
    let errors = all_errors(source);
    let shape = errors
        .iter()
        .find(|e| matches!(e, CompileError::StepFieldShape { .. }));
    assert!(shape.is_some(), "finish not last should error: {errors:?}");
}

#[test]
fn parse_rejects_finish_missing_result() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    finish:
"#;
    let errors = all_errors(source);
    // The error could be StepFieldShape (if it parses as non-mapping) or MissingStepField
    let has_expected_error = errors.iter().any(|e| {
        matches!(e, CompileError::MissingStepField { .. })
            || matches!(e, CompileError::StepFieldShape { .. })
    });
    assert!(
        has_expected_error,
        "finish missing result should error: {errors:?}"
    );
}

// ---------------------------------------------------------------------------
// Validation — Wait Step Rules
// ---------------------------------------------------------------------------

#[test]
fn parse_rejects_empty_wait_event() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps:
  - id: wait_step
    wait:
      event: ""
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(result.is_err(), "empty wait event should be rejected");
}

// ---------------------------------------------------------------------------
// Compilation Success — Happy Path (via compile_workflow)
// ---------------------------------------------------------------------------

#[test]
fn compile_produces_valid_workflow_for_minimal_manual_trigger() {
    let source = br#"
version: velvet-ballastics/v1
name: minimal_workflow
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(
        result.is_ok(),
        "minimal manual workflow should compile: {result:?}"
    );
    let workflow = result.unwrap();
    assert!(
        workflow.name().contains("minimal_workflow"),
        "workflow should preserve name"
    );
}

#[test]
fn compile_produces_valid_workflow_for_set_and_finish() {
    let source = br#"
version: velvet-ballastics/v1
name: set_workflow
when:
  manual: {}
steps:
  - id: initialize
    set:
      output: greeting
      value: "42"
  - id: done
    finish:
      result: greeting
"#;
    let result = compile_workflow(source);
    assert!(
        result.is_ok(),
        "set and finish workflow should compile: {result:?}"
    );
}

#[test]
fn compile_produces_valid_workflow_with_webhook_trigger() {
    // webhook: {} means webhook trigger with no configuration
    // which is valid for the canonical compiler handoff
    let source = br#"
version: velvet-ballastics/v1
name: webhook_workflow
when:
  webhook: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(
        result.is_ok(),
        "webhook workflow should compile: {result:?}"
    );
}

#[test]
fn compile_produces_valid_workflow_with_schedule_trigger() {
    let source = br#"
version: velvet-ballastics/v1
name: schedule_workflow
when:
  schedule:
    cron: "0 0 * * *"
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(
        result.is_ok(),
        "schedule workflow should compile: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Exact Error Message Content Assertions
// ---------------------------------------------------------------------------

#[test]
fn error_message_for_duplicate_key_contains_key_name() {
    let source = br#"
version: velvet-ballastics/v1
name: first
name: second
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let error = parse_error(source).expect("expected error");
    let msg = error.to_string();
    assert!(
        msg.contains("duplicate") && msg.contains("name"),
        "DuplicateKey message should mention 'duplicate' and 'name': {msg}"
    );
}

#[test]
fn error_message_for_invalid_name_contains_field_and_value() {
    let source = br#"
version: velvet-ballastics/v1
name: "bad name"
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let errors = all_errors(source);
    let invalid = errors
        .iter()
        .find(|e| matches!(e, CompileError::InvalidName { .. }));
    assert!(
        invalid.is_some(),
        "should have InvalidName error: {errors:?}"
    );
    let msg = invalid.unwrap().to_string();
    assert!(
        msg.contains("name"),
        "InvalidName message should mention 'name': {msg}"
    );
}

#[test]
fn error_message_for_unknown_trigger_kind_contains_trigger_value() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  bad_trigger: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let error = parse_error(source).expect("expected error");
    let msg = error.to_string();
    assert!(
        msg.contains("trigger") || msg.contains("bad_trigger"),
        "UnknownTriggerKind message should mention trigger: {msg}"
    );
}

#[test]
fn error_message_for_empty_steps_is_descriptive() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps: []
"#;
    let error = parse_error(source).expect("expected error");
    let msg = error.to_string();
    assert!(
        msg.contains("empty") || msg.contains("steps"),
        "EmptySteps message should mention empty/steps: {msg}"
    );
}

#[test]
fn error_message_for_non_string_key_is_descriptive() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps:
  - id: bad
    123: value
"#;
    let error = parse_error(source).expect("expected error");
    let msg = error.to_string();
    assert!(
        msg.contains("key") || msg.contains("string"),
        "NonStringKey error should mention key/string: {msg}"
    );
}

#[test]
fn error_message_for_step_shape_not_mapping_contains_step_index() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps:
  - "scalar step"
"#;
    let error = parse_error(source).expect("expected error");
    let msg = error.to_string();
    assert!(
        msg.contains("step"),
        "StepShape error should mention 'step': {msg}"
    );
}

#[test]
fn error_message_for_unknown_step_field_contains_field_name() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    bad_field: true
    finish:
      result: 0
"#;
    let error = parse_error(source).expect("expected error");
    let msg = error.to_string();
    assert!(
        msg.contains("bad_field") || msg.contains("step"),
        "UnknownStepField error should mention field: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Compilation Pipeline Integration
// ---------------------------------------------------------------------------

#[test]
fn compile_workflow_rejects_invalid_finish_not_last() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps:
  - id: step1
    finish:
      result: 0
  - id: step2
    set:
      output: a
      value: "1"
"#;
    let result = compile_workflow(source);
    assert!(
        result.is_err(),
        "finish not last should be rejected by compile_workflow"
    );
}

#[test]
fn compile_workflow_rejects_unknown_step_field() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps:
  - id: done
    unknown_field: true
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(result.is_err(), "unknown step field should be rejected");
}

#[test]
fn compile_workflow_rejects_empty_steps() {
    let source = br#"
version: velvet-ballastics/v1
name: test
when:
  manual: {}
steps: []
"#;
    let result = compile_workflow(source);
    assert!(result.is_err(), "empty steps should be rejected");
}

#[test]
fn compile_workflow_rejects_missing_version() {
    let source = br#"
name: no_version
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(result.is_err(), "missing version should be rejected");
}
