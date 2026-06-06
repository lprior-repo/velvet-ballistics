//! Property-based tests for workflow validation.

use super::super::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstValue, ResourceContract, WorkflowError,
    WorkflowParts,
};
use super::tests::resource_contract;
use crate::frame::{StepState, is_valid_step_state_transition};
use crate::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use proptest::prelude::*;

fn arb_step_state() -> impl Strategy<Value = StepState> {
    prop_oneof![
        Just(StepState::Pending),
        Just(StepState::Running),
        Just(StepState::Succeeded),
        Just(StepState::Failed),
        Just(StepState::Skipped),
        Just(StepState::Waiting),
        Just(StepState::Asking),
        Just(StepState::Cancelled),
    ]
}

fn is_terminal(s: StepState) -> bool {
    matches!(
        s,
        StepState::Succeeded | StepState::Failed | StepState::Skipped | StepState::Cancelled
    )
}

proptest! {
    // -- PO-PROP-001: Terminal transition invariant -------------------------
    // All terminal states (Succeeded, Failed, Cancelled, Skipped) are fully
    // absorbing. No non-self transitions are allowed. Loop body reentry uses
    // the explicit Succeeded->Pending admission path in RunFrame::mark_pending
    // before mark_running; the direct Succeeded->Running edge is invalid.
    #[test]
    fn proptest_terminal_absorption_invariant(
        from in arb_step_state(),
        to in arb_step_state(),
    ) {
        prop_assume!(is_terminal(from));
        prop_assume!(from != to);
        let result = is_valid_step_state_transition(from, to);
        prop_assert!(
            !result,
            "terminal {:?}->{:?} must be invalid (terminal states are fully absorbing)",
            from, to
        );
    }

    /// Succeeded must not transition directly to Running.
    /// Loop reentry uses the explicit mark_pending admission path.
    #[test]
    fn proptest_succeeded_to_running_rejected(_seed in any::<u64>()) {
        let result = is_valid_step_state_transition(StepState::Succeeded, StepState::Running);
        prop_assert!(
            !result,
            "Succeeded->Running must be invalid (master: no terminal->running edge)"
        );
    }
}

proptest! {
    #[test]
    fn resource_contract_max_steps_is_positive(_unused in 0u8..1u8) {
        let contract = ResourceContract::DEFAULT;
        prop_assert!(contract.max_steps > 0);
    }
}

proptest! {
    #[test]
    fn resource_contract_max_slots_is_positive(_unused in 0u8..1u8) {
        let contract = ResourceContract::DEFAULT;
        prop_assert!(contract.max_slots > 0);
    }
}

// =========================================================================
// Property A: Valid minimal workflow always passes validation
//
// Generate random (but structurally valid) workflows with 2-10 steps,
// each forming a SetConst -> ... -> Finish chain.
// =========================================================================

/// Builds a valid linear workflow with `step_count` nodes.
/// Nodes 0..N-2 are SetConst, node N-1 is Finish.
/// slot_count = 1 (slot 0 is used throughout).
fn build_valid_chain(step_count: usize) -> WorkflowParts {
    let last = step_count.saturating_sub(1);
    let mut nodes: Vec<CompiledNode> = (0..last)
        .map(|i| {
            let next_step = u16::try_from(i.saturating_add(1)).map_or(u16::MAX, |v| v);
            CompiledNode {
                id: StepIdx::new(u16::try_from(i).map_or(u16::MAX, |v| v)),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(next_step)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            }
        })
        .collect();
    nodes.push(CompiledNode {
        id: StepIdx::new(u16::try_from(last).map_or(u16::MAX, |v| v)),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    });
    let max_steps = u16::try_from(nodes.len()).map_or(u16::MAX, |v| v);
    WorkflowParts {
        name: Box::<str>::from("proptest_valid_chain"),
        digest: WorkflowDigest::from_bytes([0xAA; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(max_steps, 1, 1, 0, 0),
        step_names: Box::new([]),
    }
}

fn chain_node(index: usize, total: usize) -> CompiledNode {
    let is_last = index == total.saturating_sub(1);
    let next = if is_last {
        None
    } else {
        Some(StepIdx::new(
            u16::try_from(index.saturating_add(1)).map_or(u16::MAX, |v| v),
        ))
    };
    let kind = if is_last {
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        }
    } else {
        CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        }
    };
    CompiledNode {
        id: StepIdx::new(u16::try_from(index).map_or(u16::MAX, |v| v)),
        output: Some(SlotIdx::new(0)),
        next,
        on_error: None,
        error_slot: None,
        kind,
    }
}

