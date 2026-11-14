impl Shard {
    /// Advances the deterministic clock to the given tick.
    ///
    /// The new tick must be >= the current tick. Returns an error if
    /// the supplied tick is in the past, preventing backward clock jumps.
    ///
    /// This operates the numeric timer seam alongside the existing
    /// wall-clock `Instant`-based timers; it does not modify or affect
    /// `Instant`-derived deadlines.
    pub fn advance_clock_to(&mut self, new_tick: TimerTick) -> RuntimeResult<()> {
        if new_tick < self.current_tick {
            return Err(RuntimeError::InvalidTimerFire);
        }
        self.current_tick = new_tick;
        Ok(())
    }

    /// Returns the current tick of the deterministic clock.
    #[must_use]
    pub fn current_tick(&self) -> TimerTick {
        self.current_tick
    }

    /// Returns the next freshness generation for a run's pending timer counter.
    ///
    /// - `Some(n)` where `n > 0` is the next generation to use.
    /// - `None` if generation would overflow `u64`.
    ///
    /// If no timer exists for the run, returns `Some(1)`.
    ///
    /// This counter is diagnostic for non-journaled timer-wheel use. It is not
    /// authority for journal-backed timers; those use the durable schedule
    /// event sequence and the captured current entry.
    #[must_use]
    pub fn next_pending_timer_generation(&self, run: RunId) -> Option<u64> {
        match self.pending_timer_get(run) {
            Some(timer) => timer.generation.checked_add(1),
            None => Some(1),
        }
    }

    /// Returns frame pool metrics across all pools: (free, total_capacity).
    #[must_use]
    pub fn frame_pool_metrics(&self) -> (usize, usize) {
        let mut free = 0usize;
        let mut total = 0usize;
        for pool in self.frame_pools.values() {
            free = free.saturating_add(pool.available());
            total = total.saturating_add(pool.capacity());
        }
        (free, total)
    }

    /// Processes one command from the queue. Returns false if the shard should shut down.
    pub fn tick(&mut self) -> RuntimeResult<bool> {
        self.completion_capture.reset();
        self.tick_inner()
    }

    pub(crate) fn tick_observed(
        &mut self,
    ) -> (
        RuntimeResult<bool>,
        crate::runtime::completion_observation::CompletionObservationSlot,
    ) {
        self.completion_capture.reset();
        self.completion_capture.enabled = true;
        let mut guard = CompletionCaptureGuard { shard: self };
        guard.tick()
    }

    fn drive_one_ready_run(&mut self) -> RuntimeResult<()> {
        let now = std::time::Instant::now();
        let due = self
            .pending_timer_iter()
            .filter(|(_, timer)| timer.deadline <= now)
            .min_by_key(|(run, timer)| (timer.deadline, run.get()))
            .map(|(run, timer)| (*run, *timer));
        let Some((run, timer)) = due else {
            return self.drive_one_pending_run();
        };
        self.handle_timer(run, timer.generation, timer.deadline, timer.kind)
    }

    fn tick_inner(&mut self) -> RuntimeResult<bool> {
        if self.shutting_down {
            return Ok(false);
        }

        self.expire_overdue_run_lifetime()?;

        let Some(queued) = self.command_queue.pop_queued() else {
            self.drive_one_ready_run()?;
            return Ok(true);
        };
        let pending_run = queued.command.run_id();
        self.pending_quota_reservation = queued
            .quota
            .zip(pending_run)
            .map(|(reservation, run)| (run, reservation));
        let cmd = queued.command;
        let submit_run = submission_run(&cmd);
        let result = self.dispatch_command(cmd);
        self.finish_queued_quota(submit_run, result)
    }

    fn runtime_action_failure_error(error: RuntimeError) -> RuntimeError {
        match error {
            RuntimeError::RunNotFound => RuntimeError::InvalidActionCompletion,
            other => other,
        }
    }

    /// Returns a reference to the shard counters.
    #[must_use]
    pub const fn counters(&self) -> &ShardCounters {
        &self.counters
    }
}
