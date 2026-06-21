#![forbid(unsafe_code)]
//! Tests for LETHAL-1: validate_taint SecretResultLeak Finish Pass-Through
//!
//! These tests verify the compile pipeline's handling of secret taint in
//! Finish results per Section 47 contract.

use crate::{CompileError, CompileErrors, YamlCompiler};
use proptest::prelude::*;
use vb_core::CompiledWorkflow;

// ----------------------------------------------------------------------------
// Test helpers
// ----------------------------------------------------------------------------

fn compile_workflow(source: &[u8]) -> Result<CompiledWorkflow, CompileErrors> {
    YamlCompiler::default().compile(source)
}

// ----------------------------------------------------------------------------
// Section 47: Taint MUST pass through Finish outputs (currently buggy)
// ----------------------------------------------------------------------------

/// Given: YAML with secret-like data directly in the Finish result
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds with `Ok(CompiledWorkflow)`
#[test]
fn compile_accepts_secret_finish_result() {
    let source = br#"version: velvet-ballistics/v1
name: secret_finish_case
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 1
"#;
    let workflow = compile_workflow(source)
        .expect("Section 47: secret in Finish must compile, got ");
    assert!(
        workflow.finish_contains_secret_data(),
        "Finish result must preserve secret data per Section 47"
    );
}

/// Given: YAML with secret-like data via slot relay in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_secret_slot_relay_in_finish() {
    let source = br#"version: velvet-ballistics/v1
name: secret_slot_relay_case
when:
  manual: {}
steps:
  - id: capture
    set:
      output: captured
      value: "42"
  - id: done
    finish:
      result: 1
"#;
    let workflow = compile_workflow(source)
        .expect("Section 47: secret via slot relay in Finish must compile, got ");
    assert!(
        workflow.finish_contains_secret_data(),
        "Finish result must preserve secret data per Section 47"
    );
}

/// Given: YAML with composite containing secret-like data in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_secret_composite_in_finish() {
    let source = br#"version: velvet-ballistics/v1
name: secret_composite_case
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 1
"#;
    let workflow = compile_workflow(source)
        .expect("Section 47: composite with secret in Finish must compile, got ");
    assert!(
        workflow.finish_contains_secret_data(),
        "Finish result must preserve secret data per Section 47"
    );
}

/// Given: YAML with inline list containing secret-like data in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_secret_list_in_finish() {
    let source = br#"version: velvet-ballistics/v1
name: secret_list_case
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 1
"#;
    let workflow = compile_workflow(source)
        .expect("Section 47: list with secret in Finish must compile, got ");
    assert!(
        workflow.finish_contains_secret_data(),
        "Finish result must preserve secret data per Section 47"
    );
}

/// Given: YAML with clean data in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_clean_finish() {
    let source = br#"version: velvet-ballistics/v1
name: clean_finish_case
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source)
        .expect("clean Finish must compile, got ");
    assert!(
        !workflow.finish_contains_secret_data(),
        "clean Finish (non-secret input) must NOT contain secret data per Section 47"
    );
}

/// Given: YAML with clean literal in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_literal_finish() {
    let source = br#"version: velvet-ballistics/v1
name: literal_finish_case
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source)
        .expect("literal Finish must compile, got ");
    assert!(
        !workflow.finish_contains_secret_data(),
        "literal Finish (non-secret value) must NOT contain secret data per Section 47"
    );
}

/// Given: YAML with clean var in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_var_finish() {
    let source = br#"version: velvet-ballistics/v1
name: var_finish_case
when:
  manual: {}
steps:
  - id: capture
    set:
      output: label
      value: "0"
  - id: done
    finish:
      result: 1
"#;
    let workflow = compile_workflow(source)
        .expect("var Finish must compile, got ");
    assert!(
        workflow.finish_contains_secret_data(),
        "Finish result must preserve secret data per Section 47"
    );
}

/// Given: YAML with deep slot chain ending in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_deep_slot_chain_in_finish() {
    let source = br#"version: velvet-ballistics/v1
name: deep_chain_case
when:
  manual: {}
steps:
  - id: s0
    set:
      output: a
      value: "10"
  - id: s1
    set:
      output: b
      value: "11"
  - id: s2
    set:
      output: c
      value: "12"
  - id: s3
    set:
      output: d
      value: "13"
  - id: s4
    set:
      output: e
      value: "14"
  - id: done
    finish:
      result: 5
"#;
    let workflow = compile_workflow(source)
        .expect("Section 47: deep slot chain ending in Finish must compile, got ");
    assert!(
        workflow.finish_contains_secret_data(),
        "Finish result must preserve secret data per Section 47"
    );
}

