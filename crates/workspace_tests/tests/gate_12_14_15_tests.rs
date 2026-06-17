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
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::enum_variant_names,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
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
    unused_variables
)]
#![forbid(unsafe_code)]
//! Unit tests for Gates 12, 14, 15 (bead vb-qi37.8).
//!
//! These tests complement the existing gate_tests.rs and cover:
//! - Gate 12: Action contract bijection (8 tests)
//! - Gate 14: Slot type consistency (4 tests)
//! - Gate 15: Determinism proof (5 tests)

use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::Capability;
use vb_core::ids::{ActionId, ConstIdx, SlotIdx, StepIdx};
use vb_core::value::ConstValue;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_validate::ValidationError;
use vb_validate::gates::{
    validate_gate_12_action_contract_completeness, validate_gate_14_slot_type_consistency,
    validate_gate_15_determinism_proof,
};

// ---------------------------------------------------------------------------
// Helper constructors
// ---------------------------------------------------------------------------

fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn finish_node(index: u16, result_slot: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(result_slot),
        },
    }
}

fn do_node(index: u16, action: u16, input: u16, next: Option<StepIdx>) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: Some(SlotIdx::new(0)),
        next,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(action),
            input: SlotIdx::new(input),
        },
    }
}

fn make_contract(action_id: u16) -> ActionContract {
    ActionContract {
        id: ActionId::new(action_id),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    }
}

// ===========================================================================
// Gate 12: Action contract completeness
// ===========================================================================

#[test]
fn gate_12_accepts_empty_do_nodes() {
    // No Do nodes, no contracts => trivially valid bijection
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    let contracts: Vec<ActionContract> = vec![];
    assert_eq!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Ok(())
    );
}

#[test]
fn gate_12_accepts_single_do_with_contract() {
    let nodes = vec![do_node(0, 1, 0, Some(StepIdx::new(1))), finish_node(1, 0)];
    let parts = make_parts(nodes, 1);
    let contracts = vec![make_contract(1)];
    assert_eq!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Ok(())
    );
}

#[test]
fn gate_12_accepts_multiple_matching() {
    let nodes = vec![
        do_node(0, 1, 0, Some(StepIdx::new(1))),
        do_node(1, 2, 0, Some(StepIdx::new(2))),
        do_node(2, 3, 0, None),
    ];
    let parts = make_parts(nodes, 1);
    let contracts = vec![make_contract(1), make_contract(2), make_contract(3)];
    assert_eq!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Ok(())
    );
}

#[test]
fn gate_12_rejects_missing_contract() {
    let nodes = vec![do_node(0, 99, 0, Some(StepIdx::new(1))), finish_node(1, 0)];
    let parts = make_parts(nodes, 1);
    let contracts = vec![make_contract(1)]; // No contract for action 99
    assert!(matches!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Err(ValidationError::ActionContractMissing {
            action_id: 99,
            node_index: 0
        })
    ));
}

#[test]
fn gate_12_rejects_orphan_contract() {
    let nodes = vec![finish_node(0, 0)];
    let parts = make_parts(nodes, 1);
    let contracts = vec![make_contract(42)]; // No Do node uses action 42
    assert!(matches!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Err(ValidationError::ActionContractOrphan { action_id: 42 })
    ));
}

#[test]
fn gate_12_duplicate_do_same_action() {
    // Two Do nodes with same action_id, one contract => valid
    let nodes = vec![
        do_node(0, 1, 0, Some(StepIdx::new(1))),
        do_node(1, 1, 0, Some(StepIdx::new(2))), // Same action_id
        finish_node(2, 0),
    ];
    let parts = make_parts(nodes, 1);
    let contracts = vec![make_contract(1)]; // One contract covers both Do nodes
    assert_eq!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Ok(())
    );
}

#[test]
fn gate_12_contract_capability_validation() {
    // Contract with empty capability name should fail
    let nodes = vec![do_node(0, 1, 0, Some(StepIdx::new(1))), finish_node(1, 0)];
    let parts = make_parts(nodes, 1);
    let bad_contract = ActionContract {
        id: ActionId::new(1),
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([Capability::new(
            Box::from(""), // Empty name - invalid
            ActionId::new(1),
        )]),
    };
    let result = validate_gate_12_action_contract_completeness(&parts, &[bad_contract]);
    assert!(matches!(
        result,
        Err(ValidationError::CapabilityNameEmpty {
            action_id: 1,
            capability_index: 0
        })
    ));
}

