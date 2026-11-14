struct RetryTimerPlan {
    timer: PendingTimer,
    event: RuntimeJournalEvent,
}

struct RetryTimerEventContext<'a> {
    run: RunId,
    preflight: &'a ActionFailurePreflight,
    timer: PendingTimer,
    base_bytes: u64,
    failed_event: RuntimeJournalEvent,
    scheduled_unix_ms: u64,
    deadline_unix_ms: u64,
}

impl Shard {
    fn retry_timer_event(
        &self,
        run: RunId,
        preflight: &ActionFailurePreflight,
        timer: PendingTimer,
    ) -> RuntimeResult<RuntimeJournalEvent> {
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        let checkpoint = self.retry_timer_checkpoint(run, preflight, state)?;
        let (scheduled_unix_ms, deadline_unix_ms) = Self::retry_timer_times(preflight.delay_ms)?;
        let failed_event = RuntimeJournalEvent::ActionFailed {
            run,
            step: preflight.ticket.step,
            action: preflight.ticket.action,
            attempt: preflight.ticket.attempt,
        };
        let context = RetryTimerEventContext {
            run,
            preflight,
            timer,
            base_bytes: state.journal_bytes_written,
            failed_event,
            scheduled_unix_ms,
            deadline_unix_ms,
        };
        self.retry_timer_event_with_checkpoint(&context, checkpoint)
    }

    fn retry_timer_times(delay_ms: u64) -> RuntimeResult<(u64, u64)> {
        let scheduled = crate::shard::helpers::wall_clock_millis_since_epoch()
            .ok_or(RuntimeError::InvalidTimerFire)?;
        let deadline = scheduled
            .checked_add(delay_ms)
            .ok_or(RuntimeError::InvalidTimerFire)?;
        Ok((scheduled, deadline))
    }

    fn retry_timer_checkpoint(
        &self,
        run: RunId,
        preflight: &ActionFailurePreflight,
        state: &RunState,
    ) -> RuntimeResult<crate::run_checkpoint::RunCheckpointV1> {
        let admission = state
            .admission
            .as_ref()
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        let failed = Self::failed_retry_state(state, preflight)?;
        crate::run_checkpoint::RunCheckpointV1::capture(
            run,
            admission.artifact_digest(),
            state.run_attempt,
            preflight.ticket.step,
            self.journal_sequence_for(run).get(),
            &failed,
        )
        .map_err(|error| RuntimeError::Recovery {
            error: error.to_string(),
        })
    }

    fn failed_retry_state(
        state: &RunState,
        preflight: &ActionFailurePreflight,
    ) -> RuntimeResult<RunState> {
        let mut failed = state.clone();
        failed
            .frame
            .mark_failed(preflight.ticket.step)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        *failed
            .action_attempts
            .get_mut(preflight.ticket.step.as_usize())
            .ok_or(RuntimeError::InvalidActionCompletion)? = preflight.next_attempt;
        Ok(failed)
    }

    fn retry_timer_event_with_checkpoint(
        &self,
        context: &RetryTimerEventContext<'_>,
        mut checkpoint: crate::run_checkpoint::RunCheckpointV1,
    ) -> RuntimeResult<RuntimeJournalEvent> {
        for _ in 0..32 {
            let (event, projected) = self.retry_timer_event_candidate(context, &checkpoint)?;
            if checkpoint.journal_bytes_written == projected {
                return Ok(event);
            }
            checkpoint.journal_bytes_written = projected;
        }
        Err(RuntimeError::EncodeFailed)
    }

    fn retry_timer_event_candidate(
        &self,
        context: &RetryTimerEventContext<'_>,
        checkpoint: &crate::run_checkpoint::RunCheckpointV1,
    ) -> RuntimeResult<(RuntimeJournalEvent, u64)> {
        let payload = postcard::to_allocvec(checkpoint).map_err(|_| RuntimeError::EncodeFailed)?;
        let event = RuntimeJournalEvent::RetryTimerScheduled {
            run: context.run,
            step: context.preflight.ticket.step,
            timer: vb_storage::retry_timer::RetryTimerV1 {
                version: 1,
                failed_ticket: context.preflight.ticket,
                next_attempt: context.preflight.next_attempt,
                generation: context.timer.generation,
                scheduled_unix_ms: context.scheduled_unix_ms,
                deadline_unix_ms: context.deadline_unix_ms,
                checkpoint_digest: *blake3::hash(&payload).as_bytes(),
                checkpoint: payload,
            },
        };
        let projected = self.retry_timer_projected_bytes(context, &event)?;
        Ok((event, projected))
    }
    fn retry_timer_projected_bytes(
        &self,
        context: &RetryTimerEventContext<'_>,
        event: &RuntimeJournalEvent,
    ) -> RuntimeResult<u64> {
        let charge = self.journal_batch_bytes(&[context.failed_event.clone(), event.clone()])?;
        context
            .base_bytes
            .checked_add(charge)
            .ok_or(RuntimeError::EncodeFailed)
    }
}
