enum DriveApplyFailure {
    BeforeCommit(RuntimeError),
    AfterCommit(RuntimeError),
}

type DriveJournalEvents = Vec<RuntimeJournalEvent>;
type DriveTraceEvents = Vec<TraceEvent>;
type AwaitingActionPlan = (
    DriveJournalEvents,
    DriveTraceEvents,
    RunState,
    ActionTicket,
    StepIdx,
);
type AwaitingTimerPlan = (
    DriveJournalEvents,
    DriveTraceEvents,
    RunState,
    Option<PendingTimer>,
);

impl Shard {
    pub(crate) fn handle_timer(
        &mut self,
        run: RunId,
        generation: u64,
        deadline: std::time::Instant,
        kind: PendingTimerKind,
    ) -> RuntimeResult<()> {
        let Some(current_timer) = self.pending_timer_get(run) else {
            return Err(RuntimeError::InvalidTimerFire);
        };
        if !current_timer.matches_authority(generation, deadline, kind) {
            return Err(RuntimeError::InvalidTimerFire);
        }
        let state_ref = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        crate::shard::helpers::validate_timer_fire(state_ref, current_timer)?;
        self.append_timer_resolution_event(run, current_timer)?;
        let mut state = self.take_run_state(run)?;
        let timer = match self.pending_timer_remove(run) {
            Some(timer) => timer,
            None => {
                self.run_state_insert(run, state)?;
                return Err(RuntimeError::InvalidTimerFire);
            }
        };
        crate::shard::helpers::advance_after_timer_fire(&mut state, timer)?;
        let rollback_state = state.clone();
        let mut evidence = EvidenceCollector::new();
        let result = Self::drive_state(&mut state, self.step_budget_per_tick, &mut evidence);
        match self.apply_drive_result(run, state, result, &mut evidence) {
            Ok(()) => Ok(()),
            Err(DriveApplyFailure::BeforeCommit(error)) => {
                self.restore_run_state_after_drive_failure(run, rollback_state, error)
            }
            Err(DriveApplyFailure::AfterCommit(error)) => Err(error),
        }
    }

    fn append_timer_resolution_event(
        &mut self,
        run: RunId,
        timer: PendingTimer,
    ) -> RuntimeResult<()> {
        match timer.kind {
            PendingTimerKind::Wait => self.append_journal_event(RuntimeJournalEvent::WaitResolved {
                run,
                step: timer.step,
            }),
            PendingTimerKind::Ask => self.append_journal_event(RuntimeJournalEvent::AskTimedOut {
                run,
                step: timer.step,
            }),
        }
    }

    pub(crate) fn handle_cancel(
        &mut self,
        run: RunId,
        reason: Option<String>,
    ) -> RuntimeResult<()> {
        // C2: Reject missing runs with a typed error.
        if !self.run_state_contains(run) && !self.terminal_runs_contains(run) {
            return Err(RuntimeError::RunNotFound);
        }
        if self.run_state_contains(run) {
            self.emit_action_abandoned_for_pending(run)?;
            self.append_journal_event(RuntimeJournalEvent::RunCancelled { run, reason })?;
            let _removed_timer = self.pending_timer_remove(run);
            let Some(state) = self.run_state_remove(run) else {
                return Err(RuntimeError::RunNotFound);
            };
            self.release_frame(state.frame);
            self.terminal_runs_insert(run)?;
            self.action_abi_digests_remove(run);
            self.runtime_state_remove(run);
            self.counters.inc_failed();
            self.trace_ring.push(TraceEvent::RunCancelled { run });
            self.clear_executed_step_accounting(run);
        }
        self.discard_journal_sequence(run);
        Ok(())
    }

    pub(crate) fn handle_kill(&mut self, run: RunId, _reason: Option<String>) -> RuntimeResult<()> {
        // C2: Reject missing runs with a typed error.
        if !self.run_state_contains(run) && !self.terminal_runs_contains(run) {
            return Err(RuntimeError::RunNotFound);
        }
        if self.run_state_contains(run) {
            self.emit_action_abandoned_for_pending(run)?;
            self.append_journal_event(RuntimeJournalEvent::RunKilled { run })?;
            let _removed_timer = self.pending_timer_remove(run);
        }
        if let Some(state) = self.run_state_remove(run) {
            self.release_frame(state.frame);
            self.terminal_runs_insert(run)?;
            self.action_abi_digests_remove(run);
            self.runtime_state_remove(run);
            self.counters.inc_failed();
            self.trace_ring.push(TraceEvent::RunKilled { run });
            self.clear_executed_step_accounting(run);
        }
        self.discard_journal_sequence(run);
        Ok(())
    }

    pub(crate) fn handle_inspect(&mut self, run: RunId, correlation: u64) {
        self.inspect_response = Some(self.snapshot_run(run, correlation));
    }
}
