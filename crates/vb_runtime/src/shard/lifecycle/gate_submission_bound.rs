// Bound manual gate submission flow (MASTER §77.15 additive path).
//
// Mirrors `gate_submission.rs` for the versioned bound representation. The
// legacy registry and the bound registry are disjoint: neither path can
// approve a gate opened through the other, so a legacy gate cannot silently
// gain binding authority and a bound gate cannot lose it.

impl Shard {
    /// Opens a bound manual gate and suspends the run until decided.
    pub(crate) fn open_bound_gate(
        &mut self,
        gate: BoundManualGate,
        opened_at_ms: u64,
        requester: ApproverId,
    ) -> RuntimeResult<()> {
        let run = gate.gate.run;
        if !gate.is_structurally_valid() || !requester.is_valid() {
            return Err(RuntimeError::InvalidGateOpenedRecord);
        }
        if !self.run_state_contains(run) {
            return Err(RuntimeError::RunNotFound);
        }
        if self.open_gates.contains_key(&gate.gate.id)
            || self.bound_open_gates.contains_key(&gate.gate.id)
        {
            return Err(RuntimeError::GateAlreadyOpen { gate: gate.gate.id });
        }
        self.check_gate_suspendable(run)?;
        self.reserve_open_gate_slot()?;
        self.append_gate_opened_bound_strict(gate.clone(), opened_at_ms, requester.clone())?;
        self.apply(run, RuntimeEvent::AwaitAction)?;
        let open = BoundOpenGate {
            gate,
            opened_at_ms,
            requester,
            delegated_to: None,
        };
        self.bound_open_gates.insert(open.gate.gate.id, open);
        Ok(())
    }

    /// Submits a decision to an open bound gate and applies the outcome.
    pub(crate) fn submit_bound_gate_decision(
        &mut self,
        run: RunId,
        input: GateDecisionInput,
        operator_trust: OperatorTrust,
    ) -> RuntimeResult<BoundGateOutcomeRecord> {
        if self.open_gates.contains_key(&input.gate) {
            return Err(RuntimeError::GateEvaluationFailed {
                source: crate::manual_gate::GateError::LegacyGateWithoutBinding {
                    gate: input.gate.get(),
                },
            });
        }
        let open = self
            .bound_open_gates
            .get(&input.gate)
            .ok_or(RuntimeError::GateNotOpen { gate: input.gate })?
            .clone();
        Self::check_bound_gate_run(&open.gate, run)?;
        let run = open.gate.gate.run;
        if !self.run_state_contains(run) {
            self.bound_open_gates.swap_remove(&input.gate);
            return Err(RuntimeError::RunNotFound);
        }
        if let Some(delegate) = &open.delegated_to
            && input.approver != *delegate
        {
            return Err(RuntimeError::GateWrongDelegate {
                gate: input.gate,
                expected: delegate.clone(),
                actual: input.approver.clone(),
            });
        }
        let record = evaluate_bound_decision(
            &open.gate,
            open.opened_at_ms,
            &open.requester,
            operator_trust,
            &input,
        )
        .map_err(|source| RuntimeError::GateEvaluationFailed { source })?;
        self.append_gate_decision_bound_strict(record.clone())?;
        match input.decision {
            GateDecision::Approve | GateDecision::BreakGlass => {
                self.bound_open_gates.swap_remove(&input.gate);
                self.handle_resume(run).map_err(RuntimeError::from)?;
            }
            GateDecision::Deny => {
                self.bound_open_gates.swap_remove(&input.gate);
                self.fail_gated_run(run)?;
            }
            GateDecision::RequestMoreEvidence => {}
            GateDecision::Delegate => {
                let delegated_to = record.record.delegated_to.clone();
                if let Some(open) = self.bound_open_gates.get_mut(&input.gate) {
                    open.delegated_to = delegated_to;
                }
            }
        }
        Ok(record)
    }

    /// Escalates an open bound gate whose deadline has passed.
    pub(crate) fn escalate_timed_out_bound_gate(
        &mut self,
        run: RunId,
        gate: GateId,
        now_ms: u64,
    ) -> RuntimeResult<BoundGateOutcomeRecord> {
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
        let run = open.gate.gate.run;
        if !self.run_state_contains(run) {
            self.bound_open_gates.swap_remove(&gate);
            return Err(RuntimeError::RunNotFound);
        }
        let record = evaluate_bound_timeout(&open.gate, &open.requester, open.opened_at_ms, now_ms)
            .map_err(|source| RuntimeError::GateEvaluationFailed { source })?;
        self.append_gate_decision_bound_strict(record.clone())?;
        self.bound_open_gates.swap_remove(&gate);
        self.fail_gated_run(run)?;
        Ok(record)
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
            Some(open) => Self::check_bound_gate_run(&open.gate, run),
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
