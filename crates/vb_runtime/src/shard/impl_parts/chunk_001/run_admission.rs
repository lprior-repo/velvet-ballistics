impl Shard {
    /// Inserts a run state for the given run ID.
    pub(crate) fn run_state_insert(
        &mut self,
        run_id: RunId,
        state: RunState,
    ) -> RuntimeResult<Option<RunState>> {
        if self.terminal_runs_contains(run_id) || self.run_state_contains(run_id) {
            return Err(RuntimeError::RunAlreadyExists);
        }
        if !self.run_state_has_aggregate_visibility(run_id) {
            return Err(RuntimeError::RunNotFound);
        }
        self.reserve_run_state_slot(run_id)?;
        let previous = self.runs.insert(run_id, state);
        self.run_aggregate.checked_out_remove(run_id);
        Ok(previous)
    }

    fn run_state_has_aggregate_visibility(&self, run_id: RunId) -> bool {
        self.runtime_state_get(run_id).is_some() || self.checked_out_run_contains(run_id)
    }

    pub(crate) fn admit_run_state(
        &mut self,
        run_id: RunId,
        state: RunState,
        runtime_state: RuntimeState,
    ) -> RuntimeResult<()> {
        self.validate_new_active_run(run_id, runtime_state)?;
        self.reserve_run_state_slot(run_id)?;
        self.reserve_runtime_state_slot(run_id)?;
        let _previous = self.runs.insert(run_id, state);
        match self.runtime_state_insert(run_id, runtime_state) {
            Ok(_) => Ok(()),
            Err(error) => {
                let _removed = self.runs.swap_remove(&run_id);
                Err(error)
            }
        }
    }

    fn validate_new_active_run(
        &self,
        run_id: RunId,
        runtime_state: RuntimeState,
    ) -> RuntimeResult<()> {
        if runtime_state == RuntimeState::Failed {
            return Err(RuntimeError::UnsupportedOperation {
                operation: "runtime_state_failed_terminal_split",
            });
        }
        if self.run_state_contains(run_id)
            || self.checked_out_run_contains(run_id)
            || self.runtime_state_get(run_id).is_some()
            || self.terminal_runs_contains(run_id)
        {
            return Err(RuntimeError::RunAlreadyExists);
        }
        Ok(())
    }

    pub(crate) fn checked_out_run_insert(&mut self, run_id: RunId) -> RuntimeResult<()> {
        self.run_aggregate
            .checked_out_insert(run_id, self.max_active_runs)
    }

    pub(crate) fn checked_out_run_remove(&mut self, run_id: RunId) {
        self.run_aggregate.checked_out_remove(run_id);
    }

    pub(crate) fn checked_out_run_contains(&self, run_id: RunId) -> bool {
        self.run_aggregate.checked_out_contains(run_id)
    }

    pub(crate) fn checked_out_run_iter(&self) -> impl Iterator<Item = &RunId> {
        self.run_aggregate.checked_out_iter()
    }

    /// Inserts a run into the terminal runs set.
    pub fn terminal_runs_insert(&mut self, run_id: RunId) -> RuntimeResult<bool> {
        self.run_aggregate
            .terminal_insert(&self.runs, run_id, self.max_active_runs)
    }

    pub(crate) fn reserve_terminal_run_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        self.run_aggregate
            .reserve_terminal_insert_slot(run_id, self.max_active_runs)
    }

    /// Removes a run from the terminal runs set.
    pub fn terminal_runs_remove(&mut self, run_id: RunId) {
        self.run_aggregate.terminal_remove(run_id);
    }
}
