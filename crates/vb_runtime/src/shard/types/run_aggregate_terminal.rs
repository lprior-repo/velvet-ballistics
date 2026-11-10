impl RunAggregate {
    pub(crate) fn terminal_contains(&self, run: RunId) -> bool {
        self.terminal_runs.contains(&run)
    }

    /// Retains terminal identity only after active run-state ownership has
    /// left `Shard::runs`, and clears runtime plus active-boundary side data
    /// for that run.
    pub(crate) fn terminal_insert(
        &mut self,
        runs: &IndexMap<RunId, RunState>,
        run: RunId,
        capacity: usize,
    ) -> RuntimeResult<bool> {
        if self.terminal_contains(run) {
            return Ok(false);
        }
        if runs.contains_key(&run) {
            return Err(crate::RuntimeError::RunAlreadyExists);
        }
        if !self.checked_out_contains(run) {
            return Err(crate::RuntimeError::RunNotFound);
        }
        self.reserve_terminal_slot(run, capacity)?;
        self.runtime_state_remove(run);
        self.checked_out_remove(run);
        let _removed_timer = self.pending_timers.swap_remove(&run);
        let _removed_action = self.pending_actions.swap_remove(&run);
        Ok(self.terminal_runs.insert(run))
    }

    pub(crate) fn terminal_remove(&mut self, run: RunId) {
        let _removed = self.terminal_runs.swap_remove(&run);
    }

    pub(crate) fn reserve_terminal_insert_slot(
        &mut self,
        run: RunId,
        capacity: usize,
    ) -> RuntimeResult<()> {
        if self.terminal_runs.contains(&run) {
            return Ok(());
        }
        if capacity == 0 {
            return Err(crate::RuntimeError::ActiveRunCapacityExceeded { capacity });
        }
        if self.terminal_runs.len() >= capacity {
            return Ok(());
        }
        self.terminal_runs
            .try_reserve(1)
            .map_err(|_| crate::RuntimeError::ActiveRunCapacityExceeded { capacity })
    }

    fn reserve_terminal_slot(&mut self, run: RunId, capacity: usize) -> RuntimeResult<()> {
        if self.terminal_runs.contains(&run) {
            return Ok(());
        }
        if capacity == 0 {
            return Err(crate::RuntimeError::ActiveRunCapacityExceeded { capacity });
        }
        if self.terminal_runs.len() >= capacity {
            let evicted = self.terminal_runs.iter().next().copied();
            if let Some(evicted) = evicted {
                let _removed = self.terminal_runs.shift_remove(&evicted);
            }
        }
        self.terminal_runs
            .try_reserve(1)
            .map_err(|_| crate::RuntimeError::ActiveRunCapacityExceeded { capacity })
    }

    pub(crate) fn run_is_live_for_boundary(
        &self,
        runs: &IndexMap<RunId, RunState>,
        run: RunId,
    ) -> bool {
        !self.terminal_contains(run) && (runs.contains_key(&run) || self.checked_out_contains(run))
    }

    pub(crate) fn pending_timer_len(&self) -> usize {
        self.pending_timers.len()
    }

    pub(crate) fn reserve_pending_timer_slot(
        &mut self,
        run: RunId,
        capacity: usize,
    ) -> RuntimeResult<()> {
        if self.pending_timers.contains_key(&run) {
            return Ok(());
        }
        if self.pending_timers.len() >= capacity {
            return Err(crate::RuntimeError::ActiveRunCapacityExceeded { capacity });
        }
        self.pending_timers
            .try_reserve(1)
            .map_err(|_| crate::RuntimeError::ActiveRunCapacityExceeded { capacity })
    }

    pub(crate) fn pending_timer_insert(
        &mut self,
        runs: &IndexMap<RunId, RunState>,
        capacity: usize,
        run: RunId,
        timer: PendingTimer,
    ) -> RuntimeResult<Option<PendingTimer>> {
        if !self.run_is_live_for_boundary(runs, run) {
            return Err(crate::RuntimeError::RunNotFound);
        }
        self.reserve_pending_timer_slot(run, capacity)?;
        Ok(self.pending_timers.insert(run, timer))
    }

    pub(crate) fn pending_timer_get(&self, run: RunId) -> Option<PendingTimer> {
        self.pending_timers.get(&run).copied()
    }

    pub(crate) fn pending_timer_clone(&self) -> IndexMap<RunId, PendingTimer> {
        self.pending_timers.clone()
    }

    pub(crate) fn pending_timer_remove(&mut self, run: RunId) -> Option<PendingTimer> {
        self.pending_timers.swap_remove(&run)
    }

    pub(crate) fn pending_timer_contains(&self, run: RunId) -> bool {
        self.pending_timers.contains_key(&run)
    }

    pub(crate) fn pending_timer_iter(&self) -> impl Iterator<Item = (&RunId, &PendingTimer)> {
        self.pending_timers.iter()
    }

    pub(crate) fn pending_timer_clear(&mut self) {
        self.pending_timers.clear();
    }

    pub(crate) fn pending_action_len(&self) -> usize {
        self.pending_actions.len()
    }

    pub(crate) fn reserve_pending_action_slot(
        &mut self,
        run: RunId,
        capacity: usize,
    ) -> RuntimeResult<()> {
        if self.pending_actions.contains_key(&run) {
            return Ok(());
        }
        if self.pending_actions.len() >= capacity {
            return Err(crate::RuntimeError::ActiveRunCapacityExceeded { capacity });
        }
        self.pending_actions
            .try_reserve(1)
            .map_err(|_| crate::RuntimeError::ActiveRunCapacityExceeded { capacity })
    }

    pub(crate) fn pending_action_insert(
        &mut self,
        runs: &IndexMap<RunId, RunState>,
        capacity: usize,
        run: RunId,
        ticket: ActionTicket,
    ) -> RuntimeResult<Option<ActionTicket>> {
        if !self.run_is_live_for_boundary(runs, run) {
            return Err(crate::RuntimeError::RunNotFound);
        }
        self.reserve_pending_action_slot(run, capacity)?;
        Ok(self.pending_actions.insert(run, ticket))
    }

    pub(crate) fn pending_action_get(&self, run: RunId) -> Option<ActionTicket> {
        self.pending_actions.get(&run).copied()
    }

    pub(crate) fn pending_action_clone(&self) -> IndexMap<RunId, ActionTicket> {
        self.pending_actions.clone()
    }

    pub(crate) fn pending_action_remove(&mut self, run: RunId) -> Option<ActionTicket> {
        self.pending_actions.swap_remove(&run)
    }

    pub(crate) fn pending_action_iter(&self) -> impl Iterator<Item = (&RunId, &ActionTicket)> {
        self.pending_actions.iter()
    }
}
