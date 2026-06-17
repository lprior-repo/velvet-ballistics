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

//! Validation edge case tests covering verified gaps in vb_compile error paths.
//!
//! Each test targets a specific production code path:
//! - `ast/parse.rs` — `parse_trigger` (unknown trigger kind, empty when mapping)
//! - `references.rs` — `validate_slot_reference` (non-numeric slot, non-numeric accessor)
//! - `mod_compile_core.rs` — `check_idempotency_gates` (Unknown retry safety)
//! - `mod_compile_validation/part_04.rs` — `required_branch_targets` (together empty branches)
//!
//! Master spec references:
//! - Trigger kind validation: §38 line 1250
//! - Idempotency gate: §65 line 2620
//! - Together branch shape: §38 line 1580

#![forbid(unsafe_code)]

use crate::{CompileError, CompileErrors, YamlCompiler};
use vb_core::action::ActionName;
use vb_core::{ActionContract, ActionId, Idempotency, RetrySafety, SideEffect};

// ── Helpers ────────────────────────────────────────────────────────────────

/// Helper to construct a minimal `ActionContract` with configurable
/// retry/idempotency/side_effect values while keeping resource fields minimal.
fn make_contract(
    id: u16,
    side_effect: SideEffect,
    retry_safety: RetrySafety,
    idempotency: Idempotency,
) -> ActionContract {
    ActionContract {
        id: ActionId::new(id),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 4096,
        max_output_bytes: 4096,
        timeout_ms: 30_000,
        idempotency,
        side_effect,
        retry_safety,
        required_capabilities: Box::new([]),
    }
}

// ── Gap 1: Unknown trigger kind ───────────────────────────────────────────

/// A workflow that declares an unknown trigger kind must produce
/// `UnknownTriggerKind`. The production `parse_trigger` function matches
/// only `manual`, `webhook`, `schedule`, and `event`; everything else falls
/// through to the `other` arm.
#[test]
fn parse_trigger_unknown_trigger_kind_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: unknown_trigger
when:
  custom_trigger: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let result = YamlCompiler::default().parse_ast(source);
    match result {
        Err(errors) => {
            let err = errors
                .0
                .into_iter()
                .next()
                .expect("expected at least one error");
            assert!(
                matches!(err, CompileError::UnknownTriggerKind { .. }),
                "expected UnknownTriggerKind, got: {err:?}"
            );
            if let CompileError::UnknownTriggerKind { trigger } = err {
                assert_eq!(trigger.as_ref(), "custom_trigger");
            }
        }
        Ok(_) => panic!("parse_ast should have failed for unknown trigger kind 'custom_trigger'"),
    }
}

/// Unknown trigger kind `custom_event` also rejected.
#[test]
fn parse_trigger_unknown_trigger_kind_custom_event_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: unknown_trigger_event
when:
  custom_event: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let result = YamlCompiler::default().parse_ast(source);
    match result {
        Err(errors) => {
            let err = errors
                .0
                .into_iter()
                .next()
                .expect("expected at least one error");
            assert!(
                matches!(err, CompileError::UnknownTriggerKind { .. }),
                "expected UnknownTriggerKind, got: {err:?}"
            );
            if let CompileError::UnknownTriggerKind { trigger } = err {
                assert_eq!(trigger.as_ref(), "custom_event");
            }
        }
        Ok(_) => panic!("parse_ast should have failed for unknown trigger kind 'custom_event'"),
    }
}

// ── Gap 2: Empty when mapping ────────────────────────────────────────────

/// When the `when` mapping is empty, `parse_trigger` sees zero entries and
/// returns `InvalidTriggerCount { count: 0 }`.
#[test]
fn parse_trigger_empty_when_mapping_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: empty_when
when: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let result = YamlCompiler::default().parse_ast(source);
    match result {
        Err(errors) => {
            let err = errors
                .0
                .into_iter()
                .next()
                .expect("expected at least one error");
            assert!(
                matches!(err, CompileError::InvalidTriggerCount { count } if count == 0),
                "expected InvalidTriggerCount {{ count: 0 }}, got: {err:?}"
            );
        }
        Ok(_) => panic!("parse_ast should have failed for empty when mapping"),
    }
}

