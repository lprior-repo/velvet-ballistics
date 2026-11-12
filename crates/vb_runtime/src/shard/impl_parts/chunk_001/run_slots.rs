impl Shard {
    pub(crate) fn prepare_run_slots(&mut self, run_id: RunId) -> RuntimeResult<()> {
        self.reserve_run_state_slot(run_id)?;
        self.reserve_checked_out_run_slot(run_id)?;
        self.reserve_runtime_state_slot(run_id)?;
        self.reserve_journal_sequence_slot(run_id)?;
        self.reserve_pending_timer_slot(run_id)?;
        self.reserve_pending_action_slot(run_id)?;
        self.reserve_pending_ask_authority_slot(run_id)
    }

    fn reserve_index_map_slot<T>(
        slots: &mut IndexMap<RunId, T>,
        run_id: RunId,
        capacity: usize,
    ) -> RuntimeResult<()> {
        if slots.contains_key(&run_id) {
            return Ok(());
        }
        if slots.len() >= capacity {
            return Err(Self::run_capacity_error(capacity));
        }
        slots
            .try_reserve(1)
            .map_err(|_| Self::run_capacity_error(capacity))
    }

    fn reserve_run_state_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        Self::reserve_index_map_slot(&mut self.runs, run_id, self.max_active_runs)
    }

    pub(crate) fn reserve_checked_out_run_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        self.run_aggregate
            .reserve_checked_out_slot(run_id, self.max_active_runs)
    }

    fn reserve_runtime_state_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        self.run_aggregate
            .reserve_runtime_state_slot(run_id, self.max_active_runs)
    }

    fn reserve_journal_sequence_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        Self::reserve_index_map_slot(&mut self.journal_sequences, run_id, self.max_active_runs)
    }

    pub(crate) fn reserve_pending_timer_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        self.run_aggregate
            .reserve_pending_timer_slot(run_id, self.max_active_runs)
    }

    pub(crate) fn reserve_pending_ask_authority_slot(
        &mut self,
        run_id: RunId,
    ) -> RuntimeResult<()> {
        Self::reserve_index_map_slot(
            &mut self.pending_ask_authorities,
            run_id,
            self.max_active_runs,
        )
    }

    fn reserve_pending_action_slot(&mut self, run_id: RunId) -> RuntimeResult<()> {
        self.run_aggregate
            .reserve_pending_action_slot(run_id, self.max_active_runs)
    }
}
