//! PS-006 cargo-fuzz target: Slot validation fuzzing (POB-vb-fzgdn-027)
//! Production binding: crates/vb_runtime/src/shard/helpers.rs timer_registration_required
//!
//! Fuzzes the timer_registration_required function with arbitrary node kinds
//! and step indices. The function must never panic for any valid or invalid input.

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts};
use vb_core::frame::RunFrame;
use vb_runtime::shard::RunState;
use vb_runtime::shard::helpers::timer_registration_required;

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }

    let has_timeout = data[0] % 2 == 0;
    let node_kind_byte = data[0] % 5;
    let step_idx = u16::from(data[1]).min(99);

    let node_kind = match node_kind_byte {
        0 => CompiledNodeKind::WaitUntil { deadline_slot: SlotIdx::ZERO },
        1 => CompiledNodeKind::WaitEvent { event: SlotIdx::ZERO, timeout_slot: if has_timeout { Some(SlotIdx::new(1)) } else { None } },
        2 => CompiledNodeKind::Ask { ask_slot: SlotIdx::ZERO, answer_slot: SlotIdx::new(1), resume_step: StepIdx::new(2), timeout_slot: if has_timeout { Some(SlotIdx::new(2)) } else { None } },
        3 => CompiledNodeKind::Do { action: ActionId::new(0), input: SlotIdx::ZERO },
        _ => CompiledNodeKind::Finish { result: SlotIdx::ZERO },
    };

    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: node_kind,
    };

    let parts = WorkflowParts {
        name: Box::from("fuzz_wf"),
        digest: WorkflowDigest::from_bytes([0xAB; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };

    let Ok(wf) = CompiledWorkflow::try_from_parts(parts) else { return; };
    let Ok(frame) = RunFrame::new(vb_core::ids::RunId::new(1), StepIdx::ZERO, 1, 3) else { return; };

    let state = RunState {
        frame,
        workflow: wf,
        store: vb_core::value_store::ValueStore::new(),
        action_attempts: vec![0u16; 1].into_boxed_slice(),
        admission: None,
        collect_states: vb_runtime::primitives::collect::CollectStates::new(),
        action_contracts: Box::new([]),
    };

    // Must never panic for any input
    let _result = timer_registration_required(&state, StepIdx::new(step_idx));
});
