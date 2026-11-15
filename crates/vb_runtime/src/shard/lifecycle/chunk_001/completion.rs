impl Shard {
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn handle_action_completion(
        &mut self,
        ticket: ActionTicket,
        output: ActionOutputReady,
    ) -> RuntimeResult<()> {
        self.reject_unregistered_completion(ticket)?;
        self.apply_action_completion(ticket, output)
    }

    pub(crate) fn apply_action_completion(
        &mut self,
        ticket: ActionTicket,
        output: ActionOutputReady,
    ) -> RuntimeResult<()> {
        let run = ticket.run;
        self.require_pending_action_ownership(ticket)?;
        let (preflight, action_abi_digest) =
            self.prepare_completion_preflight(run, ticket, output)?;
        if self.append_completion_journal(run, &preflight, action_abi_digest)? {
            return self.fail_completion_journal_budget(run);
        }
        self.finish_action_completion(run, preflight)
    }

    fn apply_action_completion_frame(
        &mut self,
        run: RunId,
        preflight: ActionCompletionPreflight,
    ) -> RuntimeResult<(SlotIdx, StepIdx, Vec<u8>)> {
        self.write_action_completion_frame(run, preflight)
    }

    fn trace_action_completion(
        &mut self,
        run: RunId,
        slot: SlotIdx,
        step: StepIdx,
        encoded_value: Vec<u8>,
    ) {
        self.trace_ring.push(TraceEvent::SlotWritten {
            run,
            slot,
            value: encoded_value,
        });
        self.trace_ring
            .push(TraceEvent::ActionCompleted { run, step });
    }

    pub(crate) fn handle_legacy_action_completion(
        &mut self,
        run: RunId,
        step: StepIdx,
    ) -> RuntimeResult<()> {
        let ticket = self.require_legacy_pending_action_ownership(run, step)?;
        self.reject_unregistered_completion(ticket)?;
        self.validate_legacy_action_state(ticket)?;
        if self.append_legacy_action_event(run, ticket)? {
            let _removed_ticket = self.pending_action_remove(run);
            return self.fail_command_run_on_journal_budget(run);
        }
        self.finish_legacy_action_success(run, ticket)
    }

    fn validate_legacy_action_state(&self, ticket: ActionTicket) -> RuntimeResult<()> {
        self.run_state_get(ticket.run)
            .ok_or(RuntimeError::RunNotFound)?
            .frame
            .step_state(ticket.step)
            .map(|_| ())
            .map_err(|_| RuntimeError::RunNotFound)
    }

    fn append_legacy_action_event(
        &mut self,
        run: RunId,
        ticket: ActionTicket,
    ) -> RuntimeResult<bool> {
        self.append_command_journal_events(
            run,
            [RuntimeJournalEvent::StepSucceeded {
                run,
                step: ticket.step,
                output: SlotIdx::ZERO,
                attempt: ticket.attempt,
            }],
        )
    }

    fn finish_legacy_action_success(
        &mut self,
        run: RunId,
        ticket: ActionTicket,
    ) -> RuntimeResult<()> {
        let state = self
            .run_state_get_mut(run)
            .ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_succeeded(ticket.step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        crate::shard::helpers::advance_after_action_completion(state, ticket.step)?;
        state
            .frame
            .increment_executed()
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        let _removed_ticket = self.pending_action_remove(run);
        self.trace_ring.push(TraceEvent::ActionCompleted {
            run,
            step: ticket.step,
        });
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
        self.require_pending_action_ownership(ticket)?;
        let ticket = self.ticket_with_retry_capacity(ticket, failure.retry_policy)?;
        let preflight = {
            let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
            preflight_action_failure(state, ticket, &failure)?
        };
        if preflight.outcome == ActionFailureOutcome::RetryNow {
            self.can_retry(run)?;
        }
        if self.commit_immediate_retry(run, &preflight, code)? {
            return Ok(());
        }
        let retry_timer = self.prepare_retry_timer(run, &preflight)?;
        let budget_exceeded =
            self.append_action_failure_events(run, &preflight, retry_timer.as_ref())?;
        self.apply_action_failure_frame(run, &preflight)?;
        self.record_action_failure(run, &preflight, code);
        self.finish_action_failure(run, preflight, retry_timer, budget_exceeded)
    }

    fn append_action_failure_events(
        &mut self,
        run: RunId,
        preflight: &ActionFailurePreflight,
        retry_timer: Option<&RetryTimerPlan>,
    ) -> RuntimeResult<bool> {
        if preflight.outcome == ActionFailureOutcome::FailRun {
            return self.append_terminal_action_failure(run, preflight);
        }
        self.append_retryable_action_failure(run, preflight, retry_timer)
    }

    fn apply_action_failure_frame(
        &mut self,
        run: RunId,
        preflight: &ActionFailurePreflight,
    ) -> RuntimeResult<()> {
        let state = self
            .run_state_get_mut(run)
            .ok_or(RuntimeError::RunNotFound)?;
        apply_action_failure_preflight(state, preflight)
    }

    fn record_action_failure(
        &mut self,
        run: RunId,
        preflight: &ActionFailurePreflight,
        code: vb_core::ActionFailureCode,
    ) {
        let _removed_ticket = self.pending_action_remove(run);
        self.trace_ring.push(TraceEvent::ActionFailed {
            run,
            step: preflight.ticket.step,
            code,
        });
    }

    fn finish_action_failure(
        &mut self,
        run: RunId,
        preflight: ActionFailurePreflight,
        retry_timer: Option<RetryTimerPlan>,
        budget_exceeded: bool,
    ) -> RuntimeResult<()> {
        if let Some(plan) = retry_timer {
            return self.finish_delayed_action_retry(run, plan.timer, budget_exceeded);
        }
        match preflight.outcome {
            ActionFailureOutcome::RetryNow | ActionFailureOutcome::DriveHandler => {
                if budget_exceeded {
                    return self.fail_command_run_on_journal_budget(run);
                }
                self.drive_run(run)
            }
            ActionFailureOutcome::FailRun => {
                let state = self.take_run_state(run)?;
                self.fail_run_state_with_retry_checkpoint(run, preflight.ticket.step, state)
            }
        }
    }

    fn finish_delayed_action_retry(
        &mut self,
        run: RunId,
        timer: PendingTimer,
        budget_exceeded: bool,
    ) -> RuntimeResult<()> {
        if budget_exceeded {
            return self.fail_command_run_on_journal_budget(run);
        }
        let state = self
            .run_state_get_mut(run)
            .ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .mark_waiting(timer.step)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        self.pending_timer_insert(run, timer)?;
        self.apply(run, RuntimeEvent::AwaitTimer)
    }
}

include!("completion_helpers.rs");
include!("immediate_retry.rs");
