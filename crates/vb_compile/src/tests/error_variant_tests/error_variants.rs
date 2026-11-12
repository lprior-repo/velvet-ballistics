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
    let source = UNKNOWN_STEP_FIELD_SOURCE;
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
    let source = LAST_STEP_SAVE_SOURCE;
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

