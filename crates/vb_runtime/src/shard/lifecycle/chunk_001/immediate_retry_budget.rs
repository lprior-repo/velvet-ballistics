impl Shard {
    fn commit_immediate_retry_budget_failure(
        &mut self,
        run: RunId,
        preflight: &ActionFailurePreflight,
        code: vb_core::ActionFailureCode,
    ) -> RuntimeResult<()> {
        self.reserve_terminal_run_slot(run)?;
        self.reserve_checked_out_run_slot(run)?;
        let (events, projected) = self.immediate_retry_budget_events(run, preflight)?;
        self.append_journal_event_batch(&events)?;
        let mut state = self.take_run_state(run)?;
        state.journal_bytes_written = projected;
        apply_action_failure_preflight(&mut state, preflight)?;
        self.record_action_failure(run, preflight, code);
        self.fail_run_state_with_retry_checkpoint(run, preflight.ticket.step, state)?;
        self.apply(run, RuntimeEvent::Fail)
    }

    fn immediate_retry_budget_events(
        &self,
        run: RunId,
        preflight: &ActionFailurePreflight,
    ) -> RuntimeResult<(Vec<RuntimeJournalEvent>, u64)> {
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        let failure = action_failed_event(run, preflight);
        let terminal = RuntimeJournalEvent::RunFailed {
            run,
            attempt: state.run_attempt.get(),
        };
        let mut events = Vec::new();
        if !self.command_journal_bytes_exceeds(state, &[failure.clone(), terminal.clone()])? {
            Self::push_drive_journal_event(&mut events, failure)?;
        }
        Self::push_drive_journal_event(&mut events, terminal)?;
        let projected = self.journal_bytes_after_append(state, &events)?;
        Ok((events, projected))
    }
}
