impl Shard {
    fn commit_awaiting_action(
        &mut self,
        run: RunId,
        state: RunState,
        ticket: ActionTicket,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        let ticket = self.fenced_action_ticket(run, ticket);
        if evidence.is_empty() && !self.resolving_retry_timer(run, &state) {
            return self.apply_unjournaled_action(run, state, ticket);
        }
        self.commit_journaled_action(run, state, ticket, evidence)
    }

    fn fenced_action_ticket(&mut self, run: RunId, ticket: ActionTicket) -> ActionTicket {
        ActionTicket {
            fencing_token: self.fencing_tokens.mint(run),
            ..ticket
        }
    }

    fn apply_unjournaled_action(
        &mut self,
        run: RunId,
        state: RunState,
        ticket: ActionTicket,
    ) -> Result<(), DriveApplyFailure> {
        self.apply(run, RuntimeEvent::AwaitAction)
            .map_err(DriveApplyFailure::AfterCommit)?;
        let outcome = self
            .await_action(run, state, ticket)
            .map_err(DriveApplyFailure::AfterCommit)?;
        match outcome {
            TransitionOutcome::JournalBudgetExceeded(state) => {
                self.fail_run_on_journal_budget(run, *state, Vec::new())
            }
            _ => Ok(()),
        }
    }

    fn commit_journaled_action(
        &mut self,
        run: RunId,
        state: RunState,
        ticket: ActionTicket,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        let plan = self
            .prepare_awaiting_action(run, state, ticket, evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.commit_awaiting_action_plan(run, plan)
    }

    fn commit_awaiting_action_plan(
        &mut self,
        run: RunId,
        (journal_events, trace_events, mut state, ticket, step): AwaitingActionPlan,
    ) -> Result<(), DriveApplyFailure> {
        if !self
            .charge_drive_journal_bytes(&mut state, &journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?
        {
            return self.fail_run_on_journal_budget(run, state, trace_events);
        }
        self.append_journal_event_batch(&journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.push_trace_events(trace_events);
        self.trace_ring
            .push(TraceEvent::ActionScheduled { run, step });
        self.add_executed_step_delta(run, state.frame.executed());
        self.run_state_insert(run, state)
            .map_err(DriveApplyFailure::AfterCommit)?;
        self.pending_action_insert(run, ticket)
            .map_err(DriveApplyFailure::AfterCommit)?;
        self.clear_matching_action_timer(run, step);
        self.apply(run, RuntimeEvent::AwaitAction)
            .map_err(DriveApplyFailure::AfterCommit)
    }

    fn clear_matching_action_timer(&mut self, run: RunId, step: StepIdx) {
        if self
            .pending_timer_get(run)
            .is_some_and(|timer| timer.step == step)
        {
            self.pending_timer_remove(run);
        }
    }

    fn commit_awaiting_timer(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        if evidence.is_empty() {
            return self.apply_unjournaled_timer(run, state, kind);
        }
        self.commit_journaled_timer(run, state, kind, evidence)
    }

    fn apply_unjournaled_timer(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
    ) -> Result<(), DriveApplyFailure> {
        let outcome = self
            .await_timer(run, state, kind)
            .map_err(DriveApplyFailure::AfterCommit)?;
        if let TransitionOutcome::JournalBudgetExceeded(state) = outcome {
            return self.fail_run_on_journal_budget(run, *state, Vec::new());
        }
        self.apply(run, RuntimeEvent::AwaitTimer)
            .map_err(DriveApplyFailure::AfterCommit)
    }

    fn commit_journaled_timer(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        if kind == PendingTimerKind::Ask {
            self.reserve_pending_ask_authority_slot(run)
                .map_err(DriveApplyFailure::BeforeCommit)?;
        }
        let plan = self
            .prepare_awaiting_timer(run, state, kind, evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.commit_awaiting_timer_plan(run, plan)
    }

    fn commit_awaiting_timer_plan(
        &mut self,
        run: RunId,
        (journal_events, trace_events, mut state, timer, ask_boundary_seq): AwaitingTimerPlan,
    ) -> Result<(), DriveApplyFailure> {
        if !self
            .charge_drive_journal_bytes(&mut state, &journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?
        {
            return self.fail_run_on_journal_budget(run, state, trace_events);
        }
        self.append_journal_event_batch(&journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.record_timer_ask_boundary(run, &state, ask_boundary_seq);
        self.push_trace_events(trace_events);
        self.add_executed_step_delta(run, state.frame.executed());
        self.run_state_insert(run, state)
            .map_err(DriveApplyFailure::AfterCommit)?;
        self.insert_pending_timer(run, timer)?;
        self.apply(run, RuntimeEvent::AwaitTimer)
            .map_err(DriveApplyFailure::AfterCommit)
    }

    fn record_timer_ask_boundary(
        &mut self,
        run: RunId,
        state: &RunState,
        sequence: Option<vb_storage::EventSeq>,
    ) {
        if let Some(boundary_seq) = sequence {
            self.record_pending_ask_authority(run, state.frame.pc(), boundary_seq);
        }
    }

    fn insert_pending_timer(
        &mut self,
        run: RunId,
        timer: Option<PendingTimer>,
    ) -> Result<(), DriveApplyFailure> {
        if let Some(timer) = timer {
            self.pending_timer_insert(run, timer)
                .map_err(DriveApplyFailure::AfterCommit)?;
        }
        Ok(())
    }
}
