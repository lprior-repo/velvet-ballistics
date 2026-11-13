// Open-gate registry helpers shared by legacy and bound gate paths.

impl Shard {
    /// Returns the first open gate id for a run, in opening order, if any.
    pub(crate) fn open_gate_for(&self, run: RunId) -> Option<GateId> {
        self.open_gates
            .values()
            .find(|open| open.gate.run == run)
            .map(|open| open.gate.id)
            .or_else(|| {
                self.bound_open_gates
                    .values()
                    .find(|open| open.gate.gate.run == run)
                    .map(|open| open.gate.gate.id)
            })
    }

    /// Counts open gates for one run across both registries.
    pub(crate) fn open_gate_count(&self, run: RunId) -> u64 {
        let legacy = self
            .open_gates
            .values()
            .filter(|open| open.gate.run == run)
            .fold(0u64, |count, _| count.saturating_add(1));
        self.bound_open_gates
            .values()
            .filter(|open| open.gate.gate.run == run)
            .fold(legacy, |count, _| count.saturating_add(1))
    }

    /// Removes every open gate owned by a terminal run.
    pub(crate) fn retire_run_gates(&mut self, run: RunId) {
        self.open_gates.retain(|_, open| open.gate.run != run);
        self.bound_open_gates
            .retain(|_, open| open.gate.gate.run != run);
    }

    /// Reserves one slot in the shard-wide open-gate registry.
    ///
    /// Total open gates are capped at the shard's active-run capacity:
    /// every open gate belongs to an active run, and the registry must
    /// stay bounded even for runs admitted without a budget.
    fn reserve_open_gate_slot(&mut self) -> RuntimeResult<()> {
        let total = self
            .open_gates
            .len()
            .saturating_add(self.bound_open_gates.len());
        if total >= self.max_active_runs {
            return Err(RuntimeError::ActiveRunCapacityExceeded {
                capacity: self.max_active_runs,
            });
        }
        self.open_gates
            .try_reserve(1)
            .map_err(|_| RuntimeError::ActiveRunCapacityExceeded {
                capacity: self.max_active_runs,
            })?;
        self.bound_open_gates
            .try_reserve(1)
            .map_err(|_| RuntimeError::ActiveRunCapacityExceeded {
                capacity: self.max_active_runs,
            })
    }
}
