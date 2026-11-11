impl Shard {
    pub(crate) fn handle_timer(
        &mut self,
        run: RunId,
        generation: u64,
        deadline: std::time::Instant,
        kind: PendingTimerKind,
    ) -> RuntimeResult<()> {
        self.checked_timer_with_clock(run, generation, deadline, kind, || {
            std::time::Instant::now()
        })
    }

    pub(super) fn handle_timer_at(
        &mut self,
        run: RunId,
        generation: u64,
        deadline: std::time::Instant,
        kind: PendingTimerKind,
        now: std::time::Instant,
    ) -> RuntimeResult<()> {
        self.checked_timer_with_clock(run, generation, deadline, kind, || now)
    }

    fn checked_timer_with_clock<F>(
        &mut self,
        run: RunId,
        generation: u64,
        deadline: std::time::Instant,
        kind: PendingTimerKind,
        clock: F,
    ) -> RuntimeResult<()>
    where
        F: FnOnce() -> std::time::Instant,
    {
        let Some(current_timer) = self.pending_timer_get(run) else {
            return Err(RuntimeError::InvalidTimerFire);
        };
        if !current_timer.matches_authority(generation, deadline, kind) {
            return Err(RuntimeError::InvalidTimerFire);
        }
        let now = clock();
        if current_timer.deadline > now {
            return Err(RuntimeError::InvalidTimerFire);
        }
        self.execute_timer_fire(run, current_timer)
    }

    fn execute_timer_fire(&mut self, run: RunId, timer: PendingTimer) -> RuntimeResult<()> {
        let state_ref = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        crate::shard::helpers::validate_timer_fire(state_ref, timer)?;
        let budget_exceeded = self.append_timer_resolution_event(run, timer)?;
        if timer.kind == PendingTimerKind::Ask {
            self.clear_pending_ask_authority(run);
        }
        if budget_exceeded {
            return self.fail_command_run_on_journal_budget(run);
        }
        self.commit_timer_fire(run)
    }

    fn commit_timer_fire(&mut self, run: RunId) -> RuntimeResult<()> {
        let mut state = self.take_run_state(run)?;
        let removed_timer = match self.pending_timer_remove(run) {
            Some(t) => t,
            None => {
                self.run_state_insert(run, state)?;
                return Err(RuntimeError::InvalidTimerFire);
            }
        };
        crate::shard::helpers::advance_after_timer_fire(&mut state, removed_timer)?;
        let rollback_state = state.clone();
        let mut evidence = EvidenceCollector::new();
        let result = self.drive_state(run, &mut state, self.step_budget_per_tick, &mut evidence);
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
    ) -> RuntimeResult<bool> {
        let event = match timer.kind {
            PendingTimerKind::Wait => RuntimeJournalEvent::WaitResolved {
                run,
                step: timer.step,
            },
            PendingTimerKind::Ask => RuntimeJournalEvent::AskTimedOut {
                run,
                step: timer.step,
            },
        };
        self.append_command_journal_events(run, [event])
    }
}
