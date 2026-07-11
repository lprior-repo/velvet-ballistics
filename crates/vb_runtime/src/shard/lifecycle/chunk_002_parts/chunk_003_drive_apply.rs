impl Shard {
    fn apply_awaiting_action(
        &mut self,
        run: RunId,
        state: RunState,
        ticket: ActionTicket,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        if evidence.is_empty() {
            self.apply(run, RuntimeEvent::AwaitAction)
                .map_err(DriveApplyFailure::AfterCommit)?;
            return self
                .await_action(run, state, ticket)
                .map_err(DriveApplyFailure::AfterCommit);
        }
        let (journal_events, trace_events, state, ticket, step) = self
            .prepare_awaiting_action(run, state, ticket, evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
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
        self.apply(run, RuntimeEvent::AwaitAction)
            .map_err(DriveApplyFailure::AfterCommit)
    }

    fn apply_awaiting_timer(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        if evidence.is_empty() {
            self.await_timer(run, state, kind)
                .map_err(DriveApplyFailure::AfterCommit)?;
            return self
                .apply(run, RuntimeEvent::AwaitTimer)
                .map_err(DriveApplyFailure::AfterCommit);
        }
        let (journal_events, trace_events, state, timer) = self
            .prepare_awaiting_timer(run, state, kind, evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.append_journal_event_batch(&journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.push_trace_events(trace_events);
        self.add_executed_step_delta(run, state.frame.executed());
        self.run_state_insert(run, state)
            .map_err(DriveApplyFailure::AfterCommit)?;
        if let Some(timer) = timer {
            self.pending_timer_insert(run, timer)
                .map_err(DriveApplyFailure::AfterCommit)?;
        }
        self.apply(run, RuntimeEvent::AwaitTimer)
            .map_err(DriveApplyFailure::AfterCommit)
    }

    fn apply_terminal_finished(
        &mut self,
        run: RunId,
        state: RunState,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        if evidence.is_empty() {
            self.finish_run(run, state)
                .map_err(DriveApplyFailure::AfterCommit)?;
            return self
                .apply(run, RuntimeEvent::DriveFinished)
                .map_err(DriveApplyFailure::AfterCommit);
        }
        let result = match crate::shard::helpers::result_slot_for_finished_run(&state) {
            Some(slot) => slot,
            None => SlotIdx::ZERO,
        };
        let (mut journal_events, trace_events) = self
            .prepare_evidence_events(run, evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        Self::push_drive_journal_event(
            &mut journal_events,
            RuntimeJournalEvent::RunFinished { run, result },
        )
        .map_err(DriveApplyFailure::BeforeCommit)?;
        self.append_journal_event_batch(&journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.push_trace_events(trace_events);
        self.finish_run_after_journaled(run, state)
            .map_err(DriveApplyFailure::AfterCommit)?;
        self.apply(run, RuntimeEvent::DriveFinished)
            .map_err(DriveApplyFailure::AfterCommit)
    }

    fn apply_terminal_failed(
        &mut self,
        run: RunId,
        state: RunState,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        if evidence.is_empty() {
            self.fail_run_state(run, state)
                .map_err(DriveApplyFailure::AfterCommit)?;
            return self
                .apply(run, RuntimeEvent::Fail)
                .map_err(DriveApplyFailure::AfterCommit);
        }
        let (mut journal_events, trace_events) = self
            .prepare_evidence_events(run, evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        Self::push_drive_journal_event(
            &mut journal_events,
            RuntimeJournalEvent::RunFailed { run },
        )
        .map_err(DriveApplyFailure::BeforeCommit)?;
        self.append_journal_event_batch(&journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.push_trace_events(trace_events);
        self.fail_run_state_after_journaled(run, state)
            .map_err(DriveApplyFailure::AfterCommit)?;
        self.apply(run, RuntimeEvent::Fail)
            .map_err(DriveApplyFailure::AfterCommit)
    }
}
