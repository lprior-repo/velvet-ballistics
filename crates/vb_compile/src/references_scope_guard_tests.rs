//! Scope-guard regression tests for bead vb-sitry.
//!
//! The `$attempt` root binding is only legal inside a `Repeat` body step.
//! When encountered outside such a body, the compiler must reject the
//! reference with `CompileError::InvalidVariableScope` (the dedicated
//! scope variant) rather than the generic `UnknownReferenceRoot`.
//!
//! These tests live in their own module so they can run independently of
//! the larger aspirational `attempt_number_tests.rs` suite, which assumes
//! a `Repeat { steps: ... }` shape that the cold AST does not yet preserve.

use crate::{CompileError, CompileErrors, YamlCompiler};

/// Helper: parse source and return the first error variant, or panic on
/// unexpected success / empty error list.
fn first_error(source: &[u8]) -> CompileError {
    match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => panic!("parse_ast unexpectedly succeeded with AST: {ast:?}"),
        Err(CompileErrors(errors)) => errors
            .into_iter()
            .next()
            .expect("parse_ast failed with empty error list"),
    }
}

/// `$attempt.number` in a `save` field at the top level (not inside a
/// Repeat body) must produce `InvalidVariableScope` with the canonical
/// payload.
#[test]
fn attempt_number_in_save_outside_repeat_emits_invalid_variable_scope() {
    let source = br#"version: velvet-ballistics/v1
name: save_attempt
when:
  manual: {}
steps:
  - id: log_attempt
    save:
      value: $attempt.number
  - id: done
    finish:
      result: 0
"#;
    let err = first_error(source);
    match err {
        CompileError::InvalidVariableScope {
            reference,
            context,
            allowed,
            mark: _,
        } => {
            assert_eq!(reference.as_ref(), "$attempt.number");
            assert_eq!(context, "outside repeat body");
            assert_eq!(
                allowed.as_ref(),
                ["repeat_attempt.body", "repeat_check"].as_slice()
            );
        }
        other => panic!(
            "expected CompileError::InvalidVariableScope for $attempt.number in save, got: {other:?}"
        ),
    }
}

/// `$attempt.number` in a `do.input` is not exercisable here: the `do:`
/// primitive's `input:` field is a slot index, not a reference string, and
/// is rejected by the schema before the reference validator runs. (The
/// `save:` case above is the canonical reference-bearing primitive at the
/// top level; the `choose.condition` case below is the other one.)

/// `$attempt.number` in a `choose.condition` at the top level must
/// produce `InvalidVariableScope`.
#[test]
fn attempt_number_in_choose_outside_repeat_emits_invalid_variable_scope() {
    let source = br#"version: velvet-ballistics/v1
name: choose_attempt
when:
  manual: {}
steps:
  - id: route
    choose:
      condition: $attempt.number > 1
      on_true: 1
      on_false: 1
  - id: done
    finish:
      result: 0
"#;
    let err = first_error(source);
    assert!(
        matches!(err, CompileError::InvalidVariableScope { .. }),
        "expected InvalidVariableScope, got: {err:?}"
    );
}
