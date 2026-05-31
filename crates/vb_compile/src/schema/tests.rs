use crate::schema::validate_input_schemas;
use crate::{CompileError, CompileErrors, YamlCompiler};
use saphyr::{LoadableYamlNode, Yaml};
use vb_core::SymbolicCode;

fn validate_inputs(inputs: &str) -> Result<(), CompileError> {
    let source = format!("version: velvet-ballistics/v1\ninputs:\n{inputs}\n");
    let docs = Yaml::load_from_str(&source)?;
    let Some(doc) = docs.first() else {
        return Err(CompileError::EmptySource);
    };
    match validate_input_schemas(doc) {
        Ok(()) => Ok(()),
        Err(errors) => match errors.first() {
            Some(error) => Err(error.clone()),
            None => Err(CompileError::EmptySource),
        },
    }
}

#[test]
fn input_schema_rejects_unknown_fields() {
    let result = validate_inputs("  value:\n    is: text\n    kind: text\n");

    assert!(matches!(
        result,
        Err(CompileError::UnknownInputSchemaField { .. })
    ));
}

#[test]
fn input_schema_rejects_invalid_bounds() {
    let result = validate_inputs("  value:\n    is: text\n    min_length: 9\n    max_length: 1\n");

    assert!(matches!(
        result,
        Err(CompileError::InvalidInputSchema { .. })
    ));
}

// ---------------------------------------------------------------------------
// vb-yd5x RED PHASE: Shared IR parity tests
// ---------------------------------------------------------------------------

/// Minimal canonical workflow for testing.
const VB_YD5X_MINIMAL_VALID_WORKFLOW: &[u8] = br#"
version: velvet-ballistics/v1
name: minimal_valid
when:
  manual: {}
steps:
  - id: start
    set:
      output: answer
      value: "1"
  - id: done
    finish:
      result: answer
"#;

/// Workflow with out-of-range slot reference (Gate 9)
/// This uses a slot index that is out of bounds for the compiled workflow.
/// The issue is the result slot 99 doesn't exist.
const VB_YD5X_MALFORMED_SLOT_REF: &[u8] = br#"
version: velvet-ballistics/v1
name: bad_slot_ref
when:
  manual: {}
steps:
  - id: start
    save:
      value: 1
  - id: use_missing_slot
    for_each:
      input: 99
      item: 1
      limit: 10
  - id: done
    finish:
      result: 0
"#;

/// Workflow with loop body type mismatch (Gate 11)
/// The for_each 'input' field expects expression string but gets a number.
const VB_YD5X_MALFORMED_LOOP_BODY: &[u8] = br#"
version: velvet-ballistics/v1
name: bad_loop_body
when:
  manual: {}
steps:
  - id: fanout
    for_each:
      variable: i
      input: 123
      steps:
        - id: step
          finish:
            result: 0
  - id: join
    finish:
      result: 0
"#;

/// Workflow with duplicate step ID
const VB_YD5X_MALFORMED_DUPLICATE_ID: &[u8] = br#"
version: velvet-ballistics/v1
name: duplicate_ids
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: build
    finish:
      result: 0
"#;

/// Workflow with unknown reference
const VB_YD5X_MALFORMED_UNKNOWN_REF: &[u8] = br#"
version: velvet-ballistics/v1
name: unknown_ref
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: $input.missing == true
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: true
"#;

/// Helper: validate via canonical compile then shared pipeline.
fn vb_yd5x_validate_via_compile(source: &[u8]) -> Result<(), CompileErrors> {
    let compiled = YamlCompiler::default().compile(source)?;
    let parts = compiled.to_parts();
    vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))
}

fn first_compile_code(source: &[u8]) -> Result<SymbolicCode, String> {
    match YamlCompiler::default().compile(source) {
        Ok(workflow) => Err(format!("compile unexpectedly succeeded: {workflow:?}")),
        Err(errors) => errors
            .first()
            .map(CompileError::code)
            .ok_or_else(|| "compile failed with no errors".to_owned()),
    }
}

#[test]
fn vb_yd5x_valid_workflow_passes_both_paths() {
    let source = VB_YD5X_MINIMAL_VALID_WORKFLOW;
    let compile_result = YamlCompiler::default().compile(source);
    let validate_result = vb_yd5x_validate_via_compile(source);
    assert!(
        compile_result.is_ok(),
        "valid workflow must compile: {compile_result:?}"
    );
    assert!(
        validate_result.is_ok(),
        "valid workflow must pass shared validation: {validate_result:?}"
    );
}

#[test]
fn vb_yd5x_legacy_slot_ref_shape_fails_canonical_compile() -> Result<(), String> {
    assert_eq!(
        first_compile_code(VB_YD5X_MALFORMED_SLOT_REF)?.as_str(),
        "MISSING_REQUIRED_FIELD"
    );
    Ok(())
}

#[test]
fn vb_yd5x_legacy_loop_body_shape_fails_canonical_compile() -> Result<(), String> {
    assert_eq!(
        first_compile_code(VB_YD5X_MALFORMED_LOOP_BODY)?.as_str(),
        "TYPE_MISMATCH"
    );
    Ok(())
}

#[test]
fn vb_yd5x_legacy_duplicate_id_shape_fails_canonical_compile() -> Result<(), String> {
    assert_eq!(
        first_compile_code(VB_YD5X_MALFORMED_DUPLICATE_ID)?.as_str(),
        "MISSING_REQUIRED_FIELD"
    );
    Ok(())
}

#[test]
fn vb_yd5x_legacy_unknown_ref_shape_fails_canonical_compile() -> Result<(), String> {
    assert_eq!(
        first_compile_code(VB_YD5X_MALFORMED_UNKNOWN_REF)?.as_str(),
        "UNKNOWN_TOP_LEVEL_FIELD"
    );
    Ok(())
}

#[test]
fn vb_yd5x_legacy_diagnostic_codes_remain_stable() -> Result<(), String> {
    let test_cases = [
        (VB_YD5X_MALFORMED_SLOT_REF, "MISSING_REQUIRED_FIELD"),
        (VB_YD5X_MALFORMED_LOOP_BODY, "TYPE_MISMATCH"),
        (VB_YD5X_MALFORMED_DUPLICATE_ID, "MISSING_REQUIRED_FIELD"),
        (VB_YD5X_MALFORMED_UNKNOWN_REF, "UNKNOWN_TOP_LEVEL_FIELD"),
    ];
    for (source, expected_code) in test_cases {
        assert_eq!(first_compile_code(source)?.as_str(), expected_code);
    }
    Ok(())
}
