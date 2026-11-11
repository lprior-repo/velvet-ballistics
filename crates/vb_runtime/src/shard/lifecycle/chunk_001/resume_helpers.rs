impl Shard {
    /// Re-arms a recovered timer suspension from its timeout slot.
    ///
    /// Covers waiting waits and timed asks. Wall-clock instants are not
    /// durable, so the deadline is the full timeout from the recovery point
    /// (the same helper the live drive path uses), never an expired instant
    /// that would fire spuriously.
    fn restore_recovered_timer(
        &mut self,
        run: RunId,
        step: StepIdx,
        kind: PendingTimerKind,
    ) -> RuntimeResult<()> {
        let deadline = match self.run_state_get(run) {
            Some(state) => awaiting_deadline(state, step),
            None => return Err(RuntimeError::RunNotFound),
        };
        let generation = match self.next_pending_timer_generation(run) {
            Some(generation) => generation,
            None => return Err(RuntimeError::InvalidTimerFire),
        };
        self.pending_timer_insert(
            run,
            PendingTimer {
                step,
                kind,
                generation,
                deadline,
            },
        )?;
        Ok(())
    }

    fn validate_run_exists(&self, run: RunId) -> Result<(), ResumeError> {
        if !self.run_state_contains(run) {
            return Err(ResumeError::RunIdNotFound { run_id: run });
        }
        Ok(())
    }

    fn get_runtime_state_or_running(&self, run: RunId) -> RuntimeState {
        self.runtime_state_get(run).unwrap_or(RuntimeState::Running)
    }

    fn append_resumed_event(&mut self, run: RunId) -> Result<u64, ResumeError> {
        if !self.is_run_tracked(run) {
            return Err(ResumeError::IncompleteHydration { run_id: run });
        }
        self.apply(run, RuntimeEvent::Resume)
            .map_err(ResumeError::journal_append_failed_with_source)?;
        let timestamp = current_timestamp();
        let resumed_event = RuntimeJournalEvent::Resumed { run, timestamp };
        if let Err(source) = self.record_command_journal_events(run, [resumed_event]) {
            self.apply(run, RuntimeEvent::ResumeRollback)
                .map_err(ResumeError::journal_append_failed_with_source)?;
            return Err(ResumeError::journal_append_failed_with_source(source));
        }
        Ok(timestamp)
    }

    fn is_run_tracked(&self, run: RunId) -> bool {
        self.runtime_state_get(run).is_some()
    }

    fn observe_resume_drive_result(
        &mut self,
        run: RunId,
        result: RuntimeResult<()>,
    ) -> Result<(), ResumeError> {
        if let Err(source) = result {
            return Err(self.restore_resumable_after_drive_failure(run, source));
        }
        Ok(())
    }

    fn restore_resumable_after_drive_failure(
        &mut self,
        run: RunId,
        source: RuntimeError,
    ) -> ResumeError {
        match self.apply(run, RuntimeEvent::ResumeRollback) {
            Ok(()) => ResumeError::journal_append_failed_with_source(source),
            Err(rollback_error) => ResumeError::journal_append_failed_with_source(rollback_error),
        }
    }
}
