// VB-ERR001-KANI: error handling exhaustiveness
//
// Claim: step_once returns Err for all EngineError variants without panicking.
//        All error paths return Err without crashing.

#![forbid(unsafe_code)]

use vb_core::errors::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{RunId, StepIdx};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledWorkflow, WorkflowParts};
use vb_core::EngineSignal;
use vb_core::engine::step_once;

impl kani::Arbitrary for EngineSignal {
    fn any() -> Self {
        match kani::any::<u8>() % 6 {
            0 => EngineSignal::Continue,
            1 => EngineSignal::Finished(kani::any(), kani::any()),
            2 => EngineSignal::StepBudgetExhausted,
            3 => EngineSignal::AwaitingAction,
            4 => EngineSignal::AwaitingWait,
            _ => EngineSignal::AwaitingAsk,
        }
    }
}

#[kani::proof]
#[kani::unwind(6)]
fn step_once_error_harness() {
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

    let first_step_raw = kani::any::<u16>();
    let first_step = StepIdx::new(first_step_raw % step_count.max(1));

    let mut run = match RunFrame::new(RunId::new(1), first_step, step_count, slot_count) {
        Ok(f) => f,
        Err(_) => return,
    };

    let mut store = ValueStore::new();

    let result = step_once(&plan, &mut run, &mut store);

    match result {
        Ok(signal) => {
            match signal {
                EngineSignal::Continue
                | EngineSignal::Finished(_, _)
                | EngineSignal::StepBudgetExhausted
                | EngineSignal::AwaitingAction
                | EngineSignal::AwaitingWait
                | EngineSignal::AwaitingAsk => {
                }
            }
        }
        Err(e) => {
            match e {
                EngineError::InvalidProgramCounter { .. }
                | EngineError::MissingNextStep { .. }
                | EngineError::SlotOutOfBounds { .. }
                | EngineError::SlotUninitialized { .. }
                | EngineError::MissingOutputSlot { .. }
                | EngineError::StepStateOutOfBounds { .. }
                | EngineError::TypeMismatch { .. }
                | EngineError::DivisionByZero
                | EngineError::NonFiniteNumber
                | EngineError::ResourceLimitExceeded { .. }
                | EngineError::UnsupportedPrimitive { .. }
                | EngineError::InternalInvariantViolation { .. }
                | EngineError::StepCounterOverflow
                | EngineError::BudgetParse { .. }
                | EngineError::InvalidCompiledWorkflow { .. } => {
                }
            }
        }
    }
}