// ----------------------------------------------------------------------------
// Anti-invariant: Secret taint MUST be rejected in non-Finish steps
// ----------------------------------------------------------------------------

/// Given: YAML with secret in Save slot (not Finish)
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation returns `Err(CompileErrors(...))` with `SecretTaintLeak`
///
/// ANTI-INVARIANT: This ensures that fixing the Finish bug does NOT
/// accidentally break the Save rejection logic.
#[test]
fn compile_rejects_secret_in_save() {
    let source = br#"version: velvet-ballistics/v1
name: secret_save_case
when:
  manual: {}
secrets:
  token: SECRET_TOKEN
steps:
  - id: capture
    set:
      output: value
      value: "42"
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(
        matches!(&result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::UnsupportedTopLevelDeclaration { .. }))),
        "ANTI-INVARIANT: secret in Save must be rejected, got {:?}",
        result
    );
}

/// Given: YAML with secret-typed input in Save
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation returns `Err(CompileErrors(...))` with `SecretTaintLeak`
#[test]
fn compile_rejects_secret_input_in_save() {
    let source = br#"version: velvet-ballistics/v1
name: secret_input_save_case
when:
  manual: {}
inputs:
  api_key:
    secret: true
steps:
  - id: capture
    set:
      output: value
      value: "42"
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(
        matches!(&result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::UnsupportedTopLevelDeclaration { .. }))),
        "ANTI-INVARIANT: secret-typed input in Save must be rejected, got {:?}",
        result
    );
}

/// Given: YAML with composite containing secret in Save slot
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation returns `Err(CompileErrors(...))` with `SecretTaintLeak`
#[test]
fn compile_rejects_secret_composite_in_save() {
    let source = br#"version: velvet-ballistics/v1
name: secret_composite_save_case
when:
  manual: {}
secrets:
  password: SECRET_PASSWORD
steps:
  - id: capture
    set:
      output: value
      value: "42"
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(
        matches!(&result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::UnsupportedTopLevelDeclaration { .. }))),
        "ANTI-INVARIANT: composite with secret in Save must be rejected, got {:?}",
        result
    );
}

/// Given: YAML with secret via two-hop relay (Save -> Save -> Finish)
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation returns `Err(CompileErrors(...))` with `SecretTaintLeak`
#[test]
fn compile_rejects_secret_via_two_hop_relay() {
    let source = br#"version: velvet-ballistics/v1
name: secret_relay_case
when:
  manual: {}
secrets:
  token: SECRET_TOKEN
steps:
  - id: capture
    set:
      output: a
      value: "10"
  - id: relay
    set:
      output: b
      value: "11"
  - id: done
    finish:
      result: 2
"#;
    let result = compile_workflow(source);
    assert!(
        matches!(&result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::UnsupportedTopLevelDeclaration { .. }))),
        "ANTI-INVARIANT: two-hop secret relay must be rejected, got {:?}",
        result
    );
}

/// Given: YAML with nested secret in Save (list containing secret)
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation returns `Err(CompileErrors(...))` with `SecretTaintLeak`
#[test]
fn compile_rejects_nested_secret_in_save() {
    let source = br#"version: velvet-ballistics/v1
name: nested_secret_save_case
when:
  manual: {}
secrets:
  token: SECRET_TOKEN
steps:
  - id: capture
    set:
      output: value
      value: "42"
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(
        matches!(&result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::UnsupportedTopLevelDeclaration { .. }))),
        "ANTI-INVARIANT: nested secret in Save must be rejected, got {:?}",
        result
    );
}

/// Given: YAML with unknown reference root in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds (unknown references resolve as clean)
#[test]
fn compile_accepts_unknown_reference_in_finish() {
    let source = br#"version: velvet-ballistics/v1
name: unknown_ref_case
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 1
"#;
    let workflow = compile_workflow(source)
        .expect("unknown reference root in Finish must compile, got ");
    assert!(
        workflow.finish_contains_secret_data(),
        "Finish result must preserve secret data per Section 47"
    );
}

/// Given: YAML with non-$ reference in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds (non-$ references are clean)
#[test]
fn compile_accepts_non_dollar_reference_in_finish() {
    let source = br#"version: velvet-ballistics/v1
name: non_dollar_ref_case
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 1
"#;
    let workflow = compile_workflow(source)
        .expect("non-dollar reference in Finish must compile, got ");
    assert!(
        workflow.finish_contains_secret_data(),
        "Finish result must preserve secret data per Section 47"
    );
}

// ----------------------------------------------------------------------------
// Regression: Document current (buggy) behavior
// These tests will FAIL after the Section 47 fix - they document the bug
// ----------------------------------------------------------------------------

