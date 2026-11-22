impl Shard {
    /// Submits a decision to an open bound gate and applies the outcome.
    pub(crate) fn submit_bound_gate_decision(
        &mut self,
        run: RunId,
        input: GateDecisionInput,
        operator_trust: OperatorTrust,
    ) -> RuntimeResult<BoundGateOutcomeRecord> {
        let clock_ms = crate::shard::helpers::submission_time()?;
        Self::check_timestamp_not_future(input.decided_at_ms, clock_ms)?;
        let open = self.open_bound_gate_for_decision(run, input.gate)?;
        Self::check_processing_deadline(&open, clock_ms)?;
        self.check_bound_gate_submitter(run, &input, &open.gate, open.delegated_to.as_ref())?;
        let record = Self::evaluate_bound_gate_decision(&open, operator_trust, &input)?;
        Self::verify_bound_gate_authority(&record, &open.gate, open.opening_seq)?;
        let decision = input.decision;
        if matches!(decision, GateDecision::Deny) {
            self.fail_bound_gate_run(run, input.gate, record.clone())?;
            return Ok(record);
        }
        self.append_gate_decision_bound_strict(record.clone())?;
        self.apply_bound_gate_decision_effects(run, input.gate, decision, &record)?;
        Ok(record)
    }

    fn open_bound_gate_for_decision(
        &mut self,
        run: RunId,
        gate: GateId,
    ) -> RuntimeResult<crate::shard::types::BoundOpenGate> {
        if self.open_gates.contains_key(&gate) {
            return Err(Self::legacy_gate_without_binding(gate));
        }
        let open = self
            .bound_open_gates
            .get(&gate)
            .ok_or(RuntimeError::GateNotOpen { gate })?
            .clone();
        Self::check_bound_gate_run(&open.gate, run)?;
        if !self.run_state_contains(open.gate.gate.run) {
            self.bound_open_gates.shift_remove(&gate);
            return Err(RuntimeError::RunNotFound);
        }
        Ok(open)
    }

    fn evaluate_bound_gate_decision(
        open: &crate::shard::types::BoundOpenGate,
        operator_trust: OperatorTrust,
        input: &GateDecisionInput,
    ) -> RuntimeResult<BoundGateOutcomeRecord> {
        evaluate_bound_decision(
            &open.gate,
            open.opened_at_ms,
            open.opening_seq,
            &open.requester,
            operator_trust,
            input,
        )
        .map_err(|source| RuntimeError::GateEvaluationFailed { source })
    }

    fn check_bound_gate_submitter(
        &self,
        run: RunId,
        input: &GateDecisionInput,
        gate: &BoundManualGate,
        delegated_to: Option<&ApproverId>,
    ) -> RuntimeResult<()> {
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        Self::check_bound_binding(state, gate)?;
        if let Some(delegate) = delegated_to
            && input.approver != *delegate
        {
            return Err(RuntimeError::GateWrongDelegate {
                gate: input.gate,
                expected: delegate.clone(),
                actual: input.approver.clone(),
            });
        }
        Ok(())
    }

    fn apply_bound_gate_decision_effects(
        &mut self,
        run: RunId,
        gate: GateId,
        decision: GateDecision,
        record: &BoundGateOutcomeRecord,
    ) -> RuntimeResult<()> {
        match decision {
            GateDecision::Approve | GateDecision::BreakGlass => {
                self.bound_open_gates.shift_remove(&gate);
                self.handle_resume(run).map_err(RuntimeError::from)?;
            }
            GateDecision::Deny => {
                return Err(RuntimeError::UnsupportedOperation {
                    operation: "bound_deny_requires_atomic_failure",
                });
            }
            GateDecision::RequestMoreEvidence => {}
            GateDecision::Delegate => {
                let delegated_to = record.record.delegated_to.clone();
                let open = self
                    .bound_open_gates
                    .get_mut(&gate)
                    .ok_or(RuntimeError::GateNotOpen { gate })?;
                open.delegated_to = delegated_to;
            }
        }
        Ok(())
    }

    /// Journals a bound gate denial or timeout and fails its run as one atomic
    /// batch. The gate closes only after the `RunFailed` is durable.
    fn fail_bound_gate_run(
        &mut self,
        run: RunId,
        gate: GateId,
        record: BoundGateOutcomeRecord,
    ) -> RuntimeResult<()> {
        self.checked_out_run_insert(run)?;
        let projected = match self.append_bound_gate_failure_strict(record) {
            Ok(Some(projected)) => projected,
            Ok(None) => {
                self.checked_out_run_remove(run);
                return self.fail_bound_gate_run_on_budget(run, gate);
            }
            Err(error) => {
                self.checked_out_run_remove(run);
                return Err(error);
            }
        };
        self.bound_open_gates.shift_remove(&gate);
        let Some(mut state) = self.run_state_remove(run) else {
            self.checked_out_run_remove(run);
            return Err(RuntimeError::RunNotFound);
        };
        state.journal_bytes_written = projected;
        let retry_step = state.frame.pc();
        self.fail_run_state_with_retry_checkpoint(run, retry_step, state)?;
        self.apply(run, RuntimeEvent::Fail)
    }

    /// Fails a bound gate's run with only `RunFailed` when the decision batch
    /// would exceed the run's journal budget. The run is restored on a
    /// pre-commit failure. Once the run failure is durable the gate closes and
    /// this returns `GateJournalBudgetExceeded`, so the rejected decision is
    /// never reported as journaled.
    fn fail_bound_gate_run_on_budget(&mut self, run: RunId, gate: GateId) -> RuntimeResult<()> {
        let state = self.take_run_state(run)?;
        let rollback = state.clone();
        match self.fail_run_on_journal_budget(run, state, Vec::new()) {
            Ok(()) => {}
            Err(DriveApplyFailure::BeforeCommit(error)) => {
                if let Err(rollback_error) = self.run_state_insert(run, rollback) {
                    return Err(RuntimeError::rollback_failed(
                        "bound_gate_journal_budget",
                        error,
                        rollback_error,
                    ));
                }
                return Err(error);
            }
            Err(DriveApplyFailure::AfterCommit(error)) => {
                self.bound_open_gates.shift_remove(&gate);
                return Err(error);
            }
        }
        self.bound_open_gates.shift_remove(&gate);
        Err(RuntimeError::GateJournalBudgetExceeded { gate })
    }

    /// Escalates an open bound gate whose deadline has passed.
    pub(crate) fn escalate_timed_out_bound_gate(
        &mut self,
        run: RunId,
        gate: GateId,
        now_ms: u64,
    ) -> RuntimeResult<BoundGateOutcomeRecord> {
        Self::check_not_future(now_ms)?;
        let open = self.open_bound_gate_for_decision(run, gate)?;
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        Self::check_bound_binding(state, &open.gate)?;
        let record = Self::evaluate_bound_gate_timeout(&open, now_ms)?;
        self.fail_bound_gate_run(run, gate, record.clone())?;
        Ok(record)
    }

    fn evaluate_bound_gate_timeout(
        open: &crate::shard::types::BoundOpenGate,
        now_ms: u64,
    ) -> RuntimeResult<BoundGateOutcomeRecord> {
        evaluate_bound_timeout(
            &open.gate,
            &open.requester,
            open.opened_at_ms,
            open.opening_seq,
            now_ms,
        )
        .map_err(|source| RuntimeError::GateEvaluationFailed { source })
    }

    /// Fail-fast pre-check for direct-API bound submission.
    pub(crate) fn validate_open_bound_gate_decision(
        &self,
        run: RunId,
        input: &GateDecisionInput,
    ) -> RuntimeResult<()> {
        Self::check_not_future(input.decided_at_ms)?;
        if self.open_gates.contains_key(&input.gate) {
            return Err(Self::legacy_gate_without_binding(input.gate));
        }
        match self.bound_open_gates.get(&input.gate) {
            Some(open) => {
                Self::check_processing_deadline(open, crate::shard::helpers::submission_time()?)?;
                Self::check_bound_gate_run(&open.gate, run)?;
                let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
                Self::check_bound_binding(state, &open.gate)
            }
            None => Err(RuntimeError::GateNotOpen { gate: input.gate }),
        }
    }

    fn legacy_gate_without_binding(gate: GateId) -> RuntimeError {
        RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::LegacyGateWithoutBinding { gate: gate.get() },
        }
    }

    fn check_bound_gate_run(gate: &BoundManualGate, run: RunId) -> RuntimeResult<()> {
        if gate.gate.run != run {
            return Err(RuntimeError::GateRunMismatch {
                gate: gate.gate.id,
                expected: gate.gate.run,
                actual: run,
            });
        }
        Ok(())
    }

    /// Rejects a caller-supplied timestamp later than the runtime clock.
    pub(crate) fn check_not_future(timestamp_ms: u64) -> RuntimeResult<()> {
        let clock_ms = crate::shard::helpers::submission_time()?;
        Self::check_timestamp_not_future(timestamp_ms, clock_ms)
    }

    /// Rejects `timestamp_ms` when it is later than the already-read `clock_ms`.
    pub(crate) fn check_timestamp_not_future(
        timestamp_ms: u64,
        clock_ms: u64,
    ) -> RuntimeResult<()> {
        if timestamp_ms > clock_ms {
            return Err(RuntimeError::GateEvaluationFailed {
                source: crate::manual_gate::GateError::TimestampFromFuture {
                    timestamp_ms,
                    clock_ms,
                },
            });
        }
        Ok(())
    }

    /// Rejects a live decision processed after the gate deadline, whatever its stamped time.
    fn check_processing_deadline(
        open: &crate::shard::types::BoundOpenGate,
        processed_at_ms: u64,
    ) -> RuntimeResult<()> {
        crate::manual_gate::check_live_processing_window(
            &open.gate.gate,
            open.opened_at_ms,
            processed_at_ms,
        )
        .map_err(|source| RuntimeError::GateEvaluationFailed { source })
    }
}
