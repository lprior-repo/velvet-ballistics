#![forbid(unsafe_code)]
//! Tests for LETHAL-1: validate_taint SecretResultLeak Finish Pass-Through
//!
//! These tests verify the compile pipeline's handling of secret taint in
//! Finish results per Section 47 contract.

use crate::{CompileError, CompileErrors, YamlCompiler};
use proptest::prelude::*;
use vb_core::{CompiledNodeKind, ConstValue, SlotIdx, StepIdx};

// ----------------------------------------------------------------------------
// Test helpers
// ----------------------------------------------------------------------------

fn compile_workflow(source: &[u8]) -> Result<crate::CompiledWorkflow, CompileErrors> {
    YamlCompiler::default().compile(source)
}

fn assert_exact_secret_taint_leak_errors(
    result: Result<crate::CompiledWorkflow, CompileErrors>,
    expected_count: usize,
    context: &str,
) {
    let Err(CompileErrors(errors)) = result else {
        panic!("{context}: expected SecretTaintLeak errors");
    };
    assert_eq!(
        errors.len(),
        expected_count,
        "{context}: wrong SecretTaintLeak count: {errors:?}"
    );
    for error in errors {
        assert!(
            matches!(error, CompileError::SecretTaintLeak { field: "save.value" }),
            "{context}: expected only SecretTaintLeak(save.value), got {error:?}"
        );
    }
}

fn assert_literal_finish_shape(
    workflow: &crate::CompiledWorkflow,
    expected: i64,
) -> Result<(), String> {
    let parts = workflow.to_parts();
    assert_eq!(parts.slot_count, 1, "literal finish slot count");
    assert_eq!(parts.nodes.len(), 2, "literal finish node count");
    let Some(set_node) = parts.nodes.first() else {
        return Err(String::from("literal finish workflow missing SetConst node"));
    };
    assert_eq!(
        set_node.output,
        Some(SlotIdx::new(0)),
        "literal finish SetConst output slot"
    );
    assert_eq!(
        set_node.next,
        Some(StepIdx::new(1)),
        "literal finish SetConst next node"
    );
    let const_idx = match &set_node.kind {
        CompiledNodeKind::SetConst { value } => value,
        other => return Err(format!("expected SetConst node, got {other:?}")),
    };
    let Some(constant) = parts.constants.get(const_idx.as_usize()) else {
        return Err(format!("missing constant at index {}", const_idx.get()));
    };
    assert_eq!(
        constant,
        &ConstValue::I64(expected),
        "literal finish constant payload"
    );

    let Some(finish_node) = parts.nodes.get(1) else {
        return Err(String::from("literal finish workflow missing Finish node"));
    };
    assert_eq!(finish_node.output, None, "literal finish output");
    assert_eq!(finish_node.next, None, "literal finish next");
    match &finish_node.kind {
        CompiledNodeKind::Finish { result } => {
            assert_eq!(*result, SlotIdx::new(0), "literal finish result slot");
            Ok(())
        }
        other => Err(format!("expected Finish node, got {other:?}")),
    }
}

fn assert_finish_result_shape(
    workflow: &crate::CompiledWorkflow,
    context: &str,
) -> Result<(), String> {
    let parts = workflow.to_parts();
    assert!(
        !parts.nodes.is_empty(),
        "{context}: compiled workflow must contain at least one node"
    );
    let finish_index = parts
        .nodes
        .len()
        .checked_sub(1)
        .ok_or_else(|| format!("{context}: compiled workflow missing Finish node"))?;
    let Some(finish_node) = parts.nodes.get(finish_index) else {
        return Err(format!("{context}: compiled workflow missing Finish node"));
    };
    assert_eq!(finish_node.output, None, "{context}: Finish output");
    assert_eq!(finish_node.next, None, "{context}: Finish next");
    match &finish_node.kind {
        CompiledNodeKind::Finish { result } => {
            assert!(
                result.as_usize() < usize::from(parts.slot_count),
                "{context}: Finish result slot {} must be within slot_count {}",
                result.get(),
                parts.slot_count
            );
            Ok(())
        }
        other => Err(format!("{context}: expected Finish node, got {other:?}")),
    }
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
    let source = br#"version: velvet-ballastics/v1
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
    let workflow = compile_workflow(source).map_err(|errors| {
        format!("Section 47: secret in Finish must compile, got {errors:?}")
    });
    let workflow = match workflow {
        Ok(workflow) => workflow,
        Err(message) => panic!("{message}"),
    };
    assert_finish_result_shape(&workflow, "secret finish result").unwrap_or_else(|message| {
        panic!("{message}");
    });
}

