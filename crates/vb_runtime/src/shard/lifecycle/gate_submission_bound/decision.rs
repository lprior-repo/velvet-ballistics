impl Shard {
    /// Submits a decision to an open bound gate and applies the outcome.
    pub(crate) fn submit_bound_gate_decision(
        &mut self,
        run: RunId,
        input: GateDecisionInput,
        operator_trust: OperatorTrust,
    ) -> RuntimeResult<BoundGateOutcomeRecord> {
        let open = self.open_bound_gate_for_decision(run, input.gate)?;
        self.check_bound_gate_submitter(run, &input, &open.gate, open.delegated_to.as_ref())?;
        let record = Self::evaluate_bound_gate_decision(&open, operator_trust, &input)?;
        Self::verify_bound_gate_authority(&record, &open.gate, open.opening_seq)?;
        self.append_gate_decision_bound_strict(record.clone())?;
        self.apply_bound_gate_decision_effects(run, input.gate, input.decision, &record)?;
        Ok(record)
    }

    fn open_bound_gate_for_decision(
        &mut self,
        run: RunId,
        gate: GateId,
    ) -> RuntimeResult<crate::shard::types::BoundOpenGate> {
        if self.open_gates.contains_key(&gate) {
            return Err(RuntimeError::GateEvaluationFailed {
                source: crate::manual_gate::GateError::LegacyGateWithoutBinding {
                    gate: gate.get(),
                },
            });
        }
        let open = self
            .bound_open_gates
            .get(&gate)
            .ok_or(RuntimeError::GateNotOpen { gate })?
            .clone();
        Self::check_bound_gate_run(&open.gate, run)?;
        if !self.run_state_contains(open.gate.gate.run) {
            self.bound_open_gates.swap_remove(&gate);
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
                self.bound_open_gates.swap_remove(&gate);
                self.handle_resume(run).map_err(RuntimeError::from)?;
            }
            GateDecision::Deny => {
                self.bound_open_gates.swap_remove(&gate);
                self.fail_gated_run(run)?;
            }
            GateDecision::RequestMoreEvidence => {}
            GateDecision::Delegate => {
                let delegated_to = record.record.delegated_to.clone();
                if let Some(open) = self.bound_open_gates.get_mut(&gate) {
                    open.delegated_to = delegated_to;
                }
            }
        }
        Ok(())
    }

    /// Escalates an open bound gate whose deadline has passed.
    pub(crate) fn escalate_timed_out_bound_gate(
        &mut self,
        run: RunId,
        gate: GateId,
        now_ms: u64,
    ) -> RuntimeResult<BoundGateOutcomeRecord> {
        let open = self.open_bound_gate_for_decision(run, gate)?;
        let run = open.gate.gate.run;
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        Self::check_bound_binding(state, &open.gate)?;
        let record = Self::evaluate_bound_gate_timeout(&open, now_ms)?;
        self.append_gate_decision_bound_strict(record.clone())?;
        self.bound_open_gates.swap_remove(&gate);
        self.fail_gated_run(run)?;
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
        if self.open_gates.contains_key(&input.gate) {
            return Err(RuntimeError::GateEvaluationFailed {
                source: crate::manual_gate::GateError::LegacyGateWithoutBinding {
                    gate: input.gate.get(),
                },
            });
        }
        match self.bound_open_gates.get(&input.gate) {
            Some(open) => {
                Self::check_bound_gate_run(&open.gate, run)?;
                let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
                Self::check_bound_binding(state, &open.gate)
            }
            None => Err(RuntimeError::GateNotOpen { gate: input.gate }),
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
}
