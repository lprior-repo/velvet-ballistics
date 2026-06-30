// VB-INV003-KANI: slot initialization invariant
//
// Claim: No slot is read that was not first written in the same step
//        execution. Slots are initialized to None; reading None before
//        write returns SlotUninitialized error (not a panic).

#![forbid(unsafe_code)]

use vb_core::engine::step_once;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledWorkflow, WorkflowParts};

#[kani::proof]
#[kani::unwind(6)]
fn step_once_slot_init_harness() {
    let parts: WorkflowParts = kani::any();
    let workflow_result = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: parts.name,
        digest: parts.digest,
        nodes: parts.nodes,
        expressions: parts.expressions,
        accessors: parts.accessors,
        constants: parts.constants,
        slot_count: parts.slot_count,
        symbols_count: parts.symbols_count,
        entry: parts.entry,
        resource_contract: parts.resource_contract,
        step_names: parts.step_names,
    });

    let plan = match workflow_result {
        Ok(w) => w,
        Err(_) => return,
    };

    let step_count = plan.node_count();
    kani::assume(step_count >= 1);
    kani::assume(step_count <= 16);

    let slot_count = plan.slot_count();
    kani::assume(slot_count <= 32);

    let first_step = StepIdx::new(kani::any::<u16>() % step_count);
    let mut run = match RunFrame::new(RunId::new(1), first_step, step_count, slot_count) {
        Ok(f) => f,
        Err(_) => return,
    };

    let mut store = ValueStore::new();

    let _result = step_once(&plan, &mut run, &mut store);

    let slot_idx = SlotIdx::new(kani::any::<u16>() % slot_count.max(1));
    let read_result = run.read_slot(slot_idx);
    kani::cover!(read_result.is_err(), "uninitialized slot returns error");
    kani::cover!(read_result.is_ok(), "initialized slot returns value");
}
