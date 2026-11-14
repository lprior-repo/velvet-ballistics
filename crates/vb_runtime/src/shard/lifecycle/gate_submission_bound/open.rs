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
        self.register_open_bound_gate(gate, opened_at_ms, requester)
    }

    fn register_open_bound_gate(
        &mut self,
        gate: BoundManualGate,
        opened_at_ms: u64,
        requester: ApproverId,
    ) -> RuntimeResult<()> {
        let run = gate.gate.run;
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
}
