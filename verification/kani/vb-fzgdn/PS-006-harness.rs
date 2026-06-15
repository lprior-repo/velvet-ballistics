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
    let node = CompiledNode {
        id: StepIdx::ZERO, output: None, next: None, on_error: None, error_slot: None,
        kind: CompiledNodeKind::WaitUntil { deadline_slot: SlotIdx::ZERO },
    };
    let wf = make_wf_with_node(node);
    let state = make_state(wf);
    assert!(vb_runtime::shard::helpers::timer_registration_required(&state, StepIdx::ZERO));
}

#[kani::proof]
fn ps_006_timer_not_required_for_do() {
    let node = CompiledNode {
        id: StepIdx::ZERO, output: None, next: None, on_error: None, error_slot: None,
        kind: CompiledNodeKind::Do { action: ActionId::new(0), input: SlotIdx::ZERO },
    };
    let wf = make_wf_with_node(node);
    let state = make_state(wf);
    assert!(!vb_runtime::shard::helpers::timer_registration_required(&state, StepIdx::ZERO));
}

#[kani::proof]
fn ps_006_timer_not_required_for_missing_step() {
    let node = CompiledNode {
        id: StepIdx::ZERO, output: None, next: None, on_error: None, error_slot: None,
        kind: CompiledNodeKind::Do { action: ActionId::new(0), input: SlotIdx::ZERO },
    };
    let wf = make_wf_with_node(node);
    let state = make_state(wf);
    assert!(!vb_runtime::shard::helpers::timer_registration_required(&state, StepIdx::new(99)));
}