/// When the `when` mapping has two entries, `parse_trigger` rejects with
/// `InvalidTriggerCount { count: 2 }`.
#[test]
fn parse_trigger_two_trigger_kinds_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: two_triggers
when:
  manual: {}
  webhook: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let result = YamlCompiler::default().parse_ast(source);
    match result {
        Err(errors) => {
            let err = errors
                .0
                .into_iter()
                .next()
                .expect("expected at least one error");
            assert!(
                matches!(
                    err,
                    CompileError::InvalidTriggerCount { count } if count == 2
                ),
                "expected InvalidTriggerCount {{ count: 2 }}, got: {err:?}"
            );
        }
        Ok(_) => panic!("parse_ast should have failed for two trigger kinds"),
    }
}

// ── Gap 3: Idempotency gate — Unknown retry safety ────────────────────────

/// An action contract whose `retry_safety` is `Unknown` and `side_effect` is
/// not `Pure` must produce `IdempotencyViolation` when compiled through
/// `compile_workflow_with_contracts`. The gate in
/// `check_idempotency_gates` checks `RetrySafety::Unknown` alongside
/// `NotRetrySafe`.
#[test]
fn idempotency_gate_unknown_retry_safety_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: unknown_retry
when:
  manual: {}
steps:
  - id: do_action
    do:
      action: "0"
      input: "0"
  - id: done
    finish:
      result: 0
"#;

    let contracts = vec![make_contract(
        0,
        SideEffect::LocalWrite,
        RetrySafety::Unknown,
        Idempotency::IdempotentExternal,
    )];

    match crate::compile_workflow_with_contracts(source, &contracts) {
        Ok(_) => {
            panic!("compile_workflow_with_contracts should have rejected Unknown retry safety")
        }
        Err(CompileErrors(errors)) => {
            let found = errors
                .iter()
                .any(|e| matches!(e, CompileError::IdempotencyViolation { .. }));
            assert!(
                found,
                "expected IdempotencyViolation in errors, got: {errors:?}"
            );
        }
    }
}

/// An action contract with `SideEffect::Pure` and `RetrySafety::Unknown`
/// still passes because the gate rule says Pure always passes regardless of
/// retry/idempotency.
#[test]
fn idempotency_gate_pure_with_unknown_retry_accepts() {
    let source = br#"version: velvet-ballistics/v1
name: pure_unknown_retry
when:
  manual: {}
steps:
  - id: do_action
    do:
      action: "0"
      input: "0"
  - id: done
    finish:
      result: 0
"#;

    let contracts = vec![make_contract(
        0,
        SideEffect::Pure,
        RetrySafety::Unknown,
        Idempotency::DeterministicPure,
    )];

    // Pure side effects always pass regardless of retry/idempotency.
    match crate::compile_workflow_with_contracts(source, &contracts) {
        Ok(_) => {} // expected success
        Err(errors) => {
            panic!("Pure side effect with Unknown retry should pass compilation, got: {errors:?}")
        }
    }
}

/// Side-effecting action with `NotRetrySafe` is rejected by the idempotency
/// gate.
#[test]
fn idempotency_gate_not_retry_safe_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: not_retry_safe
when:
  manual: {}
steps:
  - id: do_action
    do:
      action: "0"
      input: "0"
  - id: done
    finish:
      result: 0
"#;

    let contracts = vec![make_contract(
        0,
        SideEffect::ExternalWrite,
        RetrySafety::NotRetrySafe,
        Idempotency::AtLeastOnceExternal,
    )];

    match crate::compile_workflow_with_contracts(source, &contracts) {
        Ok(_) => panic!("compile_workflow_with_contracts should have rejected NotRetrySafe"),
        Err(CompileErrors(errors)) => {
            assert!(
                errors
                    .iter()
                    .any(|e| matches!(e, CompileError::IdempotencyViolation { .. })),
                "expected IdempotencyViolation, got: {errors:?}"
            );
        }
    }
}

