impl Shard {
    fn prepare_awaiting_action(
        &self,
        run: RunId,
        mut state: RunState,
        ticket: ActionTicket,
        evidence: &mut EvidenceCollector,
    ) -> RuntimeResult<AwaitingActionPlan> {
        let step = state.frame.pc();
        let capacity = match crate::shard::helpers::retry_policy_after_action(&state, ticket.step) {
            Ok(policy) => policy.max_attempts,
            Err(RuntimeError::UnsupportedOperation {
                operation: "retry_metadata_missing",
            }) => ticket.capacity,
            Err(error) => return Err(error),
        };
        let ticket = crate::shard::helpers::normalize_scheduled_ticket(
            &state,
            ActionTicket { capacity, ..ticket },
        )?;
        crate::shard::helpers::record_scheduled_attempt(&mut state, ticket);
        let output = crate::shard::helpers::action_output_slot(&state, ticket.step)?;
        let input = crate::shard::helpers::action_input_slot(&state, ticket.step)?;
        let (mut journal_events, trace_events) =
            self.prepare_evidence_events(run, state.run_attempt.get(), evidence)?;
        // Policy decision is evaluated in apply_awaiting_action before
        // pending ticket visibility. The ActionScheduledTicket event is
        // created only after the decision is made (Permit path).
        let action_abi_digest = schedule_action_abi_digest(&state, ticket.action)?;
        Ok((
            journal_events,
            trace_events,
            state,
            ticket,
            step,
            input,
            output,
            action_abi_digest,
        ))
    }

    fn prepare_awaiting_timer(
        &self,
        run: RunId,
        state: RunState,
        kind: PendingTimerKind,
        evidence: &mut EvidenceCollector,
    ) -> RuntimeResult<AwaitingTimerPlan> {
        let step = state.frame.pc();
        let (mut journal_events, trace_events) =
            self.prepare_evidence_events(run, state.run_attempt.get(), evidence)?;
        let register_timer = crate::shard::helpers::timer_registration_required(&state, step);
        let (timer, ask_boundary_seq) =
            self.prepare_awaiting_schedule(run, step, kind, register_timer, &mut journal_events)?;
        Ok((journal_events, trace_events, state, timer, ask_boundary_seq))
    }

    fn prepare_awaiting_schedule(
        &self,
        run: RunId,
        step: StepIdx,
        kind: PendingTimerKind,
        register_timer: bool,
        journal_events: &mut Vec<RuntimeJournalEvent>,
    ) -> RuntimeResult<(Option<PendingTimer>, Option<vb_storage::EventSeq>)> {
        if !register_timer && kind != PendingTimerKind::Ask {
            return Ok((None, None));
        }
        let boundary_seq = if kind == PendingTimerKind::Ask {
            Some(self.ask_boundary_sequence(run, journal_events.len())?)
        } else {
            None
        };
        let timer = self.pending_timer_for_awaiting(run, step, kind, register_timer)?;
        Self::push_drive_journal_event(journal_events, awaiting_schedule_event(run, step, kind))?;
        Ok((timer, boundary_seq))
    }

