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

    /// Builds a timer-fired `ShardCommand` carrying the originally-armed
    /// deadline and generation token for the run, or returns `None` if no
    /// pending timer is currently registered for `run`.
    ///
    /// This lookup-based authority transfer honors master spec §20: fired-timer
    /// commands must carry the deadline and generation token that were stored
    /// at timer-arm time, never a freshly synthesized `Instant::now()` value.
    #[must_use]
    pub fn timer_fired_command(&self, run: RunId) -> Option<ShardCommand> {
        self.pending_timer_get(run).map(|timer| ShardCommand::TimerFired {
            run,
            generation: timer.generation,
            deadline: timer.deadline,
            kind: timer.kind,
        })
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

#[cfg(test)]
mod timer_fired_command_tests {
    use std::time::Instant;

    use vb_core::ids::StepIdx;
    use vb_core::policy::RuntimePolicy;

    use crate::RuntimeError;
    use crate::shard::types::{PendingTimer, PendingTimerKind};
    use crate::shard::{RunId, Shard, ShardCommand, ShardConfig};

    fn small_config() -> ShardConfig {
        ShardConfig {
            command_queue_capacity: 16,
            trace_capacity: 16,
            step_budget_per_tick: 4,
            max_active_runs: 4,
            policy: RuntimePolicy::Relaxed,
            coalesce_window_ticks: 1,
            snapshot_interval_steps: 0,
            max_terminal_runs: 16,
            terminal_runs_ttl_ticks: 86_400,
        }
    }

    #[test]
    fn timer_fired_command_returns_none_when_no_pending_timer() -> Result<(), RuntimeError> {
        // Given an empty shard with no pending timers.
        let shard = Shard::new(small_config())?;
        let run = RunId::new(42_001);

        // When timer_fired_command is called for a run with no pending timer.
        let result = shard.timer_fired_command(run);

        // Then it must return None rather than fabricating authority.
        assert!(result.is_none(), "no pending timer must yield None");
        Ok(())
    }

    #[test]
    fn timer_fired_command_returns_real_deadline_when_pending_timer_exists()
    -> Result<(), RuntimeError> {
        // Given a shard with one real pending timer armed for a run.
        let mut shard = Shard::new(small_config())?;
        let run = RunId::new(42_002);
        let armed_deadline = Instant::now();
        let armed_timer = PendingTimer {
            step: StepIdx::new(7),
            kind: PendingTimerKind::Ask,
            generation: 13,
            deadline: armed_deadline,
        };
        assert_eq!(shard.pending_timer_insert(run, armed_timer), None);

        let expected = Some(ShardCommand::TimerFired {
            run,
            generation: armed_timer.generation,
            deadline: armed_deadline,
            kind: armed_timer.kind,
        });
        assert_eq!(shard.timer_fired_command(run), expected);
        Ok(())
    }
}
