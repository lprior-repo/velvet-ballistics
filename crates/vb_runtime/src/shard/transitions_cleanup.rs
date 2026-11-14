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
        self.control
            .completion_capture
            .capture_terminal(run, &state);
        let _removed_timer = self.pending_timer_remove(run);
        let _removed_action = self.pending_action_remove(run);
        self.clear_pending_ask_authority(run);
        self.terminal_runs_insert(run)?;
        self.counters.inc_completed();
        self.release_state_quota(run)?;
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
        self.release_state_quota(run)?;
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
        let Some((event, projected)) = self.prepare_retry_checkpoint(run, retry_step, state)?
        else {
            return Ok(());
        };
        self.append_journal_event(event)?;
        state.journal_bytes_written = projected;
        Ok(())
    }

    fn prepare_retry_checkpoint(
        &self,
        run: RunId,
        retry_step: StepIdx,
        state: &mut RunState,
    ) -> RuntimeResult<Option<(RuntimeJournalEvent, u64)>> {
        let Some((mut checkpoint, next_attempt)) =
            self.capture_retry_checkpoint_value(run, retry_step, state)
        else {
            return Ok(None);
        };
        let event =
            match self.retry_checkpoint_event(&mut checkpoint, run, next_attempt, retry_step) {
                Ok(event) => event,
                Err(_) => return Ok(None),
            };
        let projected = state
            .journal_bytes_written
            .checked_add(self.journal_event_bytes(&event)?)
            .ok_or(RuntimeError::EncodeFailed)?;
        Ok(Some((event, projected)))
    }

    fn capture_retry_checkpoint_value(
        &self,
        run: RunId,
        retry_step: StepIdx,
        state: &mut RunState,
    ) -> Option<(RunCheckpointV1, NonZeroU16)> {
        if state.frame.mark_failed(retry_step).is_err() {
            return None;
        }
        let (artifact_digest, next_attempt, prior_seq) =
            self.retry_checkpoint_context(run, state)?;
        let checkpoint = RunCheckpointV1::capture(
            run,
            artifact_digest,
            next_attempt,
            retry_step,
            prior_seq,
            state,
        )
        .ok()?;
        Some((checkpoint, next_attempt))
    }

    fn retry_checkpoint_context(
        &self,
        run: RunId,
        state: &RunState,
    ) -> Option<(vb_core::ids::WorkflowDigest, NonZeroU16, u64)> {
        let artifact_digest = state.admission.as_ref()?.artifact_digest();
        let next_attempt = state
            .run_attempt
            .get()
            .checked_add(1)
            .and_then(NonZeroU16::new)?;
        let prior_seq = self.journal_sequence_for(run).get().checked_sub(1)?;
        Some((artifact_digest, next_attempt, prior_seq))
    }

    fn retry_checkpoint_event(
        &self,
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
            let (event, next) =
                self.retry_checkpoint_event_at(checkpoint, run, attempt, step, base, stored)?;
            if next == stored {
                return Ok(event);
            }
            stored = next;
        }
        Err(RuntimeError::EncodeFailed)
    }

    fn retry_checkpoint_event_at(
        &self,
        checkpoint: &mut RunCheckpointV1,
        run: RunId,
        attempt: NonZeroU16,
        step: StepIdx,
        base: u64,
        stored: u64,
    ) -> RuntimeResult<(RuntimeJournalEvent, u64)> {
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
        let event_bytes = self.journal_event_bytes(&event)?;
        let next = base
            .checked_add(event_bytes)
            .ok_or(RuntimeError::EncodeFailed)?;
        Ok((event, next))
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
