#![forbid(unsafe_code)]
//! Run state transition helpers: keep, finish, await action, await timer, fail.

use std::time::{Duration, Instant};
use vb_core::action::ActionTicket;
use vb_core::ids::{RunId, SlotIdx};
use vb_core::value::SlotValue;

use crate::journal::RuntimeJournalEvent;
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

use crate::shard::types::{
    PendingTimer, PendingTimerKind, RunState, RuntimeEvent, RuntimeState, Shard,
};

impl Shard {
    /// Applies a RuntimeEvent to mutate runtime_states.
    ///
    /// This is the single routing method for all runtime_states mutations,
    /// replacing direct insert/swap_remove call sites.
    ///
    /// # Arguments
    /// * `run` - The run identifier
    /// * `event` - The runtime event variant
    ///
    /// # State Transitions
    /// * `Submit` → `runtime_states.insert(run, RuntimeState::Initial)`
    /// * `Resume` → `runtime_states.insert(run, RuntimeState::Resuming)`
    /// * `ResumeRollback` → `runtime_states.insert(run, RuntimeState::Resumable)` (journal failure)
    /// * `DriveContinue` → `runtime_states.insert(run, RuntimeState::Running)`
    /// * `AwaitAction` → `runtime_states.insert(run, RuntimeState::Resumable)`
    /// * `AwaitTimer` → `runtime_states.insert(run, RuntimeState::Resumable)`
    /// * `Fail` → `runtime_states.insert(run, RuntimeState::Failed)`
    /// * `TerminalRemove` → `runtime_states.swap_remove(&run)`
    /// * `DriveFinished` → `runtime_states.swap_remove(&run)`
    ///
    /// # Flux refinement (PO-vb282my-RS-FLUX-001):
    /// RuntimeState FSM contract:
    /// - `Resume` transition requires `runtime_states[run] == Resumable`
    /// - `ResumeRollback` transition ensures `runtime_states[run] == Resumable`
    /// - `Running` state rejects repeated `Resume` transitions
    ///
    /// Flux signature (requires flux-rs toolchain):
    /// ```flux
    /// #[flux_rs::sig(fn(&mut Shard, run: RunId, event: RuntimeEvent)
    ///     requires event == Resume => runtime_states[run] == Resumable,
    ///     ensures event == ResumeRollback => runtime_states[run] == Resumable
    /// )]
    /// ```
    pub(crate) fn apply(&mut self, run: RunId, event: RuntimeEvent) {
        match event {
            RuntimeEvent::Submit => {
                self.runtime_state_insert(run, RuntimeState::Initial);
            }
            RuntimeEvent::Resume => {
                self.runtime_state_insert(run, RuntimeState::Resuming);
            }
            RuntimeEvent::ResumeRollback => {
                // Journal append failed during resume, revert to Resumable
                self.runtime_state_insert(run, RuntimeState::Resumable);
            }
            RuntimeEvent::DriveContinue => {
                self.runtime_state_insert(run, RuntimeState::Running);
            }
            RuntimeEvent::AwaitAction | RuntimeEvent::AwaitTimer => {
                self.runtime_state_insert(run, RuntimeState::Resumable);
            }
            RuntimeEvent::Fail => {
                self.runtime_state_insert(run, RuntimeState::Failed);
            }
            RuntimeEvent::TerminalRemove | RuntimeEvent::DriveFinished => {
                self.runtime_states.swap_remove(&run);
            }
        }
    }

    /// Re-inserts a run that has remaining work into the active runs map.
    pub(crate) fn keep_run(&mut self, run: RunId, state: RunState) {
        self.counters.add_steps(state.frame.executed());
        self.run_state_insert(run, state);
    }

