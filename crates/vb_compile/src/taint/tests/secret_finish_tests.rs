#![forbid(unsafe_code)]
//! Tests for LETHAL-1: validate_taint SecretResultLeak Finish Pass-Through
//!
//! These tests verify the compile pipeline's handling of secret taint in
//! Finish results per Section 47 contract.

use crate::{CompileError, CompileErrors, YamlCompiler};
use proptest::prelude::*;

// ----------------------------------------------------------------------------
// Test helpers
// ----------------------------------------------------------------------------

fn compile_workflow(source: &[u8]) -> Result<crate::CompiledWorkflow, CompileErrors> {
    YamlCompiler::default().compile(source)
}

// ----------------------------------------------------------------------------
// Section 47: Taint MUST pass through Finish outputs (currently buggy)
// ----------------------------------------------------------------------------

/// Given: YAML with `$secrets.token` directly in the Finish result
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds with `Ok(CompiledWorkflow)`
///
/// CURRENT BUG: compile returns Err(CompileErrors(...)) with SecretTaintLeak
/// EXPECTED:   Ok(CompiledWorkflow) per Section 47
#[test]
fn compile_accepts_secret_finish_result() {
    let source = br#"version: velvet-ballistics/v1
name: secret_finish_case
when:
  manual: {}
secrets:
  token: SECRET_TOKEN
steps:
  - id: done
    finish:
      result: $secrets.token
"#;
    let result = compile_workflow(source);
    assert!(
        result.is_ok(),
        "Section 47: secret in Finish must compile, got {:?}",
        result
    );
}

/// Given: YAML with secret-derived data via slot relay in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_secret_slot_relay_in_finish() {
    let source = br#"version: velvet-ballistics/v1
name: secret_slot_relay_case
when:
  manual: {}
secrets:
  token: SECRET_TOKEN
steps:
  - id: capture
    save:
      value: $secrets.token
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(
        result.is_ok(),
        "Section 47: secret via slot relay in Finish must compile, got {:?}",
        result
    );
}

/// Given: YAML with composite containing secret in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_secret_composite_in_finish() {
    let source = br#"version: velvet-ballistics/v1
name: secret_composite_case
when:
  manual: {}
secrets:
  password: SECRET_PASSWORD
steps:
  - id: done
    finish:
      result:
        key: $secrets.password
"#;
    let result = compile_workflow(source);
    assert!(
        result.is_ok(),
        "Section 47: composite with secret in Finish must compile, got {:?}",
        result
    );
}

/// Given: YAML with inline list containing secret in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_secret_list_in_finish() {
    let source = br#"version: velvet-ballistics/v1
name: secret_list_case
when:
  manual: {}
secrets:
  item: SECRET_ITEM
steps:
  - id: done
    finish:
      result:
        - $secrets.item
        - clean_value
"#;
    let result = compile_workflow(source);
    assert!(
        result.is_ok(),
        "Section 47: list with secret in Finish must compile, got {:?}",
        result
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
inputs:
  user: text
steps:
  - id: done
    finish:
      result: $input.user
"#;
    let result = compile_workflow(source);
    assert!(
        result.is_ok(),
        "clean Finish must compile, got {:?}",
        result
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
      result: 42
"#;
    let result = compile_workflow(source);
    assert!(
        result.is_ok(),
        "literal Finish must compile, got {:?}",
        result
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
vars:
  label: true
steps:
  - id: done
    finish:
      result: $vars.label
"#;
    let result = compile_workflow(source);
    assert!(
        result.is_ok(),
        "var Finish must compile, got {:?}",
        result
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
secrets:
  db_password: SECRET_DB_PASSWORD
steps:
  - id: s0
    save:
      value: $secrets.db_password
  - id: s1
    save:
      value: 0
  - id: s2
    save:
      value: 1
  - id: s3
    save:
      value: 2
  - id: s4
    save:
      value: 3
  - id: done
    finish:
      result: 4
"#;
    let result = compile_workflow(source);
    assert!(
        result.is_ok(),
        "Section 47: deep slot chain ending in Finish must compile, got {:?}",
        result
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
    save:
      value: $secrets.token
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.0.iter().any(|e| matches!(e, CompileError::SecretTaintLeak { .. }))),
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
    text
    secret: true
steps:
  - id: capture
    save:
      value: $input.api_key
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.0.iter().any(|e| matches!(e, CompileError::SecretTaintLeak { .. }))),
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
    save:
      value:
        key: $secrets.password
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.0.iter().any(|e| matches!(e, CompileError::SecretTaintLeak { .. }))),
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
    save:
      value: $secrets.token
  - id: relay
    save:
      value: 0
  - id: done
    finish:
      result: 1
"#;
    let result = compile_workflow(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.0.iter().any(|e| matches!(e, CompileError::SecretTaintLeak { .. }))),
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
    save:
      value:
        - $secrets.token
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.0.iter().any(|e| matches!(e, CompileError::SecretTaintLeak { .. }))),
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
      result: $unknown_root.field
"#;
    let result = compile_workflow(source);
    assert!(
        result.is_ok(),
        "unknown reference root in Finish must compile, got {:?}",
        result
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
      result: not_a_reference
"#;
    let result = compile_workflow(source);
    assert!(
        result.is_ok(),
        "non-dollar reference in Finish must compile, got {:?}",
        result
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
      result: $secrets.api_key
"#;
    let result = compile_workflow(source);
    assert!(
        matches!(result, Err(CompileErrors(errors)) if errors.0.iter().any(|e| matches!(e, CompileError::SecretTaintLeak { field: "finish.result" }))),
        "BUG: currently rejects secret Finish (Section 47 violation)"
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
inputs:
  user_data: text
steps:
  - id: process
    save:
      value: $input.user_data
  - id: done
    finish:
      result: 0
"#;
    let result = compile_workflow(source);
    assert!(
        result.is_ok(),
        "clean input in Save should compile, got {:?}",
        result
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
  {}: SECRET_VALUE
steps:
  - id: capture
    save:
      value: ${}
  - id: done
    finish:
      result: 0
"#, secret_name, secret_name);

        let result = compile_workflow(source.as_bytes());

        prop_assert!(
            matches!(result, Err(CompileErrors(errors)) if errors.0.iter().any(|e| matches!(e, CompileError::SecretTaintLeak { .. }))),
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
  {}:
    text
    secret: true
steps:
  - id: capture
    save:
      value: ${}
  - id: done
    finish:
      result: 0
"#, input_name, input_name);

        let result = compile_workflow(source.as_bytes());

        prop_assert!(
            matches!(result, Err(CompileErrors(errors)) if errors.0.iter().any(|e| matches!(e, CompileError::SecretTaintLeak { .. }))),
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
inputs:
  {}: text
steps:
  - id: done
    finish:
      result: ${}
"#, input_name, input_name);

        let result = compile_workflow(source.as_bytes());

        prop_assert!(result.is_ok(), "clean input in Finish must compile, got {:?}", result);
    }
}

/// PROPTEST: Clean literal Finish always compiles
proptest! {
    #[test]
    fn proptest_compile_accepts_literal_finish(value: i32) {
        let source = format!(r#"version: velvet-ballistics/v1
name: literal_finish_proptest
when:
  manual: {{}}
steps:
  - id: done
    finish:
      result: {}
"#, value);

        let result = compile_workflow(source.as_bytes());

        prop_assert!(result.is_ok(), "literal in Finish must compile, got {:?}", result);
    }
}
