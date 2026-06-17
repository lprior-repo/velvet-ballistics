impl Shard {
    // =============================================================================
    // Resume lifecycle methods
    // =============================================================================

    pub fn handle_resume(&mut self, run: RunId) -> Result<ResumeResult, ResumeError> {
        self.validate_run_exists(run)?;
        let current_state = self.get_runtime_state_or_running(run);
        if current_state != RuntimeState::Resumable {
            return Err(ResumeError::NotResumable {
                run_id: run,
                current_state,
            });
        }
        let timestamp = self.append_resumed_event(run)?;
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
        self.apply(run, RuntimeEvent::Resume);
        let timestamp = current_timestamp();
        let resumed_event = RuntimeJournalEvent::Resumed { run, timestamp };
        if let Err(source) = self.append_journal_event(resumed_event) {
            self.apply(run, RuntimeEvent::ResumeRollback);
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
        self.apply(run, RuntimeEvent::ResumeRollback);
        ResumeError::journal_append_failed_with_source(source)
    }
}