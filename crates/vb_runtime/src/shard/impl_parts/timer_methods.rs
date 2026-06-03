impl Shard {
    /// Returns the number of pending timers on this shard.
    #[must_use]
    pub fn pending_timer_count(&self) -> usize {
        self.pending_timers.len()
    }

    /// Inserts a pending timer for the given run ID.
    pub fn pending_timer_insert(&mut self, run_id: RunId, timer: PendingTimer) -> Option<PendingTimer> {
        self.pending_timers.insert(run_id, timer)
    }

    /// Returns the pending timer for the given run ID, if it exists.
    #[must_use]
    pub fn pending_timer_get(&self, run_id: RunId) -> Option<PendingTimer> {
        self.pending_timers.get(&run_id).copied()
    }

    /// Returns a clone of all pending timers.
    #[must_use]
    pub fn pending_timer_clone(&self) -> IndexMap<RunId, PendingTimer> {
        self.pending_timers.clone()
    }

    /// Removes and returns the pending timer for the given run ID.
    pub fn pending_timer_remove(&mut self, run_id: RunId) -> Option<PendingTimer> {
        self.pending_timers.swap_remove(&run_id)
    }

    /// Returns true if a pending timer exists for the given run ID.
    #[must_use]
    pub fn pending_timer_contains(&self, run_id: RunId) -> bool {
        self.pending_timers.contains_key(&run_id)
    }

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

    /// Returns the next freshness generation for a run's pending timer.
    ///
    /// - `Some(n)` where `n > 0` is the next generation to use.
    /// - `None` if generation would overflow `u64`.
    ///
    /// If no timer exists for the run, returns `Some(1)`.
    #[must_use]
    pub fn next_pending_timer_generation(&self, run: RunId) -> Option<u64> {
        match self.pending_timers.get(&run).copied() {
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

    /// Builds a fail-closed legacy timer-fired command without fabricating authority.
    #[must_use]
    pub fn timer_fired_command(&self, run: RunId) -> ShardCommand {
        ShardCommand::TimerFired {
            run,
            generation: 0,
            deadline: std::time::Instant::now(),
            kind: PendingTimerKind::Wait,
        }
    }

    /// Returns the current typed timer authority for explicit capture.
    #[must_use]
    pub fn timer_entry(&self, run: RunId) -> Option<crate::shard::timer_wheel::TimerEntry> {
        self.pending_timers
            .get(&run)
            .copied()
            .map(|timer| crate::shard::timer_wheel::TimerEntry {
                run,
                generation: timer.generation,
                deadline: timer.deadline,
                kind: timer.kind,
            })
    }
}
