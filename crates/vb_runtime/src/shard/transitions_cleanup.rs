use vb_core::ids::{RunId, StepIdx};

use crate::journal::RuntimeJournalEvent;
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

    fn capture_retry_checkpoint(
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
        let checkpoint = match crate::run_checkpoint::RunCheckpointV1::capture(
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
        let payload = match postcard::to_allocvec(&checkpoint) {
            Ok(bytes) => bytes,
            Err(_) => return Ok(()),
        };
        let payload_digest = *blake3::hash(&payload).as_bytes();
        self.append_journal_event(RuntimeJournalEvent::RetryCheckpoint {
            run,
            attempt: next_attempt.get(),
            step: retry_step,
            payload_digest,
            payload,
        })
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
