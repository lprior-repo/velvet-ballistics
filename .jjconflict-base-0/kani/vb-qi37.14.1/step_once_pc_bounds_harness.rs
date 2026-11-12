// VB-INV004-KANI: PC bounds invariant
//
// Claim: The PC after step_once is always within [0, step_count).
// Bound: step_count ∈ [1, 16]

#![forbid(unsafe_code)]

use vb_core::engine::step_once;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, StepIdx};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledWorkflow, WorkflowParts};

#[kani::proof]
#[kani::unwind(6)]
fn step_once_pc_bounds_harness() {
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

    let pc = run.pc();
    let pc_usize: usize = pc.into();
    kani::assert(
        pc_usize < step_count as usize,
        "PC < step_count after step_once",
    );
}