/// Side-effecting action with `Idempotency::AtLeastOnceExternal` is rejected
/// unless retry safety is compatible.
#[test]
fn idempotency_gate_at_least_once_external_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: at_least_once
when:
  manual: {}
steps:
  - id: do_action
    do:
      action: "0"
      input: "0"
  - id: done
    finish:
      result: 0
"#;

    let contracts = vec![make_contract(
        0,
        SideEffect::ExternalWrite,
        RetrySafety::Idempotent,
        Idempotency::AtLeastOnceExternal,
    )];

    match crate::compile_workflow_with_contracts(source, &contracts) {
        Ok(_) => {
            panic!("compile_workflow_with_contracts should have rejected AtLeastOnceExternal")
        }
        Err(CompileErrors(errors)) => {
            assert!(
                errors
                    .iter()
                    .any(|e| matches!(e, CompileError::IdempotencyViolation { .. })),
                "expected IdempotencyViolation, got: {errors:?}"
            );
        }
    }
}

/// Side-effecting action with `Idempotency::DeterministicPure` is rejected.
#[test]
fn idempotency_gate_deterministic_pure_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: det_pure
when:
  manual: {}
steps:
  - id: do_action
    do:
      action: "0"
      input: "0"
  - id: done
    finish:
      result: 0
"#;

    let contracts = vec![make_contract(
        0,
        SideEffect::ExternalWrite,
        RetrySafety::Idempotent,
        Idempotency::DeterministicPure,
    )];

    match crate::compile_workflow_with_contracts(source, &contracts) {
        Ok(_) => {
            panic!("compile_workflow_with_contracts should have rejected DeterministicPure")
        }
        Err(CompileErrors(errors)) => {
            assert!(
                errors
                    .iter()
                    .any(|e| matches!(e, CompileError::IdempotencyViolation { .. })),
                "expected IdempotencyViolation, got: {errors:?}"
            );
        }
    }
}

/// Retry-safe action with `Idempotency::IdempotentExternal` passes.
#[test]
fn idempotency_gate_retry_safe_with_idempotent_external_passes() {
    let source = br#"version: velvet-ballistics/v1
name: safe_idempotent
when:
  manual: {}
steps:
  - id: do_action
    do:
      action: "0"
      input: "0"
  - id: done
    finish:
      result: 0
"#;

    let contracts = vec![make_contract(
        0,
        SideEffect::ExternalWrite,
        RetrySafety::Idempotent,
        Idempotency::IdempotentExternal,
    )];

    match crate::compile_workflow_with_contracts(source, &contracts) {
        Ok(_) => {} // expected success
        Err(errors) => {
            panic!("Idempotent + IdempotentExternal should pass compilation, got: {errors:?}")
        }
    }
}

/// Retry-safe action with `RequiresIdempotencyKey` and
/// `Idempotency::IdempotentExternal` passes.
#[test]
fn idempotency_gate_requires_idempotency_key_passes() {
    let source = br#"version: velvet-ballistics/v1
name: req_key
when:
  manual: {}
steps:
  - id: do_action
    do:
      action: "0"
      input: "0"
  - id: done
    finish:
      result: 0
"#;

    let contracts = vec![make_contract(
        0,
        SideEffect::ExternalWrite,
        RetrySafety::RequiresIdempotencyKey,
        Idempotency::IdempotentExternal,
    )];

    match crate::compile_workflow_with_contracts(source, &contracts) {
        Ok(_) => {}
        Err(errors) => {
            panic!("RequiresIdempotencyKey + IdempotentExternal should pass, got: {errors:?}")
        }
    }
}

