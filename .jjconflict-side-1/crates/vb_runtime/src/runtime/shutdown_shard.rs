impl Runtime {
    /// Stops command processing and durably drains the owned journal.
    ///
    /// Suspended runs remain owned by their shards. The report preserves their
    /// boundary authorities before timer registrations are released. On failure
    /// those registrations remain available for inspection and retry.
    pub fn shutdown_graceful(&mut self) -> Result<ShutdownReport, ShutdownFailure> {
        shutdown::shutdown(self)
    }

    fn shard_index(&self, run: RunId) -> usize {
        let hash = run.get();
        let Ok(count) = u64::try_from(self.shard_count) else {
            return 0;
        };
        let Some(remainder) = hash.checked_rem(count) else {
            return 0;
        };
        let Ok(index) = usize::try_from(remainder) else {
            return 0;
        };
        index
    }

    /// Routes to the shard that currently owns `run` (vb-906ix).
    ///
    /// Existing runs are routed by owner scan so commands reach the run
    /// after migration; a run with no owner yet (new submission, or an
    /// unknown run) falls back to its home shard, preserving the previous
    /// placement and error behavior.
    fn shard_for(&self, run: RunId) -> Result<&Shard, RuntimeError> {
        match self.shard_for_run(run) {
            Ok(shard) => Ok(shard),
            Err(_) => {
                let index = self.shard_index(run);
                self.shards.get(index).ok_or(RuntimeError::RunNotFound)
            }
        }
    }

    /// Mutable variant of [`Self::shard_for`] (vb-906ix).
    fn shard_for_mut(&mut self, run: RunId) -> Result<&mut Shard, RuntimeError> {
        let owner_index = self
            .shards
            .iter()
            .position(|shard| shard.run_state_contains(run) || shard.terminal_runs_contains(run))
            .unwrap_or_else(|| self.shard_index(run));
        self.shards
            .get_mut(owner_index)
            .ok_or(RuntimeError::RunNotFound)
    }

    /// Returns the shard that currently owns `run`, scanning all shards.
    ///
    /// RA-030 wave-15 (vb-sxkz6): the hash-based `shard_index(run)` returns
    /// the home shard which may differ from the owner after `migrate_shard`
    /// transfers a run's commands and run-state to a different shard. We
    /// check the home shard first as a fast path; on miss we scan the
    /// remaining shards in deterministic slice order.
    ///
    /// Returns `Err(RuntimeError::RunNotFound)` if `run` is not in any
    /// shard's `run_state` or `terminal_runs` set.
    fn shard_for_run(&self, run: RunId) -> Result<&Shard, RuntimeError> {
        let home_index = self.shard_index(run);
        if let Some(shard) = self.shards.get(home_index)
            && (shard.run_state_contains(run) || shard.terminal_runs_contains(run))
        {
            return Ok(shard);
        }
        self.shards
            .iter()
            .enumerate()
            .find(|(idx, shard)| {
                *idx != home_index
                    && (shard.run_state_contains(run) || shard.terminal_runs_contains(run))
            })
            .map(|(_, shard)| shard)
            .ok_or(RuntimeError::RunNotFound)
    }

    /// Mutable variant of `shard_for_run` for `take_inspect_response`.
    fn shard_for_run_mut(&mut self, run: RunId) -> Result<&mut Shard, RuntimeError> {
        // Mirror the home-fast-path + scan-fallback logic on immutable borrow,
        // then re-index after borrow ends to get the mutable shard.
        let home_index = self.shard_index(run);
        let owner_index = {
            if let Some(shard) = self.shards.get(home_index)
                && (shard.run_state_contains(run) || shard.terminal_runs_contains(run))
            {
                home_index
            } else {
                self.shards
                    .iter()
                    .position(|shard| {
                        shard.run_state_contains(run) || shard.terminal_runs_contains(run)
                    })
                    .ok_or(RuntimeError::RunNotFound)?
            }
        };
        self.shards
            .get_mut(owner_index)
            .ok_or(RuntimeError::RunNotFound)
    }
}
