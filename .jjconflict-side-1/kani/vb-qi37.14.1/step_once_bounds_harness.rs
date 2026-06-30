// VB-PRE002-KANI / VB-INV004-KANI: step_once panic freedom + PC bounds
//
// Claim: step_once never panics for any valid CompiledWorkflow, RunFrame,
//        ValueStore inputs within the bounded model.
// Bounds: step_count ∈ [1, 16], slot_count ∈ [0, 32].
// Assert: PC ∈ [0, step_count) after step_once returns Ok.
// Cover: each EngineSignal variant is reachable.

#![forbid(unsafe_code)]

use vb_core::errors::EngineError;
use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledWorkflow, WorkflowParts};
use vb_core::EngineSignal;
use vb_core::engine::step_once;

impl kani::Arbitrary for EngineSignal {
    fn any() -> Self {
        match kani::any::<u8>() % 6 {
            0 => EngineSignal::Continue,
            1 => EngineSignal::Finished(kani::any::<SlotValue>(), kani::any::<Taint>()),
            2 => EngineSignal::StepBudgetExhausted,
            3 => EngineSignal::AwaitingAction,
            4 => EngineSignal::AwaitingWait,
            _ => EngineSignal::AwaitingAsk,
        }
    }
}

#[kani::proof]
#[kani::unwind(6)]
fn step_once_bounds_harness() {
    let parts: WorkflowParts = kani::any();
    let node_count: u8 = kani::any();
    kani::assume(node_count >= 1);
    kani::assume(node_count <= 16);

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

    let first_step_raw = kani::any::<u16>();
    let first_step = StepIdx::new(first_step_raw % step_count);

    let mut run = match RunFrame::new(RunId::new(1), first_step, step_count, slot_count) {
        Ok(f) => f,
        Err(_) => return,
    };

    let mut store = ValueStore::new();

    let result = step_once(&plan, &mut run, &mut store);

    let pc = run.pc();
    kani::assert(
        pc.as_usize() < usize::from(step_count),
        "PC in bounds after step_once",
    );

    match &result {
        Ok(signal) => {
            kani::cover!(matches!(signal, EngineSignal::Continue), "Continue reachable");
            kani::cover!(matches!(signal, EngineSignal::Finished(_, _)), "Finished reachable");
            kani::cover!(matches!(signal, EngineSignal::StepBudgetExhausted), "StepBudgetExhausted reachable");
            kani::cover!(matches!(signal, EngineSignal::AwaitingAction), "AwaitingAction reachable");
            kani::cover!(matches!(signal, EngineSignal::AwaitingWait), "AwaitingWait reachable");
            kani::cover!(matches!(signal, EngineSignal::AwaitingAsk), "AwaitingAsk reachable");
        }
        Err(_) => {
            kani::cover!(true, "Err path reachable");
        }
    }
}