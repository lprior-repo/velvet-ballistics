impl Shard {
    fn resolving_retry_timer(&self, run: RunId, state: &RunState) -> bool {
        let step = state.frame.pc();
        self.pending_timer_get(run)
            .is_some_and(|timer| timer.step == step)
            && state
                .workflow
                .node(step)
                .is_some_and(|node| node.retry.is_some())
    }

    fn append_retry_timer_resolution(
        &self,
        run: RunId,
        state: &RunState,
        events: &mut Vec<RuntimeJournalEvent>,
    ) -> RuntimeResult<()> {
        if self.resolving_retry_timer(run, state) {
            Self::push_drive_journal_event(
                events,
                RuntimeJournalEvent::WaitResolved {
                    run,
                    step: state.frame.pc(),
                },
            )?;
        }
        Ok(())
    }
    fn prepare_retry_timer(
        &mut self,
        run: RunId,
        preflight: &ActionFailurePreflight,
    ) -> RuntimeResult<Option<RetryTimerPlan>> {
        if preflight.outcome != ActionFailureOutcome::RetryNow || preflight.delay_ms == 0 {
            return Ok(None);
        }
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        if let Some(budget) = state
            .admission
            .as_ref()
            .and_then(|admission| admission.budget())
            && budget.max_timer_entries == 0
        {
            return Err(RuntimeError::UnsupportedOperation {
                operation: "retry_timer_budget",
            });
        }
        self.reserve_pending_timer_slot(run)?;
        let generation = self
            .journal_sequence_for(run)
            .get()
            .checked_add(1)
            .ok_or(RuntimeError::InvalidTimerFire)?;
        let deadline = std::time::Instant::now()
            .checked_add(std::time::Duration::from_millis(preflight.delay_ms))
            .ok_or(RuntimeError::InvalidTimerFire)?;
        let mut timer = PendingTimer {
            step: preflight.ticket.step,
            kind: PendingTimerKind::Wait,
            generation,
            deadline,
        };
        let event = self.retry_timer_event(run, preflight, timer)?;
        let RuntimeJournalEvent::RetryTimerScheduled { timer: durable, .. } = &event else {
            return Err(RuntimeError::InvalidTimerFire);
        };
        timer.deadline = retry_deadline(durable.deadline_unix_ms)?;
        Ok(Some(RetryTimerPlan { timer, event }))
    }

    fn pending_retry_timer(&self, run: RunId, step: StepIdx) -> bool {
        self.pending_timer_get(run)
            .is_some_and(|timer| timer.step == step)
            && self.run_state_get(run).is_some_and(|state| {
                state.workflow.node(step).is_some_and(|node| {
                    matches!(node.kind, vb_core::CompiledNodeKind::Do { .. })
                        && node.retry.is_some()
                })
            })
    }

    fn handle_retry_timer(&mut self, run: RunId, timer: PendingTimer) -> RuntimeResult<()> {
        self.can_retry(run)?;
        let mut state = self.take_run_state(run)?;
        let rollback = state.clone();
        crate::shard::helpers::advance_after_timer_fire(&mut state, timer)?;
        let mut evidence = EvidenceCollector::new();
        let result = self.drive_state(run, &mut state, self.step_budget_per_tick, &mut evidence);
        match self.apply_drive_result(run, state, result, &mut evidence) {
            Ok(()) => Ok(()),
            Err(DriveApplyFailure::BeforeCommit(error)) => {
                self.restore_run_state_after_drive_failure(run, rollback, error)
            }
            Err(DriveApplyFailure::AfterCommit(error)) => Err(error),
        }
    }
}
