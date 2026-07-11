impl Shard {
    pub(crate) fn take_frame_for(
        &mut self,
        run: RunId,
        workflow: &CompiledWorkflow,
    ) -> RuntimeResult<RunFrame> {
        let step_count = workflow.node_count();
        let slot_count = workflow.slot_count();
        let key = (step_count, slot_count);
        if !self.frame_pools.contains_key(&key) {
            let pool = FramePool::new(step_count, slot_count, self.max_active_runs)
                .map_err(|_| RuntimeError::FramePoolUnavailable)?;
            self.frame_pools.insert(key, pool);
        }
        let pool = self
            .frame_pools
            .get_mut(&key)
            .ok_or(RuntimeError::FramePoolUnavailable)?;
        pool.take(run, workflow.entry())
            .map_err(|_| RuntimeError::FramePoolUnavailable)
    }

    pub(crate) fn release_frame(&mut self, frame: RunFrame) {
        let key = (frame.step_count(), frame.slot_count());
        if let Some(pool) = self.frame_pools.get_mut(&key) {
            pool.release(frame);
        }
    }

    /// Drains the command queue by processing commands until shutdown or capacity limit.
    pub fn drain_for_shutdown(&mut self) -> RuntimeResult<()> {
        let limit = self.command_queue.capacity();
        let mut processed = 0usize;
        while processed < limit {
            if !self.tick()? {
                self.clear_pending_timers();
                return Ok(());
            }
            processed = processed.saturating_add(1);
        }
        Err(RuntimeError::ShutdownInProgress)
    }

    /// Drains currently queued commands, then marks the shard shut down.
    pub fn drain_pending_and_shutdown(&mut self) -> RuntimeResult<()> {
        if self.shutting_down {
            self.clear_pending_timers();
            return Ok(());
        }
        self.drain_pending_commands(self.command_queue.len())?;
        self.shutting_down = true;
        self.clear_pending_timers();
        Ok(())
    }

    fn drain_pending_commands(&mut self, command_count: usize) -> RuntimeResult<()> {
        (0..command_count).try_for_each(|_| {
            if self.command_queue.is_empty() || self.shutting_down {
                return Ok(());
            }
            if !self.tick()? {
                self.clear_pending_timers();
            }
            Ok(())
        })
    }

    #[cfg(test)]
    pub(crate) fn transfer_active_run_to(
        &mut self,
        target: &mut Self,
        run: RunId,
    ) -> RuntimeResult<()> {
        let runtime_state = self.runtime_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        let pending_timer = self.pending_timer_get(run);
        let pending_action = self.pending_action_get(run);
        target.reserve_transfer_slots(run, pending_timer.is_some(), pending_action.is_some())?;
        let state = self.run_state_remove(run).ok_or(RuntimeError::RunNotFound)?;
        target.admit_run_state(run, state, runtime_state)?;
        target.restore_transfer_boundaries(run, pending_timer, pending_action)
    }

    #[cfg(test)]
    fn reserve_transfer_slots(
        &mut self,
        run: RunId,
        has_timer: bool,
        has_action: bool,
    ) -> RuntimeResult<()> {
        self.validate_new_active_run(run, RuntimeState::Running)?;
        self.reserve_run_state_slot(run)?;
        self.reserve_runtime_state_slot(run)?;
        if has_timer {
            self.reserve_pending_timer_slot(run)?;
        }
        if has_action {
            self.reserve_pending_action_slot(run)?;
        }
        Ok(())
    }

    #[cfg(test)]
    fn restore_transfer_boundaries(
        &mut self,
        run: RunId,
        pending_timer: Option<PendingTimer>,
        pending_action: Option<vb_core::action::ActionTicket>,
    ) -> RuntimeResult<()> {
        if let Some(timer) = pending_timer {
            self.pending_timer_insert(run, timer)?;
        }
        if let Some(ticket) = pending_action {
            self.pending_action_insert(run, ticket)?;
        }
        Ok(())
    }
}
