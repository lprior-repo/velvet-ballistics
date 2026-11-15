use crate::shard::transitions::TransitionOutcome;

struct AwaitingSchedule<'a> {
    run: RunId,
    state: &'a RunState,
    step: StepIdx,
    kind: PendingTimerKind,
    register_timer: bool,
    journal_events: &'a mut Vec<RuntimeJournalEvent>,
}

impl Shard {
    fn prepare_awaiting_action(
        &self,
        run: RunId,
        mut state: RunState,
        ticket: ActionTicket,
        evidence: &mut EvidenceCollector,
    ) -> RuntimeResult<AwaitingActionPlan> {
        let (ticket, step) = Self::prepare_scheduled_action_ticket(&mut state, ticket)?;
        let output = crate::shard::helpers::action_output_slot(&state, ticket.step)?;
        let input = crate::shard::helpers::action_input_slot(&state, ticket.step)?;
        let (mut journal_events, trace_events) =
            self.prepare_evidence_events(run, state.run_attempt.get(), evidence)?;
        self.append_retry_timer_resolution(run, &state, &mut journal_events)?;
        Self::push_drive_journal_event(
            &mut journal_events,
            RuntimeJournalEvent::ActionScheduledTicket {
                ticket,
                input,
                output,
                action_abi_digest: schedule_action_abi_digest(&state, ticket.action)?,
            },
        )?;
        Ok((journal_events, trace_events, state, ticket, step))
    }

    fn prepare_scheduled_action_ticket(
        state: &mut RunState,
        ticket: ActionTicket,
    ) -> RuntimeResult<(ActionTicket, StepIdx)> {
        let step = state.frame.pc();
        let capacity = match crate::shard::helpers::retry_policy_after_action(state, ticket.step) {
            Ok(policy) => policy.max_attempts,
            Err(RuntimeError::UnsupportedOperation {
                operation: "retry_metadata_missing",
            }) => ticket.capacity,
            Err(error) => return Err(error),
        };
        let ticket = crate::shard::helpers::normalize_scheduled_ticket(
            state,
            ActionTicket { capacity, ..ticket },
        )?;
        crate::shard::helpers::record_scheduled_attempt(state, ticket);
        Ok((ticket, step))
    }

    fn prepare_awaiting_timer(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
        evidence: &mut EvidenceCollector,
    ) -> RuntimeResult<AwaitingTimerPlan> {
        let step = state.frame.pc();
        let (mut journal_events, trace_events) =
            self.prepare_evidence_events(run, state.run_attempt.get(), evidence)?;
        let register_timer = crate::shard::helpers::timer_registration_required(&state, step);
        let (timer, ask_boundary_seq) = self.prepare_awaiting_schedule(AwaitingSchedule {
            run,
            state: &state,
            step,
            kind,
            register_timer,
            journal_events: &mut journal_events,
        })?;
        if let Some(timer) = timer {
            self.prepare_pending_timer(run, timer)?;
        }
        Ok((journal_events, trace_events, state, timer, ask_boundary_seq))
    }

    fn prepare_awaiting_schedule(
        &self,
        schedule: AwaitingSchedule<'_>,
    ) -> RuntimeResult<(Option<PendingTimer>, Option<vb_storage::EventSeq>)> {
        if !schedule.register_timer && schedule.kind != PendingTimerKind::Ask {
            return Ok((None, None));
        }
        let boundary_seq = self.awaiting_ask_boundary_sequence(&schedule)?;
        let timer = self.awaiting_pending_timer(&schedule)?;
        Self::push_drive_journal_event(
            schedule.journal_events,
            awaiting_schedule_event(schedule.run, schedule.step, schedule.kind),
        )?;
        Ok((timer, boundary_seq))
    }

    fn awaiting_ask_boundary_sequence(
        &self,
        schedule: &AwaitingSchedule<'_>,
    ) -> RuntimeResult<Option<vb_storage::EventSeq>> {
        if schedule.kind != PendingTimerKind::Ask {
            return Ok(None);
        }
        self.ask_boundary_sequence(schedule.run, schedule.journal_events.len())
            .map(Some)
    }

    fn awaiting_pending_timer(
        &self,
        schedule: &AwaitingSchedule<'_>,
    ) -> RuntimeResult<Option<PendingTimer>> {
        if !schedule.register_timer {
            return Ok(None);
        }
        let generation =
            self.scheduled_timer_generation(schedule.run, schedule.journal_events.len())?;
        self.pending_timer_for_awaiting(schedule.state, schedule.step, schedule.kind, generation)
            .map(Some)
    }

