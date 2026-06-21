impl Shard {
    // =============================================================================
    // Resume lifecycle methods
    // =============================================================================

    pub fn handle_resume(&mut self, run: RunId) -> Result<ResumeResult, ResumeError> {
        self.validate_run_exists(run)?;
        let current_state = self.get_runtime_state_or_running(run);
        // RQ-W0-20: accept Resuming as a recoverable state. A run in
        // Resuming implies a previous `handle_resume` call appended the
        // Resumed event successfully but never reached the drive step
        // (e.g., process crash or external inspection). Treating Resuming
        // as recoverable lets the next handle_resume drive the run
        // forward without re-appending the durable Resumed event.
        if current_state != RuntimeState::Resumable
            && current_state != RuntimeState::Resuming
        {
            return Err(ResumeError::NotResumable {
                run_id: run,
                current_state,
            });
        }
        let timestamp = if current_state == RuntimeState::Resumable {
            self.append_resumed_event(run)?
        } else {
            // Resuming: the Resumed event was already appended on the
            // prior attempt. Skip re-appending to avoid duplicate-event
            // failures from the durable journal.
            current_timestamp()
        };
        let drive_result = self.drive_run(run);
        self.observe_resume_drive_result(run, drive_result)?;
        Ok(ResumeResult {
            run_id: run,
            status: ResumeStatus::Resumed,
            timestamp,
        })
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
        // State guard already enforced above; this apply call is a
        // defense-in-depth check that mirrors the FSM contract.
        if let Err(source) = self.apply(run, RuntimeEvent::Resume) {
            return Err(ResumeError::journal_append_failed_with_source(source));
        }
        let timestamp = current_timestamp();
        let resumed_event = RuntimeJournalEvent::Resumed { run, timestamp };
        if let Err(source) = self.append_journal_event(resumed_event) {
            if let Err(rollback_err) = self.apply(run, RuntimeEvent::ResumeRollback) {
                // If rollback fails, prefer the original journal source error.
                let _ = rollback_err;
            }
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
        let _ = self.apply(run, RuntimeEvent::ResumeRollback);
        ResumeError::journal_append_failed_with_source(source)
    }
}