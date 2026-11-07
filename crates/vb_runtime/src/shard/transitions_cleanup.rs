use core::num::NonZeroU16;

use vb_core::ids::{RunId, StepIdx};

use crate::journal::RuntimeJournalEvent;
use crate::run_checkpoint::RunCheckpointV1;
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

use crate::shard::types::RunState;

impl crate::shard::Shard {
    /// Completes successful-run cleanup after `RunFinished` is already
    /// durably appended by the caller, either as a single event or as part of
    /// a same-run atomic batch with preceding drive evidence.
    pub(crate) fn finish_run_after_journaled(
        &mut self,
        run: RunId,
        state: RunState,
    ) -> RuntimeResult<()> {
        // The terminal append wrapper succeeded. Capture before fallible cleanup;
        // the caller may still receive an execution error after this observation.
        self.completion_capture.capture_terminal(run, &state);
        let _removed_timer = self.pending_timer_remove(run);
        let _removed_action = self.pending_action_remove(run);
        self.clear_pending_ask_authority(run);
        self.terminal_runs_insert(run)?;
        self.counters.inc_completed();
        self.add_executed_step_delta(run, state.frame.executed());
        self.trace_ring.push(TraceEvent::RunFinished { run });
        self.release_frame(state.frame);
        self.discard_journal_sequence(run);
        self.clear_executed_step_accounting(run);
        Ok(())
    }

    /// Completes failed-run cleanup after `RunFailed` is already durably
    /// appended by the caller, either as a single event or as part of a
    /// same-run atomic batch with preceding failure evidence.
    pub(crate) fn fail_run_state_after_journaled(
        &mut self,
        run: RunId,
        state: RunState,
    ) -> RuntimeResult<()> {
        let _removed_timer = self.pending_timer_remove(run);
        let _removed_action = self.pending_action_remove(run);
        self.clear_pending_ask_authority(run);
        self.terminal_runs_insert(run)?;
        self.counters.inc_failed();
        self.trace_ring.push(TraceEvent::RunFailed { run });
        self.release_frame(state.frame);
        self.runtime_state_remove(run);
        self.discard_journal_sequence(run);
        self.clear_executed_step_accounting(run);
        Ok(())
    }

    /// Marks a run failed and, when retry authority is intact, captures a full
    /// [`crate::run_checkpoint::RunCheckpointV1`] and appends it as durable
    /// `RetryCheckpoint` evidence.
    pub(crate) fn fail_run_state_with_retry_checkpoint(
        &mut self,
        run: RunId,
        retry_step: StepIdx,
        mut state: RunState,
    ) -> RuntimeResult<()> {
        let checkpoint_result = self.capture_retry_checkpoint(run, retry_step, &mut state);
        self.fail_run_state_after_journaled(run, state)?;
        checkpoint_result
    }

    pub(crate) fn capture_retry_checkpoint(
        &mut self,
        run: RunId,
        retry_step: StepIdx,
        state: &mut RunState,
    ) -> RuntimeResult<()> {
        if state.frame.mark_failed(retry_step).is_err() {
            return Ok(());
        }
        let Some(admission) = state.admission.as_ref() else {
            return Ok(());
        };
        let artifact_digest = admission.artifact_digest();
        let next_attempt = match state
            .run_attempt
            .get()
            .checked_add(1)
            .and_then(core::num::NonZeroU16::new)
        {
            Some(attempt) => attempt,
            None => return Ok(()),
        };
        let prior_seq = match self.journal_sequence_for(run).get().checked_sub(1) {
            Some(seq) => seq,
            None => return Ok(()),
        };
        let mut checkpoint = match RunCheckpointV1::capture(
            run,
            artifact_digest,
            next_attempt,
            retry_step,
            prior_seq,
            state,
        ) {
            Ok(checkpoint) => checkpoint,
            Err(_) => return Ok(()),
        };
        let base = state.journal_bytes_written;
        let event =
            match Self::retry_checkpoint_event(&mut checkpoint, run, next_attempt, retry_step) {
                Ok(event) => event,
                Err(_) => return Ok(()),
            };
        let projected = base
            .checked_add(Self::journal_event_bytes(&event)?)
            .ok_or(RuntimeError::EncodeFailed)?;
        self.append_journal_event(event)?;
        state.journal_bytes_written = projected;
        Ok(())
    }

    fn retry_checkpoint_event(
        checkpoint: &mut RunCheckpointV1,
        run: RunId,
        attempt: NonZeroU16,
        step: StepIdx,
    ) -> RuntimeResult<RuntimeJournalEvent> {
        let base = checkpoint.journal_bytes_written;
        let mut stored = base;
        // The stored counter must include this event's own encoded size, but
        // that size depends on the stored counter through the postcard varint
        // lengths of the counter and the payload length. Both lengths are
        // non-decreasing step functions that change at most nine times each,
        // so the fixed point is reached within nineteen rounds; thirty-two
        // is a safe static bound.
        for _ in 0..32 {
            checkpoint.journal_bytes_written = stored;
            let payload =
                postcard::to_allocvec(&*checkpoint).map_err(|_| RuntimeError::EncodeFailed)?;
            let payload_digest = *blake3::hash(&payload).as_bytes();
            let event = RuntimeJournalEvent::RetryCheckpoint {
                run,
                attempt: attempt.get(),
                step,
                payload_digest,
                payload,
            };
            let event_bytes = Self::journal_event_bytes(&event)?;
            let next = base
                .checked_add(event_bytes)
                .ok_or(RuntimeError::EncodeFailed)?;
            if next == stored {
                return Ok(event);
            }
            stored = next;
        }
        Err(RuntimeError::EncodeFailed)
    }

    pub(crate) fn rollback_run_state_preserving_error(
        &mut self,
        operation: &'static str,
        run: RunId,
        state: RunState,
        original: RuntimeError,
    ) -> RuntimeResult<()> {
        match self.run_state_insert(run, state) {
            Ok(_) => Err(original),
            Err(rollback) => Err(RuntimeError::rollback_failed(operation, original, rollback)),
        }
    }
}
