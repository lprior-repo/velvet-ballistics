#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]

//! Tests for Gates 12, 14, 15.

use super::*;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::ids::{ActionId, ConstIdx, SlotIdx, StepIdx};
use vb_core::value::ConstValue;
use vb_core::workflow::{CompiledNode, ResourceContract};

use crate::ValidationError;
use crate::gate_12_14_15::{
    validate_gate_12_action_contract_completeness, validate_gate_14_slot_type_consistency,
    validate_gate_15_determinism_proof,
};

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

fn nop_node(index: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: Some(StepIdx::new(index.saturating_add(1))),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }
}

fn do_node(index: u16, action: u16, input: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(index.saturating_add(1))),
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

fn set_const_node(index: u16, const_idx: u16, output_slot: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: Some(SlotIdx::new(output_slot)),
        next: Some(StepIdx::new(index.saturating_add(1))),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(const_idx),
        },
    }
}

// ===== Gate 12 tests =====

#[test]
fn gate_12_accepts_no_do_nodes_no_contracts() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(
        validate_gate_12_action_contract_completeness(&parts, &[]),
        Ok(())
    );
}

#[test]
fn gate_12_accepts_do_node_with_matching_contract() {
    let parts = make_parts(vec![do_node(0, 1, 0), finish_node(1, 0)], 1);
    let contracts = [make_contract(1)];
    assert_eq!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Ok(())
    );
}

#[test]
fn gate_12_accepts_multiple_do_nodes_with_contracts() {
    let parts = make_parts(
        vec![do_node(0, 1, 0), do_node(1, 2, 0), finish_node(2, 0)],
        1,
    );
    let contracts = [make_contract(1), make_contract(2)];
    assert_eq!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Ok(())
    );
}

#[test]
fn gate_12_rejects_missing_contract() {
    let parts = make_parts(vec![do_node(0, 5, 0), finish_node(1, 0)], 1);
    assert!(matches!(
        validate_gate_12_action_contract_completeness(&parts, &[]),
        Err(ValidationError::ActionContractMissing { .. })
    ));
}

#[test]
fn gate_12_rejects_orphan_contract() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    let contracts = [make_contract(99)];
    assert!(matches!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Err(ValidationError::ActionContractOrphan { .. })
    ));
}

#[test]
fn gate_12_rejects_partial_mismatch() {
    let parts = make_parts(vec![do_node(0, 1, 0), finish_node(1, 0)], 1);
    let contracts = [make_contract(1), make_contract(2)];
    // contract 2 has no matching Do node -> orphan
    assert!(matches!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Err(ValidationError::ActionContractOrphan { .. })
    ));
}

#[test]
fn gate_12_accepts_two_do_nodes_one_contract() {
    // Two Do nodes using the same action_id, one contract covers both.
    let parts = make_parts(
        vec![do_node(0, 1, 0), do_node(1, 1, 0), finish_node(2, 0)],
        1,
    );
    let contracts = [make_contract(1)];
    assert_eq!(
        validate_gate_12_action_contract_completeness(&parts, &contracts),
        Ok(())
    );
}

// ===== Gate 14 tests =====

#[test]
fn gate_14_accepts_empty_slots() {
    let parts = make_parts(vec![nop_node(0)], 0);
    assert_eq!(validate_gate_14_slot_type_consistency(&parts), Ok(()));
}

#[test]
fn gate_14_accepts_consistent_types() {
    let mut parts = make_parts(
        vec![
            set_const_node(0, 0, 0),
            set_const_node(1, 1, 0),
            finish_node(2, 0),
        ],
        1,
    );
    parts.constants = Box::new([ConstValue::I64(1), ConstValue::I64(2)]);
    assert_eq!(validate_gate_14_slot_type_consistency(&parts), Ok(()));
}

#[test]
fn gate_14_rejects_inconsistent_types() {
    let mut parts = make_parts(
        vec![
            set_const_node(0, 0, 0),
            set_const_node(1, 1, 0),
            finish_node(2, 0),
        ],
        1,
    );
    parts.constants = Box::new([ConstValue::I64(1), ConstValue::Bool(true)]);
    assert!(matches!(
        validate_gate_14_slot_type_consistency(&parts),
        Err(ValidationError::SlotTypeInconsistency { .. })
    ));
}

#[test]
fn gate_14_accepts_single_set_const() {
    let mut parts = make_parts(vec![set_const_node(0, 0, 0)], 1);
    parts.constants = Box::new([ConstValue::I64(42)]);
    assert_eq!(validate_gate_14_slot_type_consistency(&parts), Ok(()));
}

#[test]
fn gate_14_accepts_different_slots_with_different_types() {
    let mut parts = make_parts(
        vec![
            set_const_node(0, 0, 0),
            set_const_node(1, 1, 1),
            finish_node(2, 0),
        ],
        2,
    );
    parts.constants = Box::new([ConstValue::I64(1), ConstValue::Bool(false)]);
    assert_eq!(validate_gate_14_slot_type_consistency(&parts), Ok(()));
}

// ===== Gate 15 tests =====

#[test]
fn gate_15_accepts_deterministic_workflow() {
    let parts = make_parts(vec![finish_node(0, 0)], 1);
    assert_eq!(validate_gate_15_determinism_proof(&parts), Ok(()));
}

#[test]
fn gate_15_accepts_do_followed_by_deterministic() {
    let parts = make_parts(vec![do_node(0, 1, 0), finish_node(1, 0)], 1);
    assert_eq!(validate_gate_15_determinism_proof(&parts), Ok(()));
}

#[test]
fn gate_15_rejects_do_followed_by_do() {
    let parts = make_parts(
        vec![do_node(0, 1, 0), do_node(1, 2, 0), finish_node(2, 0)],
        1,
    );
    assert!(matches!(
        validate_gate_15_determinism_proof(&parts),
        Err(ValidationError::NonDeterministicPath { .. })
    ));
}

#[test]
fn gate_15_rejects_ask_followed_by_do() {
    let ask_node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Ask {
            prompt: SlotIdx::new(0),
            timeout_slot: None,
        },
    };
    let parts = make_parts(vec![ask_node, do_node(1, 1, 0), finish_node(2, 0)], 1);
    assert!(matches!(
        validate_gate_15_determinism_proof(&parts),
        Err(ValidationError::NonDeterministicPath { .. })
    ));
}

#[test]
fn gate_15_accepts_do_with_no_next() {
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(1),
            input: SlotIdx::new(0),
        },
    };
    let parts = make_parts(vec![node], 1);
    assert_eq!(validate_gate_15_determinism_proof(&parts), Ok(()));
}

#[test]
fn gate_15_accepts_nop_workflow() {
    let parts = make_parts(vec![nop_node(0)], 0);
    assert_eq!(validate_gate_15_determinism_proof(&parts), Ok(()));
}

#[test]
fn gate_15_accepts_do_followed_by_nop_then_do() {
    // Do -> Nop -> Do is OK because the Nop separates them
    let parts = make_parts(
        vec![
            do_node(0, 1, 0),
            nop_node(1),
            do_node(2, 2, 0),
            finish_node(3, 0),
        ],
        1,
    );
    assert_eq!(validate_gate_15_determinism_proof(&parts), Ok(()));
}