#[test]
fn gate_12_deterministic_behavior() {
    let nodes = vec![do_node(0, 1, 0, Some(StepIdx::new(1))), finish_node(1, 0)];
    let parts = make_parts(nodes, 1);
    let contracts = vec![make_contract(1)];

    let r1 = validate_gate_12_action_contract_completeness(&parts, &contracts);
    let r2 = validate_gate_12_action_contract_completeness(&parts, &contracts);
    assert_eq!(r1, r2);
}

// ===========================================================================
// Gate 14: Slot type consistency
// ===========================================================================

#[test]
fn gate_14_accepts_single_writer() {
    let nodes = vec![CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    }];
    let mut parts = make_parts(nodes, 1);
    parts.constants = Box::new([ConstValue::I64(42)]);
    assert_eq!(validate_gate_14_slot_type_consistency(&parts), Ok(()));
}

#[test]
fn gate_14_accepts_same_type_multi_writer() {
    // Both writers write I64 to slot 0
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(1),
            },
        },
    ];
    let mut parts = make_parts(nodes, 1);
    parts.constants = Box::new([ConstValue::I64(1), ConstValue::I64(2)]);
    assert_eq!(validate_gate_14_slot_type_consistency(&parts), Ok(()));
}

#[test]
fn gate_14_rejects_incompatible_types() {
    // Writer 1: I64, Writer 2: Bool to same slot
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(1),
            },
        },
    ];
    let mut parts = make_parts(nodes, 1);
    parts.constants = Box::new([ConstValue::I64(1), ConstValue::Bool(true)]);
    assert!(matches!(
        validate_gate_14_slot_type_consistency(&parts),
        Err(ValidationError::SlotTypeInconsistency { slot: 0 })
    ));
}

#[test]
fn gate_14_accepts_empty_slots() {
    let parts = make_parts(vec![finish_node(0, 0)], 0);
    assert_eq!(validate_gate_14_slot_type_consistency(&parts), Ok(()));
}

// ===========================================================================
// Gate 15: Determinism proof
// ===========================================================================

#[test]
fn gate_15_accepts_no_nd_nodes() {
    // Only deterministic nodes (Nop, SetConst, Copy, Finish)
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        },
        finish_node(1, 0),
    ];
    let parts = make_parts(nodes, 1);
    assert_eq!(validate_gate_15_determinism_proof(&parts), Ok(()));
}

#[test]
fn gate_15_accepts_single_nd_node() {
    let nodes = vec![do_node(0, 1, 0, Some(StepIdx::new(1))), finish_node(1, 0)];
    let parts = make_parts(nodes, 1);
    assert_eq!(validate_gate_15_determinism_proof(&parts), Ok(()));
}

#[test]
fn gate_15_accepts_separated_nd_nodes() {
    // Two ND nodes with deterministic nodes in between
    let nodes = vec![
        do_node(0, 1, 0, Some(StepIdx::new(1))),
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        },
        do_node(2, 2, 0, None),
    ];
    let parts = make_parts(nodes, 1);
    assert_eq!(validate_gate_15_determinism_proof(&parts), Ok(()));
}

#[test]
fn gate_15_rejects_adjacent_nd_nodes() {
    // Two ND nodes directly chained
    let nodes = vec![
        do_node(0, 1, 0, Some(StepIdx::new(1))),
        do_node(1, 2, 0, None),
    ];
    let parts = make_parts(nodes, 1);
    assert!(matches!(
        validate_gate_15_determinism_proof(&parts),
        Err(ValidationError::NonDeterministicPath {
            from_node: 0,
            to_node: 1
        })
    ));
}

#[test]
fn gate_15_ask_is_non_deterministic() {
    // Ask is also non-deterministic like Do
    let nodes = vec![
        CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: vb_core::workflow::CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: vb_core::workflow::CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
        },
    ];
    let parts = make_parts(nodes, 1);
    assert!(matches!(
        validate_gate_15_determinism_proof(&parts),
        Err(ValidationError::NonDeterministicPath { .. })
    ));
}

// =========================================================================
// vb-u09ai: 4-variant RetrySafety gate_12_14_15 test (Tier 1).
// =========================================================================

/// Tier 1: `vb_core::action::is_idempotent(RetrySafety::Idempotent) == true`
/// per the master §65 contract (C6). The `is_idempotent(RetrySafety)` const
/// fn is a TDD target State 11 will add — on 3-variant code this test
/// fails to compile (preserves the failing-first signal).
#[test]
fn gate_12_14_15_idempotent_retry_safety_recognized() {
    use vb_core::action::{RetrySafety, is_idempotent};
    assert!(
        is_idempotent(RetrySafety::Idempotent),
        "Idempotent must be considered idempotent (C6)"
    );
}
