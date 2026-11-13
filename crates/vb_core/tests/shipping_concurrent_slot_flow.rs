#[path = "shipping_concurrent_slot_flow/fixtures.rs"]
mod fixtures;

use fixtures::{fixed, foreach};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{
    CompiledNodeKind as K, InputSlotKind, SlotFlowError, SlotFlowFailure, WorkflowParts,
    validate_slot_flow,
};

fn assert_error(parts: WorkflowParts, step: u16, slot: Option<u16>, reason: SlotFlowFailure) {
    assert_eq!(
        validate_slot_flow(&parts),
        Err(SlotFlowError {
            step: StepIdx::new(step),
            slot: slot.map(SlotIdx::new),
            reason
        })
    );
}

#[test]
fn foreach_item_is_definitely_assigned() -> Result<(), SlotFlowError> {
    validate_slot_flow(&foreach(3, 3))
}

#[test]
fn parent_cannot_read_child_local() {
    assert_error(
        foreach(3, 2),
        4,
        Some(2),
        SlotFlowFailure::UninitializedRead,
    );
}

#[test]
fn sibling_cannot_read_other_lane_write() {
    assert_error(
        fixed(false, 1),
        3,
        Some(1),
        SlotFlowFailure::UninitializedRead,
    );
}

#[test]
fn captured_input_is_available_in_each_lane() -> Result<(), SlotFlowError> {
    validate_slot_flow(&fixed(false, 0))
}

#[test]
fn known_non_list_foreach_input_rejected() {
    let mut p = foreach(3, 3);
    if let Some(input) = p.input_slots.first_mut() {
        input.kind = InputSlotKind::I64;
    }
    assert_error(p, 0, None, SlotFlowFailure::IncompatibleType);
}

#[test]
fn zero_iterations_still_assigns_list_join() -> Result<(), SlotFlowError> {
    validate_slot_flow(&foreach(0, 3))
}

#[test]
fn first_success_observes_successful_result() -> Result<(), SlotFlowError> {
    validate_slot_flow(&fixed(true, 0))
}

#[test]
fn declared_lane_result_must_be_definite() {
    let mut p = foreach(3, 3);
    if let Some(child) = p.nodes.get_mut(1) {
        child.kind = K::Nop;
        child.output = None.into();
    }
    assert_error(p, 2, Some(2), SlotFlowFailure::UninitializedRead);
}

#[path = "shipping_concurrent_slot_flow/types.rs"]
mod types;

#[path = "shipping_concurrent_slot_flow/loops.rs"]
mod loops;

#[path = "shipping_concurrent_slot_flow/paths.rs"]
mod paths;

#[path = "shipping_concurrent_slot_flow/compositions.rs"]
mod compositions;

#[path = "shipping_concurrent_slot_flow/recovery.rs"]
mod recovery;
