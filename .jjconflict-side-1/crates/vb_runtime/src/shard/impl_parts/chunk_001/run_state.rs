impl Shard {
    /// Re-injects a pending action ticket recovered from journal replay.
    ///
    /// When a run was suspended with a pending action (sided effect in
    /// flight), recovery must restore that action to the shard's pending
    /// action map so it can be matched with a completion event.
    pub(crate) fn reinject_pending_action(
        &mut self,
        run: RunId,
        ticket: vb_core::action::ActionTicket,
    ) -> RuntimeResult<()> {
        self.fencing_tokens.restore(run, ticket.fencing_token);
        self.pending_action_insert(run, ticket)?;
        Ok(())
    }

    pub(crate) fn add_executed_step_delta(&mut self, run: RunId, executed: u64) {
        let previous = self
            .accounted_executed_steps
            .get(&run)
            .copied()
            .map_or(0, |value| value);
        let Some(delta) = executed.checked_sub(previous) else {
            return;
        };
        if delta == 0 {
            return;
        }
        self.counters.add_steps(delta);
        self.accounted_executed_steps.insert(run, executed);
    }

    pub(crate) fn clear_executed_step_accounting(&mut self, run: RunId) {
        self.accounted_executed_steps.swap_remove(&run);
    }

    /// Returns the number of pending timers on this shard.
    #[must_use]
    pub fn pending_timer_count(&self) -> usize {
        self.run_aggregate.pending_timer_len()
    }

    /// Returns the run state for the given run ID, if it exists.
    #[must_use]
    pub fn run_state_get(&self, run_id: RunId) -> Option<&RunState> {
        self.runs.get(&run_id)
    }

    /// Returns a mutable reference to the run state for the given run ID, if it exists.
    #[must_use]
    pub fn run_state_get_mut(&mut self, run_id: RunId) -> Option<&mut RunState> {
        self.runs.get_mut(&run_id)
    }

    /// Iterates active run state without exposing direct map mutation.
    pub(crate) fn active_runs_iter(&self) -> impl Iterator<Item = (&RunId, &RunState)> {
        self.runs.iter()
    }

    /// Returns true if a run with the given ID exists.
    #[must_use]
    pub fn run_state_contains(&self, run_id: RunId) -> bool {
        self.runs.contains_key(&run_id)
    }

    /// Removes and returns the run state for the given run ID.
    pub(crate) fn run_state_remove(&mut self, run_id: RunId) -> Option<RunState> {
        let state = self.runs.swap_remove(&run_id)?;
        if !self.checked_out_run_contains(run_id) {
            self.runtime_state_remove(run_id);
            let _removed_timer = self.pending_timer_remove(run_id);
            let _removed_action = self.pending_action_remove(run_id);
            let _removed_ask_authority = self.pending_ask_authorities.swap_remove(&run_id);
        }
        Some(state)
    }

    /// Computes the ABI digest for a specific action on a run.
    ///
    /// Looks up the action contract for the given action ID from the run's
    /// action contracts, serializes it, and returns the blake3 digest.
    pub(crate) fn action_abi_digest_for_run_action(
        &self,
        run: RunId,
        action: ActionId,
    ) -> RuntimeResult<WorkflowDigest> {
        let state = self.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        // Find the contract for this action
        let contract = state
            .action_contracts
            .iter()
            .find(|c| c.id == action)
            .ok_or(RuntimeError::UnsupportedOperation {
                operation: "action_contract_not_found",
            })?;
        // Compute ABI digest from the contract
        crate::recovery::action_contract_abi_digest(contract)
    }

    /// Returns the runtime state for the given run ID, if it exists.
    #[must_use]
    pub fn runtime_state_get(&self, run_id: RunId) -> Option<RuntimeState> {
        self.run_aggregate.runtime_state_get(run_id)
    }

    /// Inserts a non-terminal runtime state for the given run ID.
    pub(crate) fn runtime_state_insert(
        &mut self,
        run_id: RunId,
        state: RuntimeState,
    ) -> RuntimeResult<Option<RuntimeState>> {
        self.run_aggregate
            .runtime_state_insert(&self.runs, run_id, state, self.max_active_runs)
    }

    /// Removes the runtime state for the given run ID, if it exists.
    pub(crate) fn runtime_state_remove(&mut self, run_id: RunId) {
        if self.run_state_contains(run_id) || self.checked_out_run_contains(run_id) {
            return;
        }
        self.run_aggregate.runtime_state_remove(run_id);
    }

    /// Clears runtime state for a terminal event apply path, including when
    /// the run is checked out of `runs` for deterministic drive cleanup.
    pub(crate) fn runtime_state_terminal_clear(&mut self, run_id: RunId) {
        self.run_aggregate.runtime_state_terminal_clear(run_id);
    }

    /// Returns true if the given run ID is in the terminal state.
    #[must_use]
    pub fn terminal_runs_contains(&self, run_id: RunId) -> bool {
        self.run_aggregate.terminal_contains(run_id)
    }
}
