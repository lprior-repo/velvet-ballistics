struct RetryTimerPlan {
    timer: PendingTimer,
    event: RuntimeJournalEvent,
}

impl Shard {
    fn retry_timer_event(
        &self,
        run: RunId,
        preflight: &ActionFailurePreflight,
        timer: PendingTimer,
    ) -> RuntimeResult<RuntimeJournalEvent> {
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        let admission = state
            .admission
            .as_ref()
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        let mut failed = state.clone();
        failed
            .frame
            .mark_failed(preflight.ticket.step)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        *failed
            .action_attempts
            .get_mut(preflight.ticket.step.as_usize())
            .ok_or(RuntimeError::InvalidActionCompletion)? = preflight.next_attempt;
        let mut checkpoint = crate::run_checkpoint::RunCheckpointV1::capture(
            run,
            admission.artifact_digest(),
            state.run_attempt,
            preflight.ticket.step,
            self.journal_sequence_for(run).get(),
            &failed,
        )
        .map_err(|error| RuntimeError::Recovery {
            error: error.to_string(),
        })?;
        let scheduled_unix_ms = crate::shard::helpers::wall_clock_millis_since_epoch()
            .ok_or(RuntimeError::InvalidTimerFire)?;
        let deadline_unix_ms = scheduled_unix_ms
            .checked_add(preflight.delay_ms)
            .ok_or(RuntimeError::InvalidTimerFire)?;
        let failed_event = RuntimeJournalEvent::ActionFailed {
            run,
            step: preflight.ticket.step,
            action: preflight.ticket.action,
            attempt: preflight.ticket.attempt,
        };
        // The checkpoint includes this batch's charge. Its varint length reaches
        // a fixed point within 32 rounds, as for the run-retry checkpoint path.
        for _ in 0..32 {
            let payload =
                postcard::to_allocvec(&checkpoint).map_err(|_| RuntimeError::EncodeFailed)?;
            let event = RuntimeJournalEvent::RetryTimerScheduled {
                run,
                step: preflight.ticket.step,
                timer: vb_storage::retry_timer::RetryTimerV1 {
                    version: 1,
                    failed_ticket: preflight.ticket,
                    next_attempt: preflight.next_attempt,
                    generation: timer.generation,
                    scheduled_unix_ms,
                    deadline_unix_ms,
                    checkpoint_digest: *blake3::hash(&payload).as_bytes(),
                    checkpoint: payload,
                },
            };
            let charge = Self::journal_batch_bytes(&[failed_event.clone(), event.clone()])?;
            let projected = state
                .journal_bytes_written
                .checked_add(charge)
                .ok_or(RuntimeError::EncodeFailed)?;
            if checkpoint.journal_bytes_written == projected {
                return Ok(event);
            }
            checkpoint.journal_bytes_written = projected;
        }
        Err(RuntimeError::EncodeFailed)
    }
}
