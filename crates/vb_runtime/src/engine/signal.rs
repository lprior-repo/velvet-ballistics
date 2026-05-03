#![forbid(unsafe_code)]

//! Signal conversion helpers.

use vb_core::action::ActionTicket;
use vb_core::engine::EngineSignal;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_core::value::SlotValue;

use crate::engine::types::RuntimeSignal;

/// Converts core engine signals to runtime engine signals.
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_from_core(signal: EngineSignal) -> RuntimeSignal {
    match signal {
        EngineSignal::Continue => RuntimeSignal::Continue,
        EngineSignal::Finished(value, _taint) => RuntimeSignal::Finished(value),
        EngineSignal::StepBudgetExhausted => RuntimeSignal::StepBudgetExhausted,
        EngineSignal::AwaitingAction => RuntimeSignal::AwaitingAction(ActionTicket {
            run: RunId::ZERO,
            step: StepIdx::ZERO,
            seq: SeqNo::ZERO,
            action: ActionId::new(0),
            attempt: 1,
            idempotency_key: 0,
        }),
        EngineSignal::AwaitingWait => RuntimeSignal::AwaitingWait,
        EngineSignal::AwaitingAsk => RuntimeSignal::AwaitingAsk,
    }
}
