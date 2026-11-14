// Restore durable open gates into the live shard registry after recovery.
//
// Both methods only restore evidence that the journal already validated:
// no journal append, no gate synthesis, and no run drive. A recovered gated
// run stays suspended until the operator decides.

impl Shard {
    /// Restores legacy gates reconstructed from durable journal evidence.
    pub(crate) fn restore_open_gates(
        &mut self,
        run: RunId,
        gates: &[OpenGate],
    ) -> RuntimeResult<()> {
        for open in gates {
            if open.gate.run != run {
                return Err(RuntimeError::Recovery {
                    error: format!(
                        "recovered open gate {} belongs to a different run",
                        open.gate.id.get()
                    ),
                });
            }
            if self.open_gates.contains_key(&open.gate.id)
                || self.bound_open_gates.contains_key(&open.gate.id)
            {
                return Err(RuntimeError::GateAlreadyOpen { gate: open.gate.id });
            }
            self.reserve_open_gate_slot()?;
            self.open_gates.insert(open.gate.id, open.clone());
        }
        Ok(())
    }

    /// Restores bound gates reconstructed from durable record kinds 54/55.
    pub(crate) fn restore_bound_open_gates(
        &mut self,
        run: RunId,
        gates: &[crate::shard::types::BoundOpenGate],
    ) -> RuntimeResult<()> {
        for open in gates {
            if open.gate.gate.run != run {
                return Err(RuntimeError::Recovery {
                    error: format!(
                        "recovered bound gate {} belongs to a different run",
                        open.gate.gate.id.get()
                    ),
                });
            }
            if self.open_gates.contains_key(&open.gate.gate.id)
                || self.bound_open_gates.contains_key(&open.gate.gate.id)
            {
                return Err(RuntimeError::GateAlreadyOpen {
                    gate: open.gate.gate.id,
                });
            }
            let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
            Self::check_bound_binding(state, &open.gate)?;
            self.reserve_open_gate_slot()?;
            self.bound_open_gates
                .insert(open.gate.gate.id, open.clone());
        }
        Ok(())
    }
}