    fn apply_awaiting_action(
        &mut self,
        run: RunId,
        state: RunState,
        ticket: ActionTicket,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        self.commit_awaiting_action(run, state, ticket, evidence)
    }

    fn apply_awaiting_timer(
        &mut self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        self.commit_awaiting_timer(run, state, kind, evidence)
    }

    fn apply_terminal_finished(
        &mut self,
        run: RunId,
        state: RunState,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        if evidence.is_empty() {
            return self.apply_empty_terminal(run, state, true);
        }
        let result =
            crate::shard::helpers::result_slot_for_finished_run(&state).unwrap_or(SlotIdx::ZERO);
        let terminal_event = RuntimeJournalEvent::RunFinished {
            run,
            result,
            attempt: state.run_attempt.get(),
        };
        self.apply_terminal_journaled(run, state, evidence, terminal_event, true)
    }

    fn apply_terminal_failed(
        &mut self,
        run: RunId,
        state: RunState,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        if evidence.is_empty() {
            return self.apply_empty_terminal(run, state, false);
        }
        let terminal_event = RuntimeJournalEvent::RunFailed {
            run,
            attempt: state.run_attempt.get(),
        };
        self.apply_terminal_journaled(run, state, evidence, terminal_event, false)
    }

    fn apply_empty_terminal(
        &mut self,
        run: RunId,
        state: RunState,
        finished: bool,
    ) -> Result<(), DriveApplyFailure> {
        let outcome = if finished {
            self.finish_run(run, state)
        } else {
            self.fail_run_state(run, state)
        }
        .map_err(DriveApplyFailure::AfterCommit)?;
        if let TransitionOutcome::JournalBudgetExceeded(state) = outcome {
            return self.fail_run_on_journal_budget(run, *state, Vec::new());
        }
        let event = if finished {
            RuntimeEvent::DriveFinished
        } else {
            RuntimeEvent::Fail
        };
        self.apply(run, event)
            .map_err(DriveApplyFailure::AfterCommit)
    }

    fn apply_terminal_journaled(
        &mut self,
        run: RunId,
        mut state: RunState,
        evidence: &mut EvidenceCollector,
        terminal_event: RuntimeJournalEvent,
        finished: bool,
    ) -> Result<(), DriveApplyFailure> {
        let (journal_events, trace_events) =
            self.prepare_terminal_journal(run, &mut state, evidence, terminal_event)?;
        if !self
            .charge_drive_journal_bytes(&mut state, &journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?
        {
            return self.fail_run_on_journal_budget(run, state, trace_events);
        }
        self.commit_terminal_events(run, state, journal_events, trace_events, finished)
    }

    fn commit_terminal_events(
        &mut self,
        run: RunId,
        state: RunState,
        journal_events: Vec<RuntimeJournalEvent>,
        trace_events: Vec<TraceEvent>,
        finished: bool,
    ) -> Result<(), DriveApplyFailure> {
        self.append_journal_event_batch(&journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.push_trace_events(trace_events);
        if finished {
            self.finish_run_after_journaled(run, state)
                .map_err(DriveApplyFailure::AfterCommit)?;
        } else {
            let retry_step = state.frame.pc();
            self.fail_run_state_with_retry_checkpoint(run, retry_step, state)
                .map_err(DriveApplyFailure::AfterCommit)?;
        }
        let event = if finished {
            RuntimeEvent::DriveFinished
        } else {
            RuntimeEvent::Fail
        };
        self.apply(run, event)
            .map_err(DriveApplyFailure::AfterCommit)
    }

    fn prepare_terminal_journal(
        &self,
        run: RunId,
        state: &mut RunState,
        evidence: &mut EvidenceCollector,
        terminal_event: RuntimeJournalEvent,
    ) -> Result<(Vec<RuntimeJournalEvent>, Vec<TraceEvent>), DriveApplyFailure> {
        let (mut journal_events, trace_events) = self
            .prepare_evidence_events(run, state.run_attempt.get(), evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        Self::push_drive_journal_event(&mut journal_events, terminal_event)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        Ok((journal_events, trace_events))
    }
}

include!("chunk_005_helpers.rs");
