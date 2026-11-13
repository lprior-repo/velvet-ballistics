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
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        Self::check_bound_binding(state, &gate)?;
        if self.open_gates.contains_key(&gate.gate.id)
            || self.bound_open_gates.contains_key(&gate.gate.id)
        {
            return Err(RuntimeError::GateAlreadyOpen { gate: gate.gate.id });
        }
        self.check_bound_gate_suspendable(run, gate.gate.action)?;
        self.reserve_open_gate_slot()?;
        let opening_seq = self
            .append_gate_opened_bound_strict(gate.clone(), opened_at_ms, requester.clone())?
            .get();
        self.apply(run, RuntimeEvent::AwaitAction)?;
        let open = BoundOpenGate {
            gate,
            opened_at_ms,
            opening_seq,
            requester,
            delegated_to: None,
        };
        self.bound_open_gates.insert(open.gate.gate.id, open);
        Ok(())
    }

    fn check_bound_gate_suspendable(
        &self,
        run: RunId,
        action: vb_core::ActionId,
    ) -> RuntimeResult<()> {
        let state = self.get_runtime_state_or_running(run);
        let exact_pending = state == crate::shard::types::RuntimeState::Resumable
            && self
                .pending_action_get(run)
                .is_some_and(|ticket| ticket.action == action);
        let runnable = matches!(
            state,
            crate::shard::types::RuntimeState::Initial | crate::shard::types::RuntimeState::Running
        ) && self.pending_action_get(run).is_none();
        if (!exact_pending && !runnable)
            || self.pending_timer_get(run).is_some()
            || self.pending_ask_authorities.contains_key(&run)
        {
            return Err(RuntimeError::GateRunNotSuspendable { run, state });
        }
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
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        Self::check_bound_binding(state, &open.gate)?;
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
            open.opening_seq,
            &open.requester,
            operator_trust,
            &input,
        )
        .map_err(|source| RuntimeError::GateEvaluationFailed { source })?;
        Self::verify_bound_gate_authority(&record, &open.gate, open.opening_seq)?;
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
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        Self::check_bound_binding(state, &open.gate)?;
        let record = evaluate_bound_timeout(
            &open.gate,
            &open.requester,
            open.opened_at_ms,
            open.opening_seq,
            now_ms,
        )
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

    fn check_bound_binding(
        state: &crate::shard::types::RunState,
        gate: &BoundManualGate,
    ) -> RuntimeResult<()> {
        let binding = gate.binding;
        let admission = state
            .admission
            .as_ref()
            .ok_or_else(|| binding_error(gate, "admission"))?;
        if admission.artifact_digest().as_bytes() != binding.artifact_digest.as_bytes() {
            return Err(binding_error(gate, "artifact_digest"));
        }
        if state.run_attempt.get() != binding.attempt {
            return Err(binding_error(gate, "attempt"));
        }
        match admission.policy_pin() {
            vb_storage::RunPolicyPin::Pinned(policy_digest)
                if policy_digest.as_bytes() == binding.policy_digest.as_bytes() => {}
            vb_storage::RunPolicyPin::Unverified => {
                return Err(binding_error(gate, "policy_digest"));
            }
            _ => return Err(binding_error(gate, "policy_digest")),
        }
        if admission.input_digest()
            != vb_storage::RunInputBinding::Digest(vb_core::ids::WorkflowDigest::from_bytes(
                binding.input_digest.as_bytes(),
            ))
        {
            return Err(binding_error(gate, "input_digest"));
        }
        Ok(())
    }

    fn verify_bound_gate_authority(
        record: &BoundGateOutcomeRecord,
        gate: &BoundManualGate,
        opening_seq: u64,
    ) -> RuntimeResult<()> {
        let mismatch = || RuntimeError::GateEvaluationFailed {
            source: crate::manual_gate::GateError::BoundReceiptMismatch,
        };
        if !matches!(
            record.record.outcome,
            vb_core::manual_gate::GateOutcome::Approved
                | vb_core::manual_gate::GateOutcome::BreakGlassApproved
        ) {
            return Ok(());
        }
        let receipt = record
            .bound_receipt
            .as_ref()
            .filter(|receipt| receipt.opening_seq == opening_seq)
            .ok_or_else(mismatch)?;
        match crate::manual_gate::verify_bound_receipt(receipt, gate) {
            Ok(true) => Ok(()),
            Ok(false) => Err(mismatch()),
            Err(source) => Err(RuntimeError::GateEvaluationFailed { source }),
        }
    }
}

fn binding_error(gate: &BoundManualGate, field: &'static str) -> RuntimeError {
    RuntimeError::GateEvaluationFailed {
        source: crate::manual_gate::GateError::BoundGateBindingMismatch {
            gate: gate.gate.id.get(),
            field,
        },
    }
}
