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
        let (preflight, action_abi_digest) = {
            let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
            let pf = preflight_action_completion(state, ticket, output)?;
            let abi = schedule_action_abi_digest(state, ticket.action)?;
            (pf, abi)
        };
        if self.append_command_journal_events(
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
        )? {
            let _removed_ticket = self.pending_action_remove(run);
            return self.fail_command_run_on_journal_budget(run);
        }
        let state = self
            .run_state_get_mut(run)
            .ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .write_slot_with_taint(preflight.output_slot, preflight.value, preflight.taint)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        state
            .frame
            .mark_succeeded(preflight.ticket.step)
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        crate::shard::helpers::advance_after_action_completion(state, preflight.ticket.step)?;
        state
            .frame
            .increment_executed()
            .map_err(|_| RuntimeError::InvalidActionCompletion)?;
        let _removed_ticket = self.pending_action_remove(run);
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
        let ticket = self.require_legacy_pending_action_ownership(run, step)?;
        self.reject_unregistered_completion(ticket)?;
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        state
            .frame
            .step_state(ticket.step)
            .map_err(|_| RuntimeError::RunNotFound)?;
        // Evidence chain: emit StepSucceeded for legacy action completion.
        // Legacy path has no output slot information.
        // Journal append FIRST so a journal failure does not diverge frame and journal.
        if self.append_command_journal_events(
            run,
            [RuntimeJournalEvent::StepSucceeded {
                run,
                step: ticket.step,
                output: SlotIdx::ZERO,
                attempt: ticket.attempt,
            }],
        )? {
            let _removed_ticket = self.pending_action_remove(run);
            return self.fail_command_run_on_journal_budget(run);
        }
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
        // 1. Preflight (read-only): validate and decide the failure outcome.
        //    No mutation has occurred yet, so a journal append failure
        //    below cannot diverge memory-only state from durable evidence.
        let preflight = {
            let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
            preflight_action_failure(state, ticket, &failure)?
        };
        // 1b. Retry durability gate: a RetryNow proceeds only when every
        // event emitted for the run is durable, flushing the queued prefix
        // on demand. Runs before any journal append so a full journal still
        // surfaces QueueFull from the append with state preserved, and a
        // blocked retry mutates nothing at all.
        if preflight.outcome == ActionFailureOutcome::RetryNow {
            self.can_retry(run)?;
        }
        let retry_timer = self.prepare_retry_timer(run, &preflight)?;
        // 2. Journal append FIRST. FailRun needs both ActionFailed and
        //    RunFailed to append as one same-run batch before any pending
        //    action, trace, frame, counter, or terminal-state mutation. If
        //    the batch is rejected by a bounded queued journal, no partial
        //    ActionFailed prefix can consume the last queue slot.
        let action_failed_event = RuntimeJournalEvent::ActionFailed {
            run,
            step: preflight.ticket.step,
            action: preflight.ticket.action,
            attempt: preflight.ticket.attempt,
        };
        let budget_exceeded = if preflight.outcome == ActionFailureOutcome::FailRun {
            let attempt = self
                .run_state_get(run)
                .map(|state| state.run_attempt.get())
                .ok_or(RuntimeError::RunNotFound)?;
            if self.append_command_journal_events(
                run,
                [
                    action_failed_event,
                    RuntimeJournalEvent::RunFailed { run, attempt },
                ],
            )? {
                self.record_command_journal_events(
                    run,
                    [RuntimeJournalEvent::RunFailed { run, attempt }],
                )?;
            }
            false
        } else if let Some(plan) = retry_timer.as_ref() {
            self.append_command_journal_events(run, [action_failed_event, plan.event.clone()])?
        } else {
            self.append_command_journal_events(run, [action_failed_event])?
        };
        // 3. Apply preflight mutations to the run frame / action_attempts.
        //    Held in a narrow scope so `pending_action_remove` and
        //    `trace_ring.push` can borrow `self` after the mutable
        //    state borrow ends.
        {
            let state = self
                .run_state_get_mut(run)
                .ok_or(RuntimeError::RunNotFound)?;
            apply_action_failure_preflight(state, &preflight)?;
        }
        let _removed_ticket = self.pending_action_remove(run);
        self.trace_ring.push(TraceEvent::ActionFailed {
            run,
            step: preflight.ticket.step,
            code,
        });
        if let Some(plan) = retry_timer {
            let timer = plan.timer;
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
            self.apply(run, RuntimeEvent::AwaitTimer)?;
            return Ok(());
        }
        // 4. Drive the run state machine forward using the preflighted outcome.
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
}