/// Given: YAML with secret-derived data via slot relay in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_secret_slot_relay_in_finish() {
    let source = br#"version: velvet-ballastics/v1
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
    let workflow = compile_workflow(source).unwrap_or_else(|errors| {
        panic!("Section 47: secret via slot relay in Finish must compile, got {errors:?}");
    });
    assert_finish_result_shape(&workflow, "secret slot relay finish").unwrap_or_else(|message| {
        panic!("{message}");
    });
}

/// Given: YAML with composite containing secret in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_secret_composite_in_finish() {
    let source = br#"version: velvet-ballastics/v1
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
    let workflow = compile_workflow(source).unwrap_or_else(|errors| {
        panic!("Section 47: composite with secret in Finish must compile, got {errors:?}");
    });
    assert_finish_result_shape(&workflow, "secret composite finish").unwrap_or_else(|message| {
        panic!("{message}");
    });
}

/// Given: YAML with inline list containing secret in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_secret_list_in_finish() {
    let source = br#"version: velvet-ballastics/v1
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
    let workflow = compile_workflow(source).unwrap_or_else(|errors| {
        panic!("Section 47: list with secret in Finish must compile, got {errors:?}");
    });
    assert_finish_result_shape(&workflow, "secret list finish").unwrap_or_else(|message| {
        panic!("{message}");
    });
}

/// Given: YAML with clean data in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_clean_finish() {
    let source = br#"version: velvet-ballastics/v1
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
    let workflow = compile_workflow(source).unwrap_or_else(|errors| {
        panic!("clean Finish must compile, got {errors:?}");
    });
    assert_finish_result_shape(&workflow, "clean input finish").unwrap_or_else(|message| {
        panic!("{message}");
    });
}

/// Given: YAML with clean literal in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_literal_finish() {
    let source = br#"version: velvet-ballastics/v1
name: literal_finish_case
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 42
"#;
    let workflow = compile_workflow(source).unwrap_or_else(|errors| {
        panic!("literal Finish must compile, got {errors:?}");
    });
    assert_literal_finish_shape(&workflow, 42).unwrap_or_else(|message| {
        panic!("{message}");
    });
}

/// Given: YAML with clean var in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_var_finish() {
    let source = br#"version: velvet-ballastics/v1
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
    let workflow = compile_workflow(source).unwrap_or_else(|errors| {
        panic!("var Finish must compile, got {errors:?}");
    });
    assert_finish_result_shape(&workflow, "var finish").unwrap_or_else(|message| {
        panic!("{message}");
    });
}

/// Given: YAML with deep slot chain ending in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds
#[test]
fn compile_accepts_deep_slot_chain_in_finish() {
    let source = br#"version: velvet-ballastics/v1
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
    let workflow = compile_workflow(source).unwrap_or_else(|errors| {
        panic!("Section 47: deep slot chain ending in Finish must compile, got {errors:?}");
    });
    assert_finish_result_shape(&workflow, "deep slot chain finish").unwrap_or_else(|message| {
        panic!("{message}");
    });
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
    let source = br#"version: velvet-ballastics/v1
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
    assert_exact_secret_taint_leak_errors(result, 1, "ANTI-INVARIANT: secret in Save");
}

/// Given: YAML with secret-typed input in Save
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation returns `Err(CompileErrors(...))` with `SecretTaintLeak`
#[test]
fn compile_rejects_secret_input_in_save() {
    let source = br#"version: velvet-ballastics/v1
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
    assert_exact_secret_taint_leak_errors(result, 1, "ANTI-INVARIANT: secret input in Save");
}

/// Given: YAML with composite containing secret in Save slot
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation returns `Err(CompileErrors(...))` with `SecretTaintLeak`
#[test]
fn compile_rejects_secret_composite_in_save() {
    let source = br#"version: velvet-ballastics/v1
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
    assert_exact_secret_taint_leak_errors(result, 1, "ANTI-INVARIANT: composite secret in Save");
}

