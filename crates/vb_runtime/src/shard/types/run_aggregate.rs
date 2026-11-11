/// Private aggregate for run membership side data. Runtime lifecycle state and
/// terminal membership are co-owned here so terminal identity cannot coexist
/// with stale resumability state.
pub(crate) struct RunAggregate {
    runtime_states: IndexMap<RunId, RuntimeState>,
    checked_out_runs: IndexSet<RunId>,
    terminal_runs: IndexSet<RunId>,
    /// Timer wheel owns deadline/run/kind/generation indexing.
    pending_timer_wheel: TimerWheel,
    /// Sidecar preserves step metadata and deterministic iteration order.
    pending_timer_steps: IndexMap<RunId, StepIdx>,
    pending_actions: IndexMap<RunId, ActionTicket>,
}

impl RunAggregate {
    pub(crate) fn new() -> Self {
        Self {
            runtime_states: IndexMap::new(),
            checked_out_runs: IndexSet::new(),
            terminal_runs: IndexSet::new(),
            pending_timer_wheel: TimerWheel::new(),
            pending_timer_steps: IndexMap::new(),
            pending_actions: IndexMap::new(),
        }
    }

    pub(crate) fn reserve_runtime_state_slot(
        &mut self,
        run: RunId,
        capacity: usize,
    ) -> RuntimeResult<()> {
        if self.runtime_states.contains_key(&run) {
            return Ok(());
        }
        if self.runtime_states.len() >= capacity {
            return Err(crate::RuntimeError::ActiveRunCapacityExceeded { capacity });
        }
        self.runtime_states
            .try_reserve(1)
            .map_err(|_| crate::RuntimeError::ActiveRunCapacityExceeded { capacity })
    }

    pub(crate) fn runtime_state_get(&self, run: RunId) -> Option<RuntimeState> {
        self.runtime_states.get(&run).copied()
    }

    /// Inserts a non-terminal runtime state only when the run is not retained
    /// as terminal. Terminal failures are represented by `terminal_runs`, not a
    /// stale `RuntimeState::Failed` entry.
    pub(crate) fn runtime_state_insert(
        &mut self,
        runs: &IndexMap<RunId, RunState>,
        run: RunId,
        state: RuntimeState,
        capacity: usize,
    ) -> RuntimeResult<Option<RuntimeState>> {
        if state == RuntimeState::Failed {
            return Err(crate::RuntimeError::UnsupportedOperation {
                operation: "runtime_state_failed_terminal_split",
            });
        }
        if self.terminal_contains(run) {
            return Err(crate::RuntimeError::RunAlreadyExists);
        }
        if !runs.contains_key(&run) && !self.checked_out_contains(run) {
            return Err(crate::RuntimeError::RunNotFound);
        }
        self.reserve_runtime_state_slot(run, capacity)?;
        Ok(self.runtime_states.insert(run, state))
    }

    pub(crate) fn runtime_state_remove(&mut self, run: RunId) {
        let _removed = self.runtime_states.swap_remove(&run);
    }

    /// Clears only runtime lifecycle state for terminal `RuntimeEvent` apply.
    /// Checked-out ownership and pending boundaries are cleaned by terminal
    /// lifecycle paths, not by this state-machine transition.
    pub(crate) fn runtime_state_terminal_clear(&mut self, run: RunId) {
        self.runtime_state_remove(run);
    }

    pub(crate) fn checked_out_len(&self) -> usize {
        self.checked_out_runs.len()
    }

    pub(crate) fn checked_out_contains(&self, run: RunId) -> bool {
        self.checked_out_runs.contains(&run)
    }

    pub(crate) fn checked_out_iter(&self) -> impl Iterator<Item = &RunId> {
        self.checked_out_runs.iter()
    }

    pub(crate) fn reserve_checked_out_slot(
        &mut self,
        run: RunId,
        capacity: usize,
    ) -> RuntimeResult<()> {
        if self.checked_out_contains(run) {
            return Ok(());
        }
        if self.checked_out_runs.len() >= capacity {
            return Err(crate::RuntimeError::ActiveRunCapacityExceeded { capacity });
        }
        self.checked_out_runs
            .try_reserve(1)
            .map_err(|_| crate::RuntimeError::ActiveRunCapacityExceeded { capacity })
    }

    pub(crate) fn checked_out_insert(&mut self, run: RunId, capacity: usize) -> RuntimeResult<()> {
        self.reserve_checked_out_slot(run, capacity)?;
        if self.checked_out_runs.insert(run) {
            Ok(())
        } else {
            Err(crate::RuntimeError::RunAlreadyExists)
        }
    }

    pub(crate) fn checked_out_remove(&mut self, run: RunId) {
        let _removed = self.checked_out_runs.swap_remove(&run);
    }
}
