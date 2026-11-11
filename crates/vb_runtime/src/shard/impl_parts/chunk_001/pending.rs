impl Shard {
    /// Inserts a pending timer for the given run ID.
    pub fn pending_timer_insert(
        &mut self,
        run_id: RunId,
        timer: PendingTimer,
    ) -> RuntimeResult<Option<PendingTimer>> {
        self.run_aggregate
            .pending_timer_insert(&self.runs, self.max_active_runs, run_id, timer)
    }

    /// Returns the pending timer for the given run ID, if it exists.
    #[must_use]
    pub fn pending_timer_get(&self, run_id: RunId) -> Option<PendingTimer> {
        self.run_aggregate.pending_timer_get(run_id)
    }

    /// Returns a clone of all pending timers.
    #[must_use]
    pub fn pending_timer_clone(&self) -> IndexMap<RunId, PendingTimer> {
        self.run_aggregate.pending_timer_clone()
    }

    /// Removes and returns the pending timer for the given run ID.
    pub fn pending_timer_remove(&mut self, run_id: RunId) -> Option<PendingTimer> {
        self.run_aggregate.pending_timer_remove(run_id)
    }

    /// Returns true if a pending timer exists for the given run ID.
    #[must_use]
    pub fn pending_timer_contains(&self, run_id: RunId) -> bool {
        self.run_aggregate.pending_timer_contains(run_id)
    }

    pub(crate) fn pending_timer_iter(&self) -> impl Iterator<Item = (&RunId, &PendingTimer)> {
        self.run_aggregate.pending_timer_iter()
    }

    pub(crate) fn clear_pending_timers(&mut self) {
        self.run_aggregate.pending_timer_clear();
    }

    /// Inserts an in-flight action ticket for the given run ID.
    pub fn pending_action_insert(
        &mut self,
        run_id: RunId,
        ticket: vb_core::action::ActionTicket,
    ) -> RuntimeResult<Option<vb_core::action::ActionTicket>> {
        self.run_aggregate
            .pending_action_insert(&self.runs, self.max_active_runs, run_id, ticket)
    }

    /// Returns the in-flight action ticket for the given run, if any.
    #[must_use]
    pub fn pending_action_get(&self, run_id: RunId) -> Option<vb_core::action::ActionTicket> {
        self.run_aggregate.pending_action_get(run_id)
    }

    #[must_use]
    pub(crate) fn pending_action_len(&self) -> usize {
        self.run_aggregate.pending_action_len()
    }

    /// Returns a clone of all pending action tickets.
    #[must_use]
    pub fn pending_action_clone(&self) -> IndexMap<RunId, vb_core::action::ActionTicket> {
        self.run_aggregate.pending_action_clone()
    }

    /// Removes and returns the in-flight action ticket for the given
    /// run ID.
    pub fn pending_action_remove(
        &mut self,
        run_id: RunId,
    ) -> Option<vb_core::action::ActionTicket> {
        self.run_aggregate.pending_action_remove(run_id)
    }

    pub(crate) fn pending_action_iter(
        &self,
    ) -> impl Iterator<Item = (&RunId, &vb_core::action::ActionTicket)> {
        self.run_aggregate.pending_action_iter()
    }
}
