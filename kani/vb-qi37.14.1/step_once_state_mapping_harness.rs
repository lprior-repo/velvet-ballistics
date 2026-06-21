// VB-INV002-KANI: EngineSignal→StepState mapping invariant
//
// Claim: After step_once returns Ok, states[step] reflects the correct
//        StepState per the EngineSignal returned.
// Invariant: Continue/Finished → Succeeded, AwaitingAction/StepBudgetExhausted → Running,
//           AwaitingWait → Waiting, AwaitingAsk → Asking.

#![forbid(unsafe_code)]

use vb_core::frame::{RunFrame, StepState};
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
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
            3 => EngineSignal::AwaitingAction {
                step: StepIdx::new(kani::any::<u16>()),
                seq: SeqNo::new(kani::any::<u64>()),
                action: ActionId::new(kani::any::<u32>()),
            },
            4 => EngineSignal::AwaitingWait,
            _ => EngineSignal::AwaitingAsk,
        }
    }
}

#[kani::proof]
#[kani::unwind(6)]
fn step_once_state_mapping_harness() {
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
    kani::assume(step_count >= 2);
    kani::assume(step_count <= 16);
    let slot_count = plan.slot_count();
    kani::assume(slot_count <= 32);

    let first_step = StepIdx::new(kani::any::<u16>() % step_count);
    let mut run = match RunFrame::new(RunId::new(1), first_step, step_count, slot_count) {
        Ok(f) => f,
        Err(_) => return,
    };

    let mut store = ValueStore::new();
    let pc_before = run.pc();

    let result = step_once(&plan, &mut run, &mut store);

    if let Ok(signal) = result {
        let state = run.step_state(pc_before);
        kani::assert(state.is_ok(), "step_state read does not panic");

        let expected_state = match signal {
            EngineSignal::Continue | EngineSignal::Finished(_, _) => StepState::Succeeded,
            EngineSignal::AwaitingAction { .. } | EngineSignal::StepBudgetExhausted => StepState::Running,
            EngineSignal::AwaitingWait => StepState::Waiting,
            EngineSignal::AwaitingAsk => StepState::Asking,
        };

        kani::assert(
            state == Ok(expected_state),
            "states[step] matches signal mapping",
        );
    }
}