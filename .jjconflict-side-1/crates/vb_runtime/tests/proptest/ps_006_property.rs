//! PS-006 proptest: Slot validation for timer nodes (POB-vb-fzgdn-026)
//! Production binding: crates/vb_runtime/src/shard/helpers.rs timer_registration_required
//!
//! Property: timer_registration_required returns correct bool for various node types.

use proptest::prelude::*;
use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts};
use vb_core::frame::RunFrame;
use vb_runtime::shard::RunState;
use vb_runtime::shard::helpers::timer_registration_required;

fn wait_until_wf() -> CompiledWorkflow {
    let node = CompiledNode {
        id: StepIdx::ZERO, output: None, next: None, on_error: None, error_slot: None,
        kind: CompiledNodeKind::WaitUntil { deadline_slot: SlotIdx::ZERO },
    };
    let parts = WorkflowParts {
        name: Box::from("wait"), digest: WorkflowDigest::from_bytes([0xEE; 32]),
        nodes: Box::from([node]), expressions: Box::from([]), accessors: Box::from([]),
        constants: Box::from([]), slot_count: 1, symbols_count: 0, entry: StepIdx::ZERO,
        step_names: Box::from([]), resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).expect("wf")
}

fn do_wf() -> CompiledWorkflow {
    let node = CompiledNode {
        id: StepIdx::ZERO, output: None, next: None, on_error: None, error_slot: None,
        kind: CompiledNodeKind::Do { action: ActionId::new(0), input: SlotIdx::ZERO },
    };
    let parts = WorkflowParts {
        name: Box::from("do"), digest: WorkflowDigest::from_bytes([0xFF; 32]),
        nodes: Box::from([node]), expressions: Box::from([]), accessors: Box::from([]),
        constants: Box::from([]), slot_count: 1, symbols_count: 0, entry: StepIdx::ZERO,
        step_names: Box::from([]), resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).expect("wf")
}

fn make_state(wf: CompiledWorkflow) -> RunState {
    let frame = RunFrame::new(vb_core::ids::RunId::new(1), StepIdx::ZERO, 1, 1).expect("frame");
    RunState {
        frame, workflow: wf, store: vb_core::value_store::ValueStore::new(),
        action_attempts: vec![0u16; 1].into_boxed_slice(), admission: None,
        collect_states: vb_runtime::primitives::collect::CollectStates::new(),
        action_contracts: Box::new([]),
    }
}

proptest! {
    #[test]
    fn ps_006_wait_until_always_requires_timer() {
        let wf = wait_until_wf();
        let state = make_state(wf);
        prop_assert!(timer_registration_required(&state, StepIdx::ZERO));
    }

    #[test]
    fn ps_006_do_never_requires_timer() {
        let wf = do_wf();
        let state = make_state(wf);
        prop_assert!(!timer_registration_required(&state, StepIdx::ZERO));
    }

    #[test]
    fn ps_006_missing_step_no_timer(
        step in 50u16..100,
    ) {
        let wf = do_wf();
        let state = make_state(wf);
        prop_assert!(!timer_registration_required(&state, StepIdx::new(step)));
    }
}