// ── Gap 4: Together with empty branches ───────────────────────────────────

/// A Together step with `branches: []` must produce a `StepFieldShape` error
/// from `required_branch_targets`. The validation module checks
/// `branches` (not `together.branches`) and expects
/// "at least one integer step index".
#[test]
fn together_empty_branches_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: empty_branches
when:
  manual: {}
steps:
  - id: fanout
    together:
      branches: []
  - id: done
    finish:
      result: 0
"#;

    let result = YamlCompiler::default().parse_ast(source);
    match result {
        Err(errors) => {
            let err = errors
                .0
                .into_iter()
                .next()
                .expect("expected at least one error");
            assert!(
                matches!(err, CompileError::StepFieldShape { .. }),
                "expected StepFieldShape for empty branches, got: {err:?}"
            );
            if let CompileError::StepFieldShape {
                step,
                field,
                expected,
            } = err
            {
                assert_eq!(step, 0, "diagnostic step should be 0");
                assert_eq!(field, "branches", "field should be 'branches'");
                assert_eq!(
                    expected, "at least one integer step index",
                    "expected shape error about at least one integer step index"
                );
            }
        }
        Ok(_) => panic!("parse_ast should have failed for empty together branches"),
    }
}

/// Together with a single valid branch target (integer step index) must parse.
/// The cold AST Together uses integer step indexes, not labeled branches.
#[test]
fn together_single_branch_accepts() {
    let source = br#"version: velvet-ballistics/v1
name: single_branch
when:
  manual: {}
steps:
  - id: fanout
    together:
      branches:
        - 1
  - id: done
    finish:
      result: 0
"#;

    match YamlCompiler::default().parse_ast(source) {
        Ok(_) => {} // expected success — single integer branch target parses fine
        Err(errors) => panic!("together with one branch should succeed, errors: {errors:?}"),
    }
}

// ── Gap 5: Non-numeric slot reference ─────────────────────────────────────

/// `$slot.abc.path` must produce `UnknownReferenceName` because `abc` is not
/// a valid `u16` slot index. `validate_slot_reference` calls
/// `slot.parse::<u16>()` and rejects on error.
#[test]
fn reference_non_numeric_slot_index_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: bad_slot_name
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slot.abc.path
"#;

    let result = YamlCompiler::default().parse_ast(source);
    match result {
        Err(errors) => {
            let err = errors
                .0
                .into_iter()
                .next()
                .expect("expected at least one error");
            assert!(
                matches!(err, CompileError::UnknownReferenceName { .. }),
                "expected UnknownReferenceName for non-numeric slot, got: {err:?}"
            );
            if let CompileError::UnknownReferenceName {
                kind,
                reference,
                name,
            } = err
            {
                assert_eq!(kind, "slot", "kind must be 'slot'");
                assert_eq!(reference.as_ref(), "$slot.abc.path");
                assert_eq!(name.as_ref(), "abc");
            }
        }
        Ok(_) => panic!("parse_ast should have failed for non-numeric slot index"),
    }
}

/// `$slot.xyz` without accessor path also rejected for non-numeric index.
#[test]
fn reference_non_numeric_slot_index_bare_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: bad_slot_bare
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slot.xyz
"#;

    let result = YamlCompiler::default().parse_ast(source);
    match result {
        Err(errors) => {
            let err = errors
                .0
                .into_iter()
                .next()
                .expect("expected at least one error");
            assert!(
                matches!(
                    err,
                    CompileError::UnknownReferenceName {
                        kind,
                        ref name,
                        ..
                    } if kind == "slot" && name.as_ref() == "xyz"
                ),
                "expected UnknownReferenceName for bare non-numeric slot, got: {err:?}"
            );
        }
        Ok(_) => panic!("parse_ast should have failed for bare non-numeric slot index"),
    }
}