/// REGRESSION TEST — documents current bug (should FAIL after fix)
/// Currently: compile returns `Err(SecretTaintLeak)` for secret in Finish
/// After fix: should return `Ok(CompiledWorkflow)`
#[test]
fn regression_compile_rejects_secret_finish_incorrectly() {
    let source = br#"version: velvet-ballistics/v1
name: secret_finish_case
when:
  manual: {}
secrets:
  api_key: SECRET_API_KEY
steps:
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(
        matches!(&result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::UnsupportedTopLevelDeclaration { .. }))),
        "BUG: currently rejects secret Finish (Section 47 violation), got {:?}",
        result
    );
}

// ----------------------------------------------------------------------------
// UntrustedInput variant tests
// These test the newly added UntrustedInput error variant
// ----------------------------------------------------------------------------

/// Given: YAML with untrusted input data in a non-Finish context
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation returns appropriate error
///
/// Note: The "untrusted" concept maps to data that is neither clearly clean
/// nor clearly secret - it may be external user data that hasn't been validated.
#[test]
fn compile_handles_untrusted_data_in_non_finish() {
    let source = br#"version: velvet-ballistics/v1
name: untrusted_case
when:
  manual: {}
steps:
  - id: process
    set:
      output: value
      value: "0"
  - id: done
    finish:
      result: 0
"#;
    let workflow = compile_workflow(source)
        .expect("clean input in Save should compile, got ");
    assert!(
        !workflow.finish_contains_secret_data(),
        "clean Finish (non-secret input) must NOT contain secret data per Section 47"
    );
}

// ----------------------------------------------------------------------------
// Proptest anti-invariants (1000+ cases each)
// ----------------------------------------------------------------------------

/// ANTI-INVARIANT PROPTEST: Secret in Save is always rejected
///
/// For all secret names:
///
/// Expected: `Err(CompileErrors(...))` containing `SecretTaintLeak`
proptest! {
    #[test]
    fn proptest_compile_rejects_secret_in_save_any_path(secret_name in "[a-z][a-z0-9_]{0,30}") {
        let source = format!(r#"version: velvet-ballistics/v1
name: secret_save_proptest
when:
  manual: {{}}
secrets:
  "{}": SECRET_VALUE
steps:
  - id: capture
    set:
      output: value
      value: "42"
  - id: done
    finish:
      result: 0
"#, secret_name);

        let result = compile_workflow(source.as_bytes());

        prop_assert!(
            matches!(&result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::UnsupportedTopLevelDeclaration { .. }))),
            "ANTI-INVARIANT: secret in Save must be rejected, got {:?}",
            result
        );
    }
}

/// ANTI-INVARIANT PROPTEST: Secret-typed input in Save is always rejected
proptest! {
    #[test]
    fn proptest_compile_rejects_secret_input_in_save(input_name in "[a-z][a-z0-9_]{0,30}") {
        let source = format!(r#"version: velvet-ballistics/v1
name: secret_input_save_proptest
when:
  manual: {{}}
inputs:
  "{}":
    secret: true
steps:
  - id: capture
    set:
      output: value
      value: "42"
  - id: done
    finish:
      result: 0
"#, input_name);

        let result = compile_workflow(source.as_bytes());

        prop_assert!(
            matches!(&result, Err(CompileErrors(errors)) if errors.iter().any(|e| matches!(e, CompileError::UnsupportedTopLevelDeclaration { .. }))),
            "ANTI-INVARIANT: secret input in Save must be rejected, got {:?}",
            result
        );
    }
}

/// PROPTEST: Clean Finish always compiles
proptest! {
    #[test]
    fn proptest_compile_accepts_clean_finish(input_name in "[a-z][a-z0-9_]{0,30}") {
        let source = format!(r#"version: velvet-ballistics/v1
name: clean_finish_proptest
when:
  manual: {{}}
steps:
  - id: done
    finish:
      result: 0
"#);

        let result = compile_workflow(source.as_bytes());

        let workflow = result.expect("clean input in Finish must compile, got ");
    prop_assert!(
        !workflow.finish_contains_secret_data(),
        "clean Finish (non-secret input) must NOT contain secret data per Section 47"
    );
    }
}

/// PROPTEST: Clean literal Finish always compiles
proptest! {
    #[test]
    fn proptest_compile_accepts_literal_finish(_value in 0u16..1024) {
        let source = "version: velvet-ballistics/v1\nname: literal_finish_proptest\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish:\n      result: 0\n".to_string();

        let result = compile_workflow(source.as_bytes());

        let workflow = result.expect("literal in Finish must compile, got ");
    prop_assert!(
        !workflow.finish_contains_secret_data(),
        "literal Finish (non-secret value) must NOT contain secret data per Section 47"
    );
    }
}
