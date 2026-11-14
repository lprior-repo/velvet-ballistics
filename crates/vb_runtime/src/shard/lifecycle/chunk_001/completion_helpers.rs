impl Shard {
    fn prepare_completion_preflight(
        &self,
        run: RunId,
        ticket: ActionTicket,
        output: ActionOutputReady,
    ) -> RuntimeResult<(ActionCompletionPreflight, vb_core::ids::WorkflowDigest)> {
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        let preflight = preflight_action_completion(state, ticket, output)?;
        let abi_digest = schedule_action_abi_digest(state, ticket.action)?;
        Ok((preflight, abi_digest))
    }

    fn append_completion_journal(
        &mut self,
        run: RunId,
        preflight: &ActionCompletionPreflight,
        action_abi_digest: vb_core::ids::WorkflowDigest,
    ) -> RuntimeResult<bool> {
        self.append_command_journal_events(
            run,
            [RuntimeJournalEvent::ActionCompletedEnvelope {
                ticket: preflight.ticket,
                output: preflight.output_slot,
                value: preflight.encoded_value.clone(),
                encoded_len: preflight.encoded_len,
                taint: preflight.taint,
                value_digest: preflight.value_digest,
                action_abi_digest,
            }],
        )
    }

    fn fail_completion_journal_budget(&mut self, run: RunId) -> RuntimeResult<()> {
        let _removed_ticket = self.pending_action_remove(run);
        self.fail_command_run_on_journal_budget(run)
    }

    fn finish_action_completion(
        &mut self,
        run: RunId,
        preflight: ActionCompletionPreflight,
    ) -> RuntimeResult<()> {
        let (slot, step, encoded_value) = self.apply_action_completion_frame(run, preflight)?;
        let _removed_ticket = self.pending_action_remove(run);
        self.trace_action_completion(run, slot, step, encoded_value);
        self.drive_run(run)
    }

    fn write_action_completion_frame(
        &mut self,
        run: RunId,
        preflight: ActionCompletionPreflight,
    ) -> RuntimeResult<(SlotIdx, StepIdx, Vec<u8>)> {
        let ActionCompletionPreflight {
            ticket,
            output_slot,
            value,
            taint,
            encoded_value,
            ..
        } = preflight;
        let state = self
            .run_state_get_mut(run)
            .ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .write_slot_with_taint(output_slot, value, taint)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        advance_completed_action_frame(state, ticket.step)?;
        Ok((output_slot, ticket.step, encoded_value))
    }

    fn append_terminal_action_failure(
        &mut self,
        run: RunId,
        preflight: &ActionFailurePreflight,
    ) -> RuntimeResult<bool> {
        let attempt = self
            .run_state_get(run)
            .map(|state| state.run_attempt.get())
            .ok_or(RuntimeError::RunNotFound)?;
        let events = [
            action_failed_event(run, preflight),
            RuntimeJournalEvent::RunFailed { run, attempt },
        ];
        if self.append_command_journal_events(run, events)? {
            self.record_command_journal_events(
                run,
                [RuntimeJournalEvent::RunFailed { run, attempt }],
            )?;
        }
        Ok(false)
    }

    fn append_retryable_action_failure(
        &mut self,
        run: RunId,
        preflight: &ActionFailurePreflight,
        retry_timer: Option<&RetryTimerPlan>,
    ) -> RuntimeResult<bool> {
        let event = action_failed_event(run, preflight);
        match retry_timer {
            Some(plan) => self.append_command_journal_events(run, [event, plan.event.clone()]),
            None => self.append_command_journal_events(run, [event]),
        }
    }
}

fn advance_completed_action_frame(state: &mut RunState, step: StepIdx) -> RuntimeResult<()> {
    state
        .frame
        .mark_succeeded(step)
        .map_err(|_| RuntimeError::InvalidActionCompletion)?;
    crate::shard::helpers::advance_after_action_completion(state, step)?;
    state
        .frame
        .increment_executed()
        .map_err(|_| RuntimeError::InvalidActionCompletion)
}

fn action_failed_event(run: RunId, preflight: &ActionFailurePreflight) -> RuntimeJournalEvent {
    RuntimeJournalEvent::ActionFailed {
        run,
        step: preflight.ticket.step,
        action: preflight.ticket.action,
        attempt: preflight.ticket.attempt,
    }
}
