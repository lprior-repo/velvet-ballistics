#![forbid(unsafe_code)]
//! Continuation transitions: keep run, await action, await timer.
//!
//! - `keep_run` — re-insert run with remaining work
//! - `keep_run_with_snapshot` — re-insert with periodic snapshot
//! - `await_action` — transition to awaiting external action
//! - `await_timer` — transition to awaiting a timer (wait or ask timeout)
//! - `compute_deadline_ms_from_slot` — deadline computation helper

use vb_core::action::ActionTicket;
use vb_core::ids::{RunId, SlotIdx};

use crate::journal::RuntimeJournalEvent;
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

use super::SnapshotWriteOutcome;
use crate::shard::helpers::{
    action_input_slot, action_output_slot, normalize_scheduled_ticket, record_scheduled_attempt,
    retry_policy_after_action, timer_registration_required,
};
use crate::shard::types::{PendingTimer, PendingTimerKind, RunState, Shard};

impl Shard {
    /// Re-inserts a run that has remaining work into the active runs map.
    #[allow(dead_code)]
    pub(crate) fn keep_run(&mut self, run: RunId, state: RunState) {
        self.counters.add_steps(state.frame.executed());
        self.run_state_insert(run, state);
    }

    /// Re-inserts a run into the active runs map, attempting a periodic snapshot.
    ///
    /// If `snapshot_interval_steps > 0` and enough steps have elapsed since the
    /// last snapshot, a `RunSnapshot` is written before the state is re-inserted.
    /// Snapshot write errors are non-fatal — the run always continues.
    pub(crate) fn keep_run_with_snapshot(
        &mut self,
        run: RunId,
        mut state: RunState,
    ) -> RuntimeResult<()> {
        let executed = state.frame.executed();
        let interval = self.snapshot_interval_steps;
        let last_executed = state.last_snapshot_executed;

        // Attempt periodic snapshot if enabled.
        // write_snapshot_for_run is non-blocking: serialization and storage
        // errors return SnapshotWriteOutcome::Failed rather than propagating.
        let outcome = self.write_snapshot_for_run(run, &state, interval, executed, last_executed);

        if matches!(outcome, SnapshotWriteOutcome::Written) {
            state.last_snapshot_executed = executed;
        }

        self.counters.add_steps(executed);
        self.run_state_insert(run, state);
        Ok(())
    }

    /// Transitions a run to awaiting an external action response.
    pub(crate) fn await_action(
        &mut self,
        run: RunId,
        mut state: RunState,
        ticket: ActionTicket,
    ) -> RuntimeResult<()> {
        self.counters.add_steps(state.frame.executed());
        let step = state.frame.pc();
        let capacity = match retry_policy_after_action(&state, ticket.step) {
            Ok(policy) => policy.max_attempts,
            Err(RuntimeError::UnsupportedOperation {
                operation: "retry_metadata_missing",
            }) => ticket.capacity,
            Err(error) => return Err(error),
        };
        let ticket = normalize_scheduled_ticket(
            &state,
            ActionTicket { capacity, ..ticket },
        )?;
        record_scheduled_attempt(&mut state, ticket);
        self.trace_ring
            .push(TraceEvent::ActionScheduled { run, step });
        let output = action_output_slot(&state, ticket.step)?;
        let input = action_input_slot(&state, ticket.step)?;
        self.append_journal_event(RuntimeJournalEvent::ActionScheduledTicket {
            ticket,
            input,
            output,
        })?;
        self.run_state_insert(run, state);
        Ok(())
    }

    /// Transitions a run to awaiting a timer (wait or ask timeout).
    ///
    /// The deadline is computed from the slot the wait/ask primitive
    /// validated, NOT synthesized from `Instant::now()`. The caller
    /// must supply the slot index that the suspended node read its
    /// deadline from. An unwritten or non-numeric slot yields a zero
    /// deadline (`Instant::now()`), which causes the timer to fire
    /// immediately on the next event-loop tick.
    pub(crate) fn await_timer(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
        deadline_slot: SlotIdx,
    ) -> RuntimeResult<()> {
        self.counters.add_steps(state.frame.executed());
        let step = state.frame.pc();
        if timer_registration_required(&state, step) {
            let generation = match self.next_pending_timer_generation(run) {
                Some(generation) => generation,
                None => {
                    self.run_state_insert(run, state);
                    return Err(RuntimeError::InvalidTimerFire);
                }
            };
            let deadline_ms = compute_deadline_ms_from_slot(&state, deadline_slot);
            let append_result = match kind {
                PendingTimerKind::Wait => {
                    self.append_journal_event(RuntimeJournalEvent::WaitScheduled {
                        run,
                        step,
                        deadline_ms,
                    })
                }
                PendingTimerKind::Ask => {
                    self.append_journal_event(RuntimeJournalEvent::AskScheduled {
                        run,
                        step,
                        deadline_ms,
                    })
                }
            };
            if let Err(error) = append_result {
                self.run_state_insert(run, state);
                return Err(error);
            }
            let deadline = std::time::Instant::now()
                .checked_add(std::time::Duration::from_millis(deadline_ms))
                .unwrap_or_else(std::time::Instant::now);
            self.pending_timer_insert(
                run,
                PendingTimer {
                    step,
                    kind,
                    generation,
                    deadline,
                },
            );
        }
        self.run_state_insert(run, state);
        Ok(())
    }
}

/// Computes the wall-clock deadline for a pending timer from the slot
/// the wait/ask primitive validated.
///
/// Reads the deadline duration in milliseconds from a slot value.
/// Returns a default of 0ms when the slot is unreadable or non-numeric.
pub(crate) fn compute_deadline_ms_from_slot(state: &RunState, slot: SlotIdx) -> u64 {
    match state.frame.read_slot(slot) {
        Ok(vb_core::value::SlotValue::I64(ms)) => u64::try_from(*ms).unwrap_or(0),
        Ok(vb_core::value::SlotValue::F64(value)) => {
            let ms = value.get();
            if !ms.is_finite() || !ms.is_sign_positive() {
                return 0;
            }
            #[allow(clippy::as_conversions)]
            if ms > u64::MAX as f64 {
                u64::MAX
            } else {
                #[allow(clippy::as_conversions)]
                {
                    ms as u64
                }
            }
        }
        _ => 0,
    }
}
