#![forbid(unsafe_code)]

//! Signal conversion helpers.
//!
//! Production function `runtime_from_core` is total over the `EngineSignal`
//! enum and returns `RuntimeSignal` without panicking. It carries the active
//! run id and the next journal sequence number so that `AwaitingAction`
//! variants from the IR interpreter are converted into live `ActionTicket`
//! values with the correct `run`, `seq`, `action`, and `idempotency_key`.

use vb_core::action::{ActionTicket, compute_action_idempotency_key};
use vb_core::engine::EngineSignal;
use vb_core::ids::{RunId, SeqNo};

use crate::engine::types::RuntimeSignal;

/// Converts core engine signals to runtime engine signals.
///
/// `run` is the active run identifier that owns the frame the interpreter
/// produced the signal from. `journal_seq` is the next sequence number the
/// runtime would assign if it were the first to journal this suspension.
/// The IR interpreter emits `SeqNo::ZERO` for `AwaitingAction` because it does
/// not track journal sequences; this function substitutes the runtime's
/// sequence so the live `ActionTicket` carries the canonical idempotency key
/// (`compute_action_idempotency_key(run, seq, action)`).
#[allow(clippy::needless_pass_by_value)]
pub fn runtime_from_core(run: RunId, journal_seq: SeqNo, signal: EngineSignal) -> RuntimeSignal {
    match signal {
        EngineSignal::Continue => RuntimeSignal::Continue,
        EngineSignal::Finished(value, _taint) => RuntimeSignal::Finished(value),
        EngineSignal::StepBudgetExhausted => RuntimeSignal::StepBudgetExhausted,
        EngineSignal::AwaitingAction {
            step,
            seq,
            action,
        } => {
            // If the IR interpreter supplied a non-ZERO seq, trust it (the
            // runtime has its own SeqNo source for production handlers; this
            // branch only triggers when the IR path is exercised directly).
            // For the documented IR sentinel (SeqNo::ZERO), substitute the
            // runtime's journal_seq so the ticket carries the canonical key.
            let effective_seq = if seq == SeqNo::ZERO {
                journal_seq
            } else {
                seq
            };
            let idempotency_key = compute_action_idempotency_key(run, effective_seq, action);
            RuntimeSignal::AwaitingAction(ActionTicket {
                run,
                step,
                seq: effective_seq,
                action,
                attempt: 1,
                idempotency_key,
                capacity: 1,
                ..Default::default()
            })
        }
        EngineSignal::AwaitingWait { deadline_slot } => RuntimeSignal::AwaitingWait(deadline_slot),
        EngineSignal::AwaitingAsk { timeout_slot } => RuntimeSignal::AwaitingAsk(timeout_slot),
        // Handle any future EngineSignal variants as Continue (safest default).
        #[allow(unreachable_code)]
        _ => RuntimeSignal::Continue,
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
        let result = runtime_from_core(RunId::new(1), SeqNo::ZERO, EngineSignal::Continue);
        assert_eq!(result, RuntimeSignal::Continue);
    }

    #[test]
    fn step_budget_exhausted_maps_directly() {
        let result = runtime_from_core(
            RunId::new(1),
            SeqNo::ZERO,
            EngineSignal::StepBudgetExhausted,
        );
        assert_eq!(result, RuntimeSignal::StepBudgetExhausted);
    }

    #[test]
    fn awaiting_wait_maps_directly() {
        let result = runtime_from_core(
            RunId::new(1),
            SeqNo::ZERO,
            EngineSignal::AwaitingWait {
                deadline_slot: vb_core::ids::SlotIdx::new(0),
            },
        );
        assert_eq!(
            result,
            RuntimeSignal::AwaitingWait(vb_core::ids::SlotIdx::new(0))
        );
    }

    #[test]
    fn awaiting_ask_maps_directly() {
        let result = runtime_from_core(
            RunId::new(1),
            SeqNo::ZERO,
            EngineSignal::AwaitingAsk { timeout_slot: None },
        );
        assert_eq!(result, RuntimeSignal::AwaitingAsk(None));
    }

    #[test]
    fn awaiting_action_constructs_live_ticket_from_signal_and_run() {
        let run = RunId::new(7);
        let journal_seq = SeqNo::new(3);
        let step = StepIdx::new(2);
        let action = ActionId::new(9);
        let result = runtime_from_core(
            run,
            journal_seq,
            EngineSignal::AwaitingAction {
                step,
                seq: SeqNo::ZERO,
                action,
            },
        );
        match result {
            RuntimeSignal::AwaitingAction(ticket) => {
                assert_eq!(ticket.run, run);
                assert_eq!(ticket.step, step);
                assert_eq!(ticket.seq, journal_seq);
                assert_eq!(ticket.action, action);
                assert_eq!(ticket.attempt, 1);
                assert_eq!(ticket.capacity, 1);
                assert_eq!(
                    ticket.idempotency_key,
                    compute_action_idempotency_key(run, journal_seq, action),
                    "ticket idempotency_key must be the canonical hash of (run, seq, action)",
                );
            }
            other => panic!("expected AwaitingAction, got {other:?}"),
        }
    }

    #[test]
    fn awaiting_action_trusts_non_zero_seq_from_signal() {
        // When the IR interpreter explicitly carries a non-ZERO seq, the
        // runtime must respect it instead of overwriting with journal_seq.
        let run = RunId::new(7);
        let journal_seq = SeqNo::new(99);
        let explicit_seq = SeqNo::new(5);
        let step = StepIdx::new(2);
        let action = ActionId::new(9);
        let result = runtime_from_core(
            run,
            journal_seq,
            EngineSignal::AwaitingAction {
                step,
                seq: explicit_seq,
                action,
            },
        );
        match result {
            RuntimeSignal::AwaitingAction(ticket) => {
                assert_eq!(ticket.seq, explicit_seq);
                assert_eq!(
                    ticket.idempotency_key,
                    compute_action_idempotency_key(run, explicit_seq, action),
                );
            }
            other => panic!("expected AwaitingAction, got {other:?}"),
        }
    }

    // =====================================================================
    // runtime_from_core: Finished discards taint, keeps value
    // =====================================================================

    #[test]
    fn finished_i64_extracts_value_ignores_clean_taint() {
        let result = runtime_from_core(
            RunId::new(1),
            SeqNo::ZERO,
            EngineSignal::Finished(SlotValue::I64(42), Taint::Clean),
        );
        assert_eq!(result, RuntimeSignal::Finished(SlotValue::I64(42)));
    }

    #[test]
    fn finished_i64_extracts_value_ignores_secret_taint() {
        let result = runtime_from_core(
            RunId::new(1),
            SeqNo::ZERO,
            EngineSignal::Finished(SlotValue::I64(42), Taint::Secret),
        );
        assert_eq!(result, RuntimeSignal::Finished(SlotValue::I64(42)));
    }

    #[test]
    fn finished_i64_extracts_value_ignores_derived_taint() {
        let result = runtime_from_core(
            RunId::new(1),
            SeqNo::ZERO,
            EngineSignal::Finished(SlotValue::I64(99), Taint::DerivedFromSecret),
        );
        assert_eq!(result, RuntimeSignal::Finished(SlotValue::I64(99)));
    }

    #[test]
    fn finished_bool_extracts_value() {
        let result = runtime_from_core(
            RunId::new(1),
            SeqNo::ZERO,
            EngineSignal::Finished(SlotValue::Bool(true), Taint::Clean),
        );
        assert_eq!(result, RuntimeSignal::Finished(SlotValue::Bool(true)));
    }

    #[test]
    fn finished_null_extracts_value() {
        let result = runtime_from_core(
            RunId::new(1),
            SeqNo::ZERO,
            EngineSignal::Finished(SlotValue::Null, Taint::Clean),
        );
        assert_eq!(result, RuntimeSignal::Finished(SlotValue::Null));
    }

    // =====================================================================
    // runtime_from_core: all variants produce distinct RuntimeSignals
    // =====================================================================

    #[test]
    fn each_core_variant_maps_to_distinct_runtime_signal() {
        let run = RunId::new(1);
        let seq = SeqNo::ZERO;
        let signals = [
            runtime_from_core(run, seq, EngineSignal::Continue),
            runtime_from_core(run, seq, EngineSignal::Finished(SlotValue::Null, Taint::Clean)),
            runtime_from_core(run, seq, EngineSignal::StepBudgetExhausted),
            runtime_from_core(
                run,
                seq,
                EngineSignal::AwaitingAction {
                    step: StepIdx::ZERO,
                    seq: SeqNo::ZERO,
                    action: ActionId::new(0),
                },
            ),
            runtime_from_core(
                run,
                seq,
                EngineSignal::AwaitingWait {
                    deadline_slot: vb_core::ids::SlotIdx::new(0),
                },
            ),
            runtime_from_core(run, seq, EngineSignal::AwaitingAsk { timeout_slot: None }),
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
        let clean = runtime_from_core(
            RunId::new(1),
            SeqNo::ZERO,
            EngineSignal::Finished(SlotValue::I64(7), Taint::Clean),
        );
        let secret = runtime_from_core(
            RunId::new(1),
            SeqNo::ZERO,
            EngineSignal::Finished(SlotValue::I64(7), Taint::Secret),
        );
        assert_eq!(clean, secret);
    }

    #[test]
    fn finished_different_value_produces_different_signal() {
        let a = runtime_from_core(
            RunId::new(1),
            SeqNo::ZERO,
            EngineSignal::Finished(SlotValue::I64(1), Taint::Clean),
        );
        let b = runtime_from_core(
            RunId::new(1),
            SeqNo::ZERO,
            EngineSignal::Finished(SlotValue::I64(2), Taint::Clean),
        );
        assert_ne!(a, b);
    }
}
