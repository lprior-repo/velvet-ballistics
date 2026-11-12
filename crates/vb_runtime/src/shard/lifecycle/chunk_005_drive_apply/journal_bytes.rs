impl Shard {
    fn journal_batch_bytes(events: &[RuntimeJournalEvent]) -> RuntimeResult<u64> {
        let mut total: u64 = 0;
        for event in events {
            let encoded = postcard::to_allocvec(event).map_err(|_| RuntimeError::EncodeFailed)?;
            let len = u64::try_from(encoded.len()).map_err(|_| RuntimeError::EncodeFailed)?;
            total = total
                .checked_add(len)
                .ok_or(RuntimeError::EncodeFailed)?;
        }
        Ok(total)
    }

    pub(crate) fn journal_event_bytes(event: &RuntimeJournalEvent) -> RuntimeResult<u64> {
        Self::journal_batch_bytes(core::slice::from_ref(event))
    }

    fn charge_drive_journal_bytes(
        state: &mut RunState,
        events: &[RuntimeJournalEvent],
    ) -> RuntimeResult<bool> {
        let next = Self::journal_bytes_after_append(state, events)?;
        if Self::journal_bytes_exceeds_budget(state, next) {
            return Ok(false);
        }
        state.journal_bytes_written = next;
        Ok(true)
    }

    pub(crate) fn record_drive_journal_bytes(
        state: &mut RunState,
        events: &[RuntimeJournalEvent],
    ) -> RuntimeResult<()> {
        state.journal_bytes_written = Self::journal_bytes_after_append(state, events)?;
        Ok(())
    }

    pub(crate) fn journal_bytes_after_append(
        state: &RunState,
        events: &[RuntimeJournalEvent],
    ) -> RuntimeResult<u64> {
        let bytes = Self::journal_batch_bytes(events)?;
        state
            .journal_bytes_written
            .checked_add(bytes)
            .ok_or(RuntimeError::EncodeFailed)
    }

    pub(crate) fn journal_bytes_exceeds_budget(state: &RunState, projected: u64) -> bool {
        let Some(budget) = state.admission.as_ref().and_then(|a| a.budget()) else {
            return false;
        };
        projected > budget.max_journal_bytes_total
    }

    fn command_journal_bytes_exceeds(
        state: &RunState,
        events: &[RuntimeJournalEvent],
    ) -> RuntimeResult<bool> {
        let bytes = Self::journal_batch_bytes(events)?;
        let next = state
            .journal_bytes_written
            .checked_add(bytes)
            .ok_or(RuntimeError::EncodeFailed)?;
        Ok(state
            .admission
            .as_ref()
            .and_then(|admission| admission.budget())
            .is_some_and(|budget| next > budget.max_journal_bytes_total))
    }

    fn record_command_journal_events<const N: usize>(
        &mut self,
        run: RunId,
        events: [RuntimeJournalEvent; N],
    ) -> RuntimeResult<()> {
        let bytes = Self::journal_batch_bytes(&events)?;
        self.append_journal_events_atomically(events)?;
        let state = self
            .run_state_get_mut(run)
            .ok_or(RuntimeError::RunNotFound)?;
        state.journal_bytes_written = state
            .journal_bytes_written
            .checked_add(bytes)
            .ok_or(RuntimeError::EncodeFailed)?;
        Ok(())
    }

    fn append_command_journal_events<const N: usize>(
        &mut self,
        run: RunId,
        events: [RuntimeJournalEvent; N],
    ) -> RuntimeResult<bool> {
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        if Self::command_journal_bytes_exceeds(state, &events)? {
            return Ok(true);
        }
        self.record_command_journal_events(run, events)?;
        Ok(false)
    }

    fn fail_command_run_on_journal_budget(&mut self, run: RunId) -> RuntimeResult<()> {
        let state = self.take_run_state(run)?;
        let rollback = state.clone();
        match self.fail_run_on_journal_budget(run, state, Vec::new()) {
            Ok(()) => Ok(()),
            Err(DriveApplyFailure::BeforeCommit(error)) => {
                if let Err(rollback_error) = self.run_state_insert(run, rollback) {
                    return Err(RuntimeError::rollback_failed(
                        "command_journal_budget",
                        error,
                        rollback_error,
                    ));
                }
                Err(error)
            }
            Err(DriveApplyFailure::AfterCommit(error)) => Err(error),
        }
    }

    fn fail_run_on_journal_budget(
        &mut self,
        run: RunId,
        mut state: RunState,
        trace_events: Vec<TraceEvent>,
    ) -> Result<(), DriveApplyFailure> {
        let fallback = [RuntimeJournalEvent::RunFailed {
            run,
            attempt: state.run_attempt.get(),
        }];
        Self::record_drive_journal_bytes(&mut state, &fallback)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.append_journal_event_batch(&fallback)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.push_trace_events(trace_events);
        let retry_step = state.frame.pc();
        self.fail_run_state_with_retry_checkpoint(run, retry_step, state)
            .map_err(DriveApplyFailure::AfterCommit)?;
        self.apply(run, RuntimeEvent::Fail)
            .map_err(DriveApplyFailure::AfterCommit)
    }
}
