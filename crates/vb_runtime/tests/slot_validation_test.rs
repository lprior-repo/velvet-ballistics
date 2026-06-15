#![forbid(unsafe_code)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::expect_used,
    clippy::get_first,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
//! PS-006: Slot Validation — behavior tests (F1-F4).
//!
//! Tests the `timer_registration_required` helper which determines whether
//! a given workflow step requires timer registration.
//!
//! Uses `CompiledWorkflow::try_from_parts` to construct minimal workflows
//! with various node kinds (WaitUntil, WaitEvent, Ask, Do, Finish) and
//! validates that timer registration is correctly identified.

use vb_core::frame::RunFrame;
use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::primitives::collect::CollectStates;
use vb_runtime::shard::helpers::timer_registration_required;
use vb_runtime::shard::types::RunState;

fn new_action_attempts(count: u16) -> Box<[u16]> {
    vec![0u16; usize::from(count)].into_boxed_slice()
}

fn make_run_state(wf: CompiledWorkflow) -> RunState {
    let step_count = wf.node_count();
    let slot_count = wf.slot_count();
    let frame = RunFrame::new(
        vb_core::ids::RunId::new(1),
        wf.entry(),
        step_count,
        slot_count,
    )
    .expect("failed to create RunFrame");
    RunState {
        frame,
        workflow: wf,
        store: ValueStore::new(),
        action_attempts: new_action_attempts(step_count),
        admission: None,
        collect_states: CollectStates::new(),
        action_contracts: Box::new([]),
        last_snapshot_executed: 0,
    }
}

fn make_wf_with_kind(kind: CompiledNodeKind, slot_count: u16) -> Option<CompiledWorkflow> {
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind,
    };
    let parts = WorkflowParts {
        name: Box::from("test_wf"),
        digest: WorkflowDigest::from_bytes([0xAA; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

// ---------- Behavior F1: WaitUntil requires timer registration ----------

#[test]
fn wait_until_node_requires_timer_registration() {
    let wf = make_wf_with_kind(
        CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::ZERO,
        },
        1,
    )
    .expect("WaitUntil workflow");
    let state = make_run_state(wf);
    assert!(timer_registration_required(&state, StepIdx::ZERO));
}

// ---------- Behavior F2: WaitEvent with timeout requires timer ----------

#[test]
fn wait_event_with_timeout_requires_timer_registration() {
    let wf = make_wf_with_kind(
        CompiledNodeKind::WaitEvent {
            event: SlotIdx::ZERO,
            timeout_slot: Some(SlotIdx::new(1)),
        },
        2,
    )
    .expect("WaitEvent+timeout workflow");
    let state = make_run_state(wf);
    assert!(timer_registration_required(&state, StepIdx::ZERO));
}

#[test]
fn wait_event_without_timeout_does_not_require_timer_registration() {
    let wf = make_wf_with_kind(
        CompiledNodeKind::WaitEvent {
            event: SlotIdx::ZERO,
            timeout_slot: None,
        },
        1,
    )
    .expect("WaitEvent no-timeout workflow");
    let state = make_run_state(wf);
    assert!(!timer_registration_required(&state, StepIdx::ZERO));
}

// ---------- Behavior F3: Ask with timeout requires timer ----------

#[test]
fn ask_with_timeout_requires_timer_registration() {
    let wf = make_wf_with_kind(
        CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: Some(SlotIdx::new(1)),
        },
        2,
    )
    .expect("Ask+timeout workflow");
    let state = make_run_state(wf);
    assert!(timer_registration_required(&state, StepIdx::ZERO));
}

#[test]
fn ask_without_timeout_does_not_require_timer_registration() {
    let wf = make_wf_with_kind(
        CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: None,
        },
        1,
    )
    .expect("Ask no-timeout workflow");
    let state = make_run_state(wf);
    assert!(!timer_registration_required(&state, StepIdx::ZERO));
}

// ---------- Behavior F4: Non-timer nodes do not require registration ----------

#[test]
fn do_node_does_not_require_timer_registration() {
    let wf = make_wf_with_kind(
        CompiledNodeKind::Do {
            action: ActionId::new(0),
            input: SlotIdx::ZERO,
        },
        1,
    )
    .expect("Do workflow");
    let state = make_run_state(wf);
    assert!(!timer_registration_required(&state, StepIdx::ZERO));
}

#[test]
fn finish_node_does_not_require_timer_registration() {
    let wf = make_wf_with_kind(
        CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
        1,
    )
    .expect("Finish workflow");
    let state = make_run_state(wf);
    assert!(!timer_registration_required(&state, StepIdx::ZERO));
}

// ---------- Out-of-bounds step ----------

#[test]
fn timer_registration_required_returns_false_for_out_of_bounds_step() {
    let wf = make_wf_with_kind(
        CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::ZERO,
        },
        1,
    )
    .expect("WaitUntil workflow");
    let state = make_run_state(wf);
    // Step 99 doesn't exist in the 1-node workflow
    assert!(!timer_registration_required(&state, StepIdx::new(99)));
}