/// `$slot.-1` negative value is parsed by YAML as `$slot.` (the `-1` is
/// consumed as a YAML integer). The resulting empty slot name is still
/// rejected as `UnknownReferenceName`.
#[test]
fn reference_negative_slot_index_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: neg_slot
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slot.-1
"#;

    let result = YamlCompiler::default().parse_ast(source);
    match result {
        Err(errors) => {
            let err = errors
                .0
                .into_iter()
                .next()
                .expect("expected at least one error");
            // The slot index is non-numeric (either empty string from YAML parsing
            // or "-1"), so we expect UnknownReferenceName.
            assert!(
                matches!(err, CompileError::UnknownReferenceName { kind: "slot", .. }),
                "expected UnknownReferenceName for negative slot, got: {err:?}"
            );
        }
        Ok(_) => panic!("parse_ast should have failed for negative slot index"),
    }
}

// ── Gap 6: Non-numeric accessor path ──────────────────────────────────────

/// `$slot.0.field` must produce `UnsupportedAccessorReference` because `field`
/// is not a numeric accessor segment. `validate_slot_reference` checks that
/// the path after the slot index is purely numeric.
#[test]
fn reference_non_numeric_accessor_path_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: bad_accessor
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slot.0.field
"#;

    let result = YamlCompiler::default().parse_ast(source);
    match result {
        Err(errors) => {
            let err = errors
                .0
                .into_iter()
                .next()
                .expect("expected at least one error");
            assert!(
                matches!(err, CompileError::UnsupportedAccessorReference { .. }),
                "expected UnsupportedAccessorReference for non-numeric accessor, got: {err:?}"
            );
            if let CompileError::UnsupportedAccessorReference {
                reference,
                root,
                path,
            } = err
            {
                assert_eq!(reference.as_ref(), "$slot.0.field");
                assert_eq!(root.as_ref(), "slot.0");
                assert_eq!(path.as_ref(), "field");
            }
        }
        Ok(_) => panic!("parse_ast should have failed for non-numeric accessor path"),
    }
}

/// `$slot.0.1.name` — numeric prefix followed by non-numeric segment rejected.
#[test]
fn reference_mixed_accessor_path_rejected() {
    let source = br#"version: velvet-ballistics/v1
name: mixed_accessor
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slot.0.1.name
"#;

    let result = YamlCompiler::default().parse_ast(source);
    match result {
        Err(errors) => {
            let err = errors
                .0
                .into_iter()
                .next()
                .expect("expected at least one error");
            assert!(
                matches!(
                    err,
                    CompileError::UnsupportedAccessorReference {
                        ref reference,
                        ref root,
                        ref path,
                        ..
                    } if reference.as_ref() == "$slot.0.1.name"
                        && root.as_ref() == "slot.0"
                        && path.as_ref() == "1.name"
                ),
                "expected UnsupportedAccessorReference for mixed accessor, got: {err:?}"
            );
        }
        Ok(_) => panic!("parse_ast should have failed for mixed accessor path"),
    }
}

/// Numeric accessor path like `$slot.0.1.2` must succeed.
#[test]
fn reference_numeric_accessor_path_accepts() {
    let source = br#"version: velvet-ballistics/v1
name: good_accessor
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slot.0.1.2
"#;

    match YamlCompiler::default().parse_ast(source) {
        Ok(_) => {} // expected success — numeric accessor path is valid
        Err(errors) => {
            panic!("numeric accessor path should parse successfully, errors: {errors:?}")
        }
    }
}

/// `$slots.0` alternate root spelling passes validation.
#[test]
fn reference_alternate_slots_root_passes() {
    let source = br#"version: velvet-ballistics/v1
name: slots_root
when:
  manual: {}
steps:
  - id: build
    save:
      value: 1
  - id: done
    finish:
      result: $slots.0
"#;

    match YamlCompiler::default().parse_ast(source) {
        Ok(_) => {} // expected success
        Err(errors) => panic!("alternate $slots.0 root should parse, errors: {errors:?}"),
    }
}
