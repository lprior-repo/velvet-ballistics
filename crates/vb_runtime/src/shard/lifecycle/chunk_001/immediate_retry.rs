impl Shard {
    fn commit_immediate_retry(
        &mut self,
        run: RunId,
        preflight: &ActionFailurePreflight,
        code: vb_core::ActionFailureCode,
    ) -> RuntimeResult<bool> {
        if !self.explicit_immediate_retry(run, preflight)? {
            return Ok(false);
        }
        let plan = self.prepare_immediate_retry(run, preflight, code)?;
        self.prepare_run_slots(run)?;
        let original = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        if self.command_journal_bytes_exceeds(original, &plan.0)? {
            self.commit_immediate_retry_budget_failure(run, preflight, code)?;
            return Ok(true);
        }
        self.commit_awaiting_action_plan(run, plan)
            .map_err(|failure| match failure {
                DriveApplyFailure::BeforeCommit(error) | DriveApplyFailure::AfterCommit(error) => {
                    error
                }
            })?;
        Ok(true)
    }

    fn explicit_immediate_retry(
        &self,
        run: RunId,
        preflight: &ActionFailurePreflight,
    ) -> RuntimeResult<bool> {
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        Ok(preflight.outcome == ActionFailureOutcome::RetryNow
            && preflight.delay_ms == 0
            && state
                .workflow
                .node(preflight.ticket.step)
                .is_some_and(|node| node.retry.is_some()))
    }

    fn prepare_immediate_retry(
        &mut self,
        run: RunId,
        preflight: &ActionFailurePreflight,
        code: vb_core::ActionFailureCode,
    ) -> RuntimeResult<AwaitingActionPlan> {
        self.gate_compat_run_start(run)?;
        let (state, ticket, mut evidence) = self.drive_immediate_retry(run, preflight)?;
        let ticket = ActionTicket {
            fencing_token: self.fencing_tokens.prepare_mint(run)?,
            ..ticket
        };
        let mut plan = self.prepare_awaiting_action(run, state, ticket, &mut evidence)?;
        Self::prepend_retry_failure(run, preflight, code, &mut plan)?;
        Ok(plan)
    }

    fn drive_immediate_retry(
        &self,
        run: RunId,
        preflight: &ActionFailurePreflight,
    ) -> RuntimeResult<(RunState, ActionTicket, EvidenceCollector)> {
        let mut state = self
            .run_state_get(run)
            .ok_or(RuntimeError::RunNotFound)?
            .clone();
        apply_action_failure_preflight(&mut state, preflight)?;
        let mut evidence = EvidenceCollector::new();
        let signal = Self::drive_state_with_manual_gates(
            &mut state,
            self.step_budget_per_tick,
            self.open_gate_count(run),
            &mut evidence,
        )
        .map_err(immediate_retry_drive_error)?;
        let ticket = immediate_retry_ticket(signal)?;
        Ok((state, ticket, evidence))
    }

    fn prepend_retry_failure(
        run: RunId,
        preflight: &ActionFailurePreflight,
        code: vb_core::ActionFailureCode,
        plan: &mut AwaitingActionPlan,
    ) -> RuntimeResult<()> {
        Self::push_drive_journal_event(&mut plan.0, action_failed_event(run, preflight))?;
        plan.0.rotate_right(1);
        plan.1
            .try_reserve(1)
            .map_err(|_| RuntimeError::from(vb_storage::JournalError::QueueFull))?;
        plan.1.insert(
            0,
            TraceEvent::ActionFailed {
                run,
                step: preflight.ticket.step,
                code,
            },
        );
        Ok(())
    }
}

fn immediate_retry_drive_error(error: crate::engine::types::RuntimeEngineError) -> RuntimeError {
    match error {
        crate::engine::types::RuntimeEngineError::Core(source) => source.into(),
        crate::engine::types::RuntimeEngineError::Action(source) => {
            runtime_error_from_action_error(source)
        }
        _ => RuntimeError::UnsupportedOperation {
            operation: "immediate_retry_drive_error",
        },
    }
}

fn immediate_retry_ticket(signal: RuntimeSignal) -> RuntimeResult<ActionTicket> {
    match signal {
        RuntimeSignal::AwaitingAction(ticket) => Ok(ticket),
        RuntimeSignal::StepBudgetExhausted => {
            Err(vb_core::errors::CoreError::StepBudgetExhausted.into())
        }
        _ => Err(RuntimeError::InvalidActionCompletion),
    }
}

include!("immediate_retry_budget.rs");
