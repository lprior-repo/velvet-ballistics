#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables,
)]

//! Scope-guard regression tests for bead vb-sitry.
//!
//! The `$attempt` root binding is only legal inside a `Repeat` body step.
//! When encountered outside such a body, the compiler must reject the
//! reference with `CompileError::InvalidVariableScope` (the dedicated
//! scope variant) rather than the generic `UnknownReferenceRoot`.
//!
//! ## Architectural note
//!
//! These tests live in their own module so they can run independently of
//! the larger aspirational `attempt_number_tests.rs` suite, which assumes
//! a `Repeat { steps: ... }` shape that the cold AST does not yet preserve.
//!
//! **Why the scope guard is correct under the cold-AST invariant.** The
//! cold AST (master spec §45) drops `StepKindAst::Repeat` body expressions
//! at construction, so the validator never sees a `$attempt.*` reference
//! that is *inside* a `Repeat` body. Any `$attempt.*` that reaches the
//! validator is therefore by definition outside a `Repeat` body. There is
//! no per-step "in a Repeat body" flag on `RefTables` (only declared name
//! sets), and the cold-AST `Repeat` variant carries no body to inspect.
//! The blanket reject is correct under that invariant.
//!
//! **Follow-up bead.** When canonical lowering adds body retention (master
//! §45 follow-up), this guard will need a `repeat_step_indices` set
//! threaded through `RefTables` so the legal use case
//! (`$attempt.number` inside a `Repeat` body step) can be accepted
//! without re-introducing the bare-reference fallback. The current
//! `attempt_number_tests.rs` placeholder already encodes that contract.
//! Until then, the scope guard must remain in force and these tests
//! must keep passing.

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

/// Bare `$attempt` (no accessor) in a top-level expression must also
/// produce `InvalidVariableScope` (or the legacy `UnknownReferenceRoot`
/// fallback for the bare-reference path that delegates to the shared
/// validator). This guards the case where the `attempt` root is
/// matched without a `.something` tail.
#[test]
fn bare_attempt_outside_repeat_emits_invalid_variable_scope_or_unknown_root() {
    let source = br#"version: velvet-ballistics/v1
name: bare_attempt
when:
  manual: {}
steps:
  - id: log
    save:
      value: $attempt
  - id: done
    finish:
      result: 0
"#;
    let err = match YamlCompiler::default().parse_ast(source) {
        Ok(ast) => panic!("parse_ast unexpectedly succeeded with AST: {ast:?}"),
        Err(CompileErrors(errors)) => errors
            .into_iter()
            .next()
            .expect("parse_ast failed with empty error list"),
    };
    assert!(
        matches!(
            err,
            CompileError::InvalidVariableScope { .. } | CompileError::UnknownReferenceRoot { .. }
        ),
        "expected InvalidVariableScope or UnknownReferenceRoot for bare $attempt, got: {err:?}"
    );
}
