//! PS-006 Kani harness: Slot validation for timer nodes (POB-vb-fzgdn-024)
//! Binds to: crate::shard::helpers::timer_registration_required
#![forbid(unsafe_code)]

use vb_core::ids::{ActionId, SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};

fn make_wf_with_node(node: CompiledNode) -> CompiledWorkflow {
    let mut parts: WorkflowParts = kani::any();
    parts.nodes = vec![node].into_boxed_slice();
    match CompiledWorkflow::try_from_parts(parts) {
        Ok(v) => v,
        Err(_) => {
            kani::assume(false, "valid workflow");
            return;
        }
    }
}

fn make_state(wf: CompiledWorkflow) -> vb_runtime::shard::RunState {
    let frame = match vb_core::frame::RunFrame::new(
        vb_core::ids::RunId::new(1), StepIdx::ZERO, 1, 1,
    ) {
        Ok(frame) => frame,
        Err(_) => {
            kani::assume(false, "valid frame");
            return;
        }
    };
    vb_runtime::shard::RunState {
        frame,
        workflow: wf,
        store: vb_core::value_store::ValueStore::new(),
        action_attempts: vec![0u16; 1].into_boxed_slice(),
        admission: None,
        collect_states: vb_runtime::primitives::collect::CollectStates::new(),
        action_contracts: Box::new([]),
    }
}

#[kani::proof]
fn ps_006_timer_required_for_wait_until() {
    // Symbolic witness: `deadline_slot` is restricted to ZERO so the
    // harness exercises the precise WaitUntil-requires-timer
    // boundary.
    let deadline_slot: u16 = kani::any();
    kani::assume(deadline_slot == 0);
    let node = CompiledNode {
        id: StepIdx::ZERO, output: None, next: None, on_error: None, error_slot: None,
        kind: CompiledNodeKind::WaitUntil { deadline_slot: SlotIdx::new(deadline_slot) },
    };
    let wf = make_wf_with_node(node);
    let state = make_state(wf);
    assert!(vb_runtime::shard::helpers::timer_registration_required(&state, StepIdx::ZERO));
}

#[kani::proof]
fn ps_006_timer_not_required_for_do() {
    // Symbolic witness: `action_id` is restricted to 0 and `input`
    // slot to 0 so the harness exercises the precise Do-no-timer
    // boundary.
    let action_id: u32 = kani::any();
    kani::assume(action_id == 0);
    let input_slot: u16 = kani::any();
    kani::assume(input_slot == 0);
    let node = CompiledNode {
        id: StepIdx::ZERO, output: None, next: None, on_error: None, error_slot: None,
        kind: CompiledNodeKind::Do { action: ActionId::new(action_id), input: SlotIdx::new(input_slot) },
    };
    let wf = make_wf_with_node(node);
    let state = make_state(wf);
    assert!(!vb_runtime::shard::helpers::timer_registration_required(&state, StepIdx::ZERO));
}

#[kani::proof]
fn ps_006_timer_not_required_for_missing_step() {
    // Symbolic witness: step is restricted to a value (99) outside
    // the workflow's compiled step range so the harness exercises
    // the missing-step boundary.
    let missing_step: u16 = kani::any();
    kani::assume(missing_step == 99);
    let node = CompiledNode {
        id: StepIdx::ZERO, output: None, next: None, on_error: None, error_slot: None,
        kind: CompiledNodeKind::Do { action: ActionId::new(0), input: SlotIdx::ZERO },
    };
    let wf = make_wf_with_node(node);
    let state = make_state(wf);
    assert!(!vb_runtime::shard::helpers::timer_registration_required(&state, StepIdx::new(missing_step)));
}
