#![forbid(unsafe_code)]

//! Signal conversion helpers.

use vb_core::action::ActionTicket;
use vb_core::engine::EngineSignal;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

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
            capacity: 1,
        }),
        EngineSignal::AwaitingWait => RuntimeSignal::AwaitingWait,
        EngineSignal::AwaitingAsk => RuntimeSignal::AwaitingAsk,
        _ => RuntimeSignal::StepBudgetExhausted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
    use vb_core::value::{SlotValue, Taint};

    // =====================================================================
    // runtime_from_core: each EngineSignal variant maps correctly
    // =====================================================================

    #[test]
    fn continue_maps_to_runtime_continue() {
        let result = runtime_from_core(EngineSignal::Continue);
        assert_eq!(result, RuntimeSignal::Continue);
    }

    #[test]
    fn step_budget_exhausted_maps_directly() {
        let result = runtime_from_core(EngineSignal::StepBudgetExhausted);
        assert_eq!(result, RuntimeSignal::StepBudgetExhausted);
    }

    #[test]
    fn awaiting_wait_maps_directly() {
        let result = runtime_from_core(EngineSignal::AwaitingWait);
        assert_eq!(result, RuntimeSignal::AwaitingWait);
    }

    #[test]
    fn awaiting_ask_maps_directly() {
        let result = runtime_from_core(EngineSignal::AwaitingAsk);
        assert_eq!(result, RuntimeSignal::AwaitingAsk);
    }

    #[test]
    fn awaiting_action_produces_zeroed_ticket() {
        let result = runtime_from_core(EngineSignal::AwaitingAction);
        match result {
            RuntimeSignal::AwaitingAction(ticket) => {
                assert_eq!(ticket.run, RunId::ZERO);
                assert_eq!(ticket.step, StepIdx::ZERO);
                assert_eq!(ticket.seq, SeqNo::ZERO);
                assert_eq!(ticket.action, ActionId::new(0));
                assert_eq!(ticket.attempt, 1);
                assert_eq!(ticket.idempotency_key, 0);
            }
            other => {
                let msg = format!("expected AwaitingAction, got {other:?}");
                panic!("{msg}");
            }
        }
    }

    // =====================================================================
    // runtime_from_core: Finished discards taint, keeps value
    // =====================================================================

    #[test]
    fn finished_i64_extracts_value_ignores_clean_taint() {
        let result = runtime_from_core(EngineSignal::Finished(SlotValue::I64(42), Taint::Clean));
        assert_eq!(result, RuntimeSignal::Finished(SlotValue::I64(42)));
    }

    #[test]
    fn finished_i64_extracts_value_ignores_secret_taint() {
        let result = runtime_from_core(EngineSignal::Finished(SlotValue::I64(42), Taint::Secret));
        assert_eq!(result, RuntimeSignal::Finished(SlotValue::I64(42)));
    }

    #[test]
    fn finished_i64_extracts_value_ignores_derived_taint() {
        let result = runtime_from_core(EngineSignal::Finished(
            SlotValue::I64(99),
            Taint::DerivedFromSecret,
        ));
        assert_eq!(result, RuntimeSignal::Finished(SlotValue::I64(99)));
    }

    #[test]
    fn finished_bool_extracts_value() {
        let result = runtime_from_core(EngineSignal::Finished(SlotValue::Bool(true), Taint::Clean));
        assert_eq!(result, RuntimeSignal::Finished(SlotValue::Bool(true)));
    }

    #[test]
    fn finished_null_extracts_value() {
        let result = runtime_from_core(EngineSignal::Finished(SlotValue::Null, Taint::Clean));
        assert_eq!(result, RuntimeSignal::Finished(SlotValue::Null));
    }

    // =====================================================================
    // runtime_from_core: all variants produce distinct RuntimeSignals
    // =====================================================================

    #[test]
    fn each_core_variant_maps_to_distinct_runtime_signal() {
        let signals = [
            runtime_from_core(EngineSignal::Continue),
            runtime_from_core(EngineSignal::Finished(SlotValue::Null, Taint::Clean)),
            runtime_from_core(EngineSignal::StepBudgetExhausted),
            runtime_from_core(EngineSignal::AwaitingAction),
            runtime_from_core(EngineSignal::AwaitingWait),
            runtime_from_core(EngineSignal::AwaitingAsk),
        ];
        for (i, a) in signals.iter().enumerate() {
            for (j, b) in signals.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b, "same-index {i} should be equal");
                } else {
                    assert_ne!(a, b, "different indices {i},{j} should differ");
                }
            }
        }
    }

    // =====================================================================
    // runtime_from_core: Finished with different taints produces same signal
    // =====================================================================

    #[test]
    fn finished_same_value_different_taint_produces_equal_signal() {
        let clean = runtime_from_core(EngineSignal::Finished(SlotValue::I64(7), Taint::Clean));
        let secret = runtime_from_core(EngineSignal::Finished(SlotValue::I64(7), Taint::Secret));
        assert_eq!(clean, secret);
    }

    #[test]
    fn finished_different_value_produces_different_signal() {
        let a = runtime_from_core(EngineSignal::Finished(SlotValue::I64(1), Taint::Clean));
        let b = runtime_from_core(EngineSignal::Finished(SlotValue::I64(2), Taint::Clean));
        assert_ne!(a, b);
    }
}