/// Given: YAML with secret via two-hop relay (Save -> Save -> Finish)
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation returns `Err(CompileErrors(...))` with `SecretTaintLeak`
#[test]
fn compile_rejects_secret_via_two_hop_relay() {
    let source = br#"version: velvet-ballastics/v1
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
    assert_exact_secret_taint_leak_errors(result, 1, "ANTI-INVARIANT: two-hop secret relay");
}

/// Given: YAML with nested secret in Save (list containing secret)
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation returns `Err(CompileErrors(...))` with `SecretTaintLeak`
#[test]
fn compile_rejects_nested_secret_in_save() {
    let source = br#"version: velvet-ballastics/v1
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
    assert_exact_secret_taint_leak_errors(result, 1, "ANTI-INVARIANT: nested secret in Save");
}

/// Given: YAML with unknown reference root in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds (unknown references resolve as clean)
#[test]
fn compile_accepts_unknown_reference_in_finish() {
    let source = br#"version: velvet-ballastics/v1
name: unknown_ref_case
when:
  manual: {}
steps:
  - id: done
    finish:
      result: $unknown_root.field
"#;
    let workflow = compile_workflow(source).unwrap_or_else(|errors| {
        panic!("unknown reference root in Finish must compile, got {errors:?}");
    });
    assert_finish_result_shape(&workflow, "unknown reference finish").unwrap_or_else(|message| {
        panic!("{message}");
    });
}

/// Given: YAML with non-$ reference in Finish
/// When:  `YamlCompiler::default().compile(source)` is called
/// Then:  The compilation succeeds (non-$ references are clean)
#[test]
fn compile_accepts_non_dollar_reference_in_finish() {
    let source = br#"version: velvet-ballastics/v1
name: non_dollar_ref_case
when:
  manual: {}
steps:
  - id: done
    finish:
      result: not_a_reference
"#;
    let workflow = compile_workflow(source).unwrap_or_else(|errors| {
        panic!("non-dollar reference in Finish must compile, got {errors:?}");
    });
    assert_finish_result_shape(&workflow, "non-dollar reference finish").unwrap_or_else(|message| {
        panic!("{message}");
    });
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
    let source = br#"version: velvet-ballastics/v1
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
        let source = format!(r#"version: velvet-ballastics/v1
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

        let Err(CompileErrors(errors)) = result else {
            return Err(TestCaseError::fail("ANTI-INVARIANT: secret in Save unexpectedly compiled"));
        };
        prop_assert_eq!(errors.len(), 1);
        prop_assert!(matches!(errors.as_slice(), [CompileError::SecretTaintLeak { field: "save.value" }]));
    }
}

/// ANTI-INVARIANT PROPTEST: Secret-typed input in Save is always rejected
proptest! {
    #[test]
    fn proptest_compile_rejects_secret_input_in_save(input_name in "[a-z][a-z0-9_]{0,30}") {
        let source = format!(r#"version: velvet-ballastics/v1
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

        let Err(CompileErrors(errors)) = result else {
            return Err(TestCaseError::fail("ANTI-INVARIANT: secret input in Save unexpectedly compiled"));
        };
        prop_assert_eq!(errors.len(), 1);
        prop_assert!(matches!(errors.as_slice(), [CompileError::SecretTaintLeak { field: "save.value" }]));
    }
}

/// PROPTEST: Clean Finish always compiles
proptest! {
    #[test]
    fn proptest_compile_accepts_clean_finish(input_name in "[a-z][a-z0-9_]{0,30}") {
        let source = format!(r#"version: velvet-ballastics/v1
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
        let source = format!(r#"version: velvet-ballastics/v1
name: literal_finish_proptest
when:
  manual: {{}}
steps:
  - id: done
    finish:
      result: {}
"#, value);

        let workflow = compile_workflow(source.as_bytes()).map_err(|errors| {
            TestCaseError::fail(format!("literal in Finish must compile, got {errors:?}"))
        })?;

        assert_literal_finish_shape(&workflow, i64::from(value)).map_err(TestCaseError::fail)?;
    }
}