    fn ask_boundary_sequence(
        &self,
        run: RunId,
        preceding_events: usize,
    ) -> RuntimeResult<vb_storage::EventSeq> {
        let offset = u64::try_from(preceding_events)
            .map_err(|_| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?;
        self.journal_sequence_for(run)
            .get()
            .checked_add(offset)
            .map(vb_storage::EventSeq::new)
            .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))
    }

    fn pending_timer_for_awaiting(
        &self,
        run: RunId,
        step: StepIdx,
        kind: PendingTimerKind,
        register_timer: bool,
    ) -> RuntimeResult<Option<PendingTimer>> {
        if !register_timer {
            return Ok(None);
        }
        let generation = self
            .next_pending_timer_generation(run)
            .ok_or(RuntimeError::InvalidTimerFire)?;
        Ok(Some(PendingTimer {
            step,
            kind,
            generation,
            deadline: std::time::Instant::now(),
        }))
    }

    fn apply_awaiting_action(
        &mut self,
        run: RunId,
        state: RunState,
        ticket: ActionTicket,
        evidence: &mut EvidenceCollector,
    ) -> Result<(), DriveApplyFailure> {
        if evidence.is_empty() {
            // Historical replay: reconstruct recorded decision from journal,
            // do not re-evaluate current policy. The decision identity is
            // preserved by the journal event, not by re-deriving it.
            self.apply(run, RuntimeEvent::AwaitAction)
                .map_err(DriveApplyFailure::AfterCommit)?;
            return self
                .await_action(run, state, ticket)
                .map_err(DriveApplyFailure::AfterCommit);
        }

        // Live dispatch: evaluate policy before any pending ticket visibility.
        let (mut journal_events, trace_events, state, ticket, step, input, output, action_abi_digest) =
            self
                .prepare_awaiting_action(run, state, ticket, evidence)
                .map_err(DriveApplyFailure::BeforeCommit)?;

        // Get input value and taint for the policy digest.
        // Failures to read input are not fatal — use defaults.
        let input_value_bytes = match state.frame.read_slot(input) {
            Ok(value) => postcard::to_allocvec(value).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        let input_taint = state
            .frame
            .read_taint(input)
            .unwrap_or(vb_core::value::Taint::Clean);

        // Evaluate policy at the outbox boundary.
        let decision = self
            .policy_recorder
            .evaluate_action(&ticket, &input_value_bytes, input_taint)
            .map_err(DriveApplyFailure::BeforeCommit)?;

        // Journal the policy decision before any action schedule event.
        // This ensures crash consistency: the decision is durable before
        // the effect is published.
        let policy_decision_event = RuntimeJournalEvent::PolicyDecision {
            run,
            layer: decision.layer,
            code: decision.code,
            policy_digest: decision.policy_digest,
            input_digest: decision.input_digest,
            emergency_deny_active: decision.emergency_deny_active,
            compatibility_influence: decision.compatibility_influence,
        };
        Self::push_drive_journal_event(&mut journal_events, policy_decision_event)
            .map_err(DriveApplyFailure::BeforeCommit)?;

        if decision.code == vb_core::policy::DecisionCode::Allowed {
            // Permit path: append the action schedule event and publish
            // the pending ticket.
            Self::push_drive_journal_event(
                &mut journal_events,
                RuntimeJournalEvent::ActionScheduledTicket {
                    ticket,
                    input,
                    output,
                    action_abi_digest,
                },
            )
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
        } else {
            // Denied or suspended: do not publish a pending ticket.
            // Transition to a typed failed/suspended state.
            self.append_journal_event_batch(&journal_events)
                .map_err(DriveApplyFailure::BeforeCommit)?;
            self.push_trace_events(trace_events);
            self.run_state_insert(run, state)
                .map_err(DriveApplyFailure::AfterCommit)?;
            self.apply(run, RuntimeEvent::Fail)
                .map_err(DriveApplyFailure::AfterCommit)
        }
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
        if kind == PendingTimerKind::Ask {
            self.reserve_pending_ask_authority_slot(run)
                .map_err(DriveApplyFailure::BeforeCommit)?;
        }
        let (journal_events, trace_events, state, timer, ask_boundary_seq) = self
            .prepare_awaiting_timer(run, state, kind, evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.append_journal_event_batch(&journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        if let Some(boundary_seq) = ask_boundary_seq {
            self.record_pending_ask_authority(run, state.frame.pc(), boundary_seq);
        }
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
            .prepare_evidence_events(run, state.run_attempt.get(), evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        Self::push_drive_journal_event(
            &mut journal_events,
            RuntimeJournalEvent::RunFinished {
                run,
                result,
                attempt: state.run_attempt.get(),
            },
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
            .prepare_evidence_events(run, state.run_attempt.get(), evidence)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        Self::push_drive_journal_event(
            &mut journal_events,
            RuntimeJournalEvent::RunFailed {
                run,
                attempt: state.run_attempt.get(),
            },
        )
        .map_err(DriveApplyFailure::BeforeCommit)?;
        self.append_journal_event_batch(&journal_events)
            .map_err(DriveApplyFailure::BeforeCommit)?;
        self.push_trace_events(trace_events);
        let retry_step = state.frame.pc();
        self.fail_run_state_with_retry_checkpoint(run, retry_step, state)
            .map_err(DriveApplyFailure::AfterCommit)?;
        self.apply(run, RuntimeEvent::Fail)
            .map_err(DriveApplyFailure::AfterCommit)
    }
}