    /// Marks a run as finished, releases its frame, and updates counters.
    pub(crate) fn finish_run(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        self.pending_timer_remove(run);
        self.terminal_runs_insert(run);
        self.counters.inc_completed();
        self.counters.add_steps(state.frame.executed());
        self.trace_ring.push(TraceEvent::RunFinished { run });
        let result = match crate::shard::helpers::result_slot_for_finished_run(&state) {
            Some(slot) => slot,
            None => SlotIdx::ZERO,
        };
        // Note: StepSucceeded for the Finish step is now emitted by the evidence
        // collector during flush_evidence, before apply_drive_result is called.
        self.append_journal_event(RuntimeJournalEvent::RunFinished { run, result })?;
        self.release_frame(state.frame);
        self.discard_journal_sequence(run);
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
        let capacity = match crate::shard::helpers::retry_policy_after_action(&state, ticket.step) {
            Ok(policy) => policy.max_attempts,
            Err(RuntimeError::UnsupportedOperation {
                operation: "retry_metadata_missing",
            }) => ticket.capacity,
            Err(error) => return Err(error),
        };
        let ticket = crate::shard::helpers::normalize_scheduled_ticket(
            &state,
            ActionTicket { capacity, ..ticket },
        )?;
        crate::shard::helpers::record_scheduled_attempt(&mut state, ticket);
        self.trace_ring
            .push(TraceEvent::ActionScheduled { run, step });
        let output = crate::shard::helpers::action_output_slot(&state, ticket.step)?;
        let input = crate::shard::helpers::action_input_slot(&state, ticket.step)?;
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
        if crate::shard::helpers::timer_registration_required(&state, step) {
            let generation = match self.next_pending_timer_generation(run) {
                Some(generation) => generation,
                None => {
                    self.run_state_insert(run, state);
                    return Err(RuntimeError::InvalidTimerFire);
                }
            };
            let append_result = match kind {
                PendingTimerKind::Wait => {
                    self.append_journal_event(RuntimeJournalEvent::WaitScheduled { run, step })
                }
                PendingTimerKind::Ask => {
                    self.append_journal_event(RuntimeJournalEvent::AskScheduled { run, step })
                }
            };
            if let Err(error) = append_result {
                self.run_state_insert(run, state);
                return Err(error);
            }
            let deadline = compute_deadline_from_slot(&state, deadline_slot);
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

    /// Marks a run as failed, releases its frame, and updates counters.
    /// NOTE: runtime_states mutation (inserting Failed) is handled by apply() before this is called.
    /// This function only handles cleanup: pending_timers, counters, trace, journal, frame, sequence.
    pub(crate) fn fail_run_state(&mut self, run: RunId, state: RunState) -> RuntimeResult<()> {
        self.pending_timer_remove(run);
        self.terminal_runs_insert(run);
        self.counters.inc_failed();
        self.trace_ring.push(TraceEvent::RunFailed { run });
        self.append_journal_event(RuntimeJournalEvent::RunFailed { run })?;
        self.release_frame(state.frame);
        self.discard_journal_sequence(run);
        Ok(())
    }
}

/// Computes the wall-clock deadline for a pending timer from the slot
/// the wait/ask primitive validated.
///
/// The slot value is treated as a positive offset in milliseconds
/// from `Instant::now()`. When the slot is uninitialized or holds a
/// non-numeric value, the deadline collapses to `Instant::now()` so
/// the timer fires on the next tick (matches the previous
/// synthesizing-fallback behavior, but only as a degenerate case the
/// caller can detect via the slot's value, never silently).
fn compute_deadline_from_slot(state: &RunState, slot: SlotIdx) -> Instant {
    let now = Instant::now();
    match state.frame.read_slot(slot) {
        Ok(SlotValue::I64(ms)) => match u64::try_from(ms).ok() {
            Some(m) => now.checked_add(Duration::from_millis(m)).unwrap_or(now),
            None => now,
        },
        Ok(SlotValue::F64(value)) => {
            // FiniteF64 guarantees finiteness; only the sign check is needed.
            let ms = value.get();
            if !ms.is_sign_positive() {
                return now;
            }
            // Clamp to u64::MAX to avoid lossy cast traps.
            let capped = if ms > u64::MAX as f64 {
                u64::MAX
            } else {
                ms as u64
            };
            now.checked_add(Duration::from_millis(capped)).unwrap_or(now)
        }
        _ => now,
    }
}
