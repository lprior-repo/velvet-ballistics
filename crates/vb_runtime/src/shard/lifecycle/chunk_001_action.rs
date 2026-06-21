use vb_core::frame::StepState;

impl Shard {
    // =============================================================================
    // Action completion and failure lifecycle methods
    // =============================================================================

    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn handle_action_completion(
        &mut self,
        ticket: ActionTicket,
        output: ActionOutputReady,
    ) -> RuntimeResult<()> {
        let run = ticket.run;
        let preflight = {
            let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
            preflight_action_completion(state, ticket, output)?
        };
        self.append_journal_event(RuntimeJournalEvent::ActionCompletedEnvelope {
            ticket: preflight.ticket,
            output: preflight.output_slot,
            value: preflight.encoded_value.clone(),
            encoded_len: preflight.encoded_len,
            taint: preflight.taint,
            value_digest: preflight.value_digest,
        })?;
        let state = self.run_state_get_mut(run).ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .write_slot_with_taint(preflight.output_slot, preflight.value, preflight.taint)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        state
            .frame
            .mark_succeeded(preflight.ticket.step)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        crate::shard::helpers::advance_after_action_completion(state, preflight.ticket.step)?;
        self.trace_ring.push(TraceEvent::SlotWritten {
            run,
            slot: preflight.output_slot,
            value: preflight.encoded_value,
        });
        self.trace_ring.push(TraceEvent::ActionCompleted {
            run,
            step: preflight.ticket.step,
        });
        self.drive_run(run)
    }

    pub(crate) fn handle_legacy_action_completion(
        &mut self,
        run: RunId,
        step: StepIdx,
    ) -> RuntimeResult<()> {
        // RS-010: probe the run state but do NOT mutate the frame yet.
        // Journal the StepSucceeded event first so a journal append
        // failure leaves the in-memory frame consistent with the durable
        // record. Mirrors the ordering used by `handle_action_completion`.
        {
            let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
            if state.frame.step_state(step) != Ok(StepState::Running) {
                return Err(RuntimeError::RunNotFound);
            }
        }
        // Evidence chain: emit StepSucceeded for legacy action completion.
        // Legacy path has no output slot information. Append before any
        // frame mutation so a journal failure does not desynchronise
        // memory and durability (the journal is the source of truth).
        self.append_journal_event(RuntimeJournalEvent::StepSucceeded {
            run,
            step,
            output: SlotIdx::ZERO,
            attempt: 1,
        })?;
        let state = self.run_state_get_mut(run).ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_succeeded(step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        self.trace_ring
            .push(TraceEvent::ActionCompleted { run, step });
        self.drive_run(run)
    }

    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn handle_action_failure(
        &mut self,
        ticket: ActionTicket,
        failure: ActionFailure,
    ) -> RuntimeResult<()> {
        let run = ticket.run;
        let code = failure.code;
        let ticket = self.ticket_with_retry_capacity(ticket, failure.retry_policy)?;
        let outcome = self.apply_action_failure_to_state(ticket, failure)?;
        self.trace_ring.push(TraceEvent::ActionFailed {
            run,
            step: ticket.step,
            code,
        });
        self.append_journal_event(RuntimeJournalEvent::ActionFailed {
            run,
            step: ticket.step,
            action: ticket.action,
            attempt: ticket.attempt,
        })?;
        match outcome {
            ActionFailureOutcome::RetryNow | ActionFailureOutcome::DriveHandler => {
                self.drive_run(run)
            }
            ActionFailureOutcome::FailRun => {
                let state = self.take_run_state(run)?;
                // apply() handles runtime_states mutation; fail_run_state handles cleanup only
                let _ = self.apply(run, RuntimeEvent::Fail);
                self.fail_run_state(run, state)
            }
        }
    }

    pub fn ticket_with_retry_capacity(
        &self,
        ticket: ActionTicket,
        retry_policy: vb_core::action::RetryPolicy,
    ) -> RuntimeResult<ActionTicket> {
        let Some(state) = self.run_state_get(ticket.run) else {
            return Err(RuntimeError::RunNotFound);
        };
        if retry_policy != vb_core::action::RetryPolicy::Retryable
            || !crate::shard::helpers::retry_metadata_exists(state, ticket.step)
        {
            return Ok(ticket);
        }
        let policy = crate::shard::helpers::retry_policy_after_action(state, ticket.step)?;
        Ok(ActionTicket {
            capacity: ticket.capacity.max(policy.max_attempts),
            ..ticket
        })
    }

    fn apply_action_failure_to_state(
        &mut self,
        ticket: ActionTicket,
        failure: ActionFailure,
    ) -> RuntimeResult<ActionFailureOutcome> {
        let state = self
            .run_state_get_mut(ticket.run)
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        crate::shard::helpers::validate_action_completion(state, ticket)?;
        if retry_is_available(state, ticket, failure.retry_policy)? {
            state
                .frame
                .set_pc(ticket.step)
                .map_err(|_| RuntimeError::InvalidActionCompletion)?;
            return Ok(ActionFailureOutcome::RetryNow);
        }
        apply_error_handler(state, ticket)
    }
}