fn duplicate_step_node(index: usize, total: usize, duplicate_position: usize) -> CompiledNode {
    let claimed_id = if index == duplicate_position {
        StepIdx::new(0)
    } else {
        StepIdx::new(u16::try_from(index).map_or(u16::MAX, |v| v))
    };
    CompiledNode {
        id: claimed_id,
        ..chain_node(index, total)
    }
}

fn unreachable_finish_node(index: usize) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(u16::try_from(index).map_or(u16::MAX, |v| v)),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    }
}

proptest! {
    #[test]
    fn prop_a_valid_chain_workflow_passes_validation(step_count in 2u16..10u16) {
        let parts = build_valid_chain(usize::from(step_count));
        let result = CompiledWorkflow::try_from_parts(parts);
        prop_assert!(
            result.is_ok(),
            "valid chain with {} steps should pass validation, got {:?}",
            step_count,
            result
        );
    }
}

// =========================================================================
// Property B: SlotIdx out of bounds always rejected
//
// Generate workflows where Finish or SetConst references a slot >= slot_count.
// =========================================================================

proptest! {
    #[test]
    fn prop_b_finish_slot_out_of_bounds_rejected(
        slot_count in 1u16..10u16,
        bad_slot_delta in 1u16..50u16
    ) {
        let bad_slot = slot_count.saturating_add(bad_slot_delta);
        let parts = WorkflowParts {
            name: Box::<str>::from("prop_b_finish_oob"),
            digest: WorkflowDigest::from_bytes([0xBB; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(bad_slot),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(1, slot_count, 1, 0, 0),
            step_names: Box::new([]),
        };
        let result = CompiledWorkflow::try_from_parts(parts);
        match result {
            Err(WorkflowError::SlotOutOfBounds { slot }) => {
                prop_assert_eq!(slot, SlotIdx::new(bad_slot));
            }
            other => {
                return Err(proptest::test_runner::TestCaseError::Fail(
                    format!("expected SlotOutOfBounds, got {:?}", other).into()
                ));
            }
        }
    }
}

proptest! {
    #[test]
    fn prop_b_setconst_output_slot_out_of_bounds_rejected(
        slot_count in 1u16..10u16,
        bad_slot_delta in 1u16..50u16
    ) {
        let bad_slot = slot_count.saturating_add(bad_slot_delta);
        let parts = WorkflowParts {
            name: Box::<str>::from("prop_b_output_oob"),
            digest: WorkflowDigest::from_bytes([0xBC; 32]),
            nodes: vec![
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(bad_slot)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(2, slot_count, 1, 0, 0),
            step_names: Box::new([]),
        };
        let result = CompiledWorkflow::try_from_parts(parts);
        match result {
            Err(WorkflowError::SlotOutOfBounds { slot }) => {
                prop_assert_eq!(slot, SlotIdx::new(bad_slot));
            }
            other => {
                return Err(proptest::test_runner::TestCaseError::Fail(
                    format!("expected SlotOutOfBounds, got {:?}", other).into()
                ));
            }
        }
    }
}

// =========================================================================
// Property C: Duplicate StepIdx always rejected
//
// Generate workflows with two nodes claiming the same StepIdx.
// =========================================================================

proptest! {
    #[test]
    fn prop_c_duplicate_step_idx_rejected(
        step_count in 3u16..10u16,
        duplicate_id_pos in 1u16..9u16
    ) {
        let n = usize::from(step_count);
        let dup_pos = usize::from(duplicate_id_pos.min(step_count.saturating_sub(1)));
        let nodes = (0..n)
            .map(|index| duplicate_step_node(index, n, dup_pos))
            .collect::<Vec<_>>();
        let max_steps = u16::try_from(n).map_or(u16::MAX, |v| v);
        let parts = WorkflowParts {
            name: Box::<str>::from("prop_c_dup"),
            digest: WorkflowDigest::from_bytes([0xCC; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(max_steps, 1, 1, 0, 0),
            step_names: Box::new([]),
        };
        let result = CompiledWorkflow::try_from_parts(parts);
        match result {
            Err(WorkflowError::NodeIdMismatch { expected, actual }) => {
                prop_assert_eq!(expected, StepIdx::new(u16::try_from(dup_pos).map_or(u16::MAX, |v| v)));
                prop_assert_eq!(actual, StepIdx::new(0));
            }
            other => {
                return Err(proptest::test_runner::TestCaseError::Fail(
                    format!("expected NodeIdMismatch, got {:?}", other).into()
                ));
            }
        }
    }
}

// =========================================================================
// Property D: Unreachable nodes always rejected
//
// Generate workflows where an extra node exists but no other node points to it.
// =========================================================================

proptest! {
    #[test]
    fn prop_d_unreachable_node_rejected(
        chain_len in 2u16..8u16,
        unreachable_count in 1u16..3u16
    ) {
        let chain_n = usize::from(chain_len);
        let extra_n = usize::from(unreachable_count);
        let total = chain_n.saturating_add(extra_n);
        let nodes = (0..chain_n)
            .map(|index| chain_node(index, chain_n))
            .chain((chain_n..total).map(unreachable_finish_node))
            .collect::<Vec<_>>();

        let max_steps = u16::try_from(total).map_or(u16::MAX, |v| v);
        let parts = WorkflowParts {
            name: Box::<str>::from("prop_d_unreachable"),
            digest: WorkflowDigest::from_bytes([0xDD; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(max_steps, 1, 1, 0, 0),
            step_names: Box::new([]),
        };
        let result = CompiledWorkflow::try_from_parts(parts);
        match result {
            Err(WorkflowError::UnreachableNode { step }) => {
                // The first unreachable node should be at index chain_len.
                prop_assert_eq!(step, StepIdx::new(u16::try_from(chain_n).map_or(u16::MAX, |v| v)));
            }
            other => {
                return Err(proptest::test_runner::TestCaseError::Fail(
                    format!("expected UnreachableNode, got {:?}", other).into()
                ));
            }
        }
    }
}

// =========================================================================
// Property E: Resource contract bounds respected
//
// Workflows with step_count > max_steps fail with ResourceContractExceeded.
// =========================================================================

proptest! {
    #[test]
    fn prop_e_resource_contract_max_steps_violated(
        actual_steps in 2u16..10u16,
        shortfall in 1u16..5u16
    ) {
        let max_steps_declared = actual_steps.saturating_sub(shortfall);
        // Build a valid chain but with a contract that doesn't cover it.
        let valid_parts = build_valid_chain(usize::from(actual_steps));
        let parts = WorkflowParts {
            resource_contract: resource_contract(max_steps_declared, 1, 1, 0, 0),
            step_names: Box::new([]),
            ..valid_parts
        };
        let result = CompiledWorkflow::try_from_parts(parts);
        match result {
            Err(WorkflowError::ResourceContractExceeded { resource }) => {
                prop_assert_eq!(resource, "max_steps");
            }
            other => {
                return Err(proptest::test_runner::TestCaseError::Fail(
                    format!("expected ResourceContractExceeded for max_steps, got {:?}", other).into()
                ));
            }
        }
    }
}

proptest! {
    #[test]
    fn prop_e_resource_contract_max_slots_violated(
        actual_slots in 1u16..10u16,
        shortfall in 1u16..5u16
    ) {
        let declared_slots = actual_slots.saturating_sub(shortfall);
        // Single node that uses a slot at the boundary.
        let parts = WorkflowParts {
            name: Box::<str>::from("prop_e_slots"),
            digest: WorkflowDigest::from_bytes([0xEE; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: actual_slots,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(1, declared_slots, 1, 0, 0),
            step_names: Box::new([]),
        };
        let result = CompiledWorkflow::try_from_parts(parts);
        match result {
            Err(WorkflowError::ResourceContractExceeded { resource }) => {
                prop_assert_eq!(resource, "max_slots");
            }
            other => {
                return Err(proptest::test_runner::TestCaseError::Fail(
                    format!("expected ResourceContractExceeded for max_slots, got {:?}", other).into()
                ));
            }
        }
    }
}

// =========================================================================
// Phase 45 tests — ResourceContract default values
// =========================================================================

#[test]
fn resource_contract_default_has_reasonable_max_steps() {
    assert_eq!(ResourceContract::DEFAULT.max_steps, 1_000);
}

#[test]
fn resource_contract_default_has_reasonable_max_slots() {
    assert_eq!(ResourceContract::DEFAULT.max_slots, 1_024);
}

#[test]
fn resource_contract_default_has_reasonable_max_fanout() {
    assert_eq!(ResourceContract::DEFAULT.max_fanout, 64);
}

#[test]
fn resource_contract_default_has_reasonable_step_budget_per_tick() {
    assert_eq!(ResourceContract::DEFAULT.max_step_budget_per_tick, 10_000);
}

#[test]
fn resource_contract_default_max_steps_is_not_u16_max() {
    assert_ne!(ResourceContract::DEFAULT.max_steps, u16::MAX);
}

#[test]
fn resource_contract_default_max_slots_is_not_u16_max() {
    assert_ne!(ResourceContract::DEFAULT.max_slots, u16::MAX);
}

#[test]
fn resource_contract_default_max_fanout_is_not_u16_max() {
    assert_ne!(ResourceContract::DEFAULT.max_fanout, u16::MAX);
}

#[test]
fn resource_contract_default_max_retry_attempts_is_not_u16_max() {
    assert_ne!(ResourceContract::DEFAULT.max_retry_attempts, u16::MAX);
}
