impl Runtime {
    /// Inspects run state.
    pub fn inspect_run(&self, run: RunId, correlation: u64) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Inspect { run, correlation })
    }

    /// Returns a direct, non-queued run snapshot from the owning shard.
    pub fn snapshot_run(&self, run: RunId, correlation: u64) -> RuntimeResult<InspectResponse> {
        let shard = self.shard_for(run)?;
        Ok(shard.snapshot_run(run, correlation))
    }

    #[must_use]
    pub fn pending_boundary_snapshot(
        &self,
        max_items_per_shard: usize,
    ) -> RuntimePendingBoundarySnapshot {
        let mut shards = Vec::with_capacity(self.shards.len());
        let mut totals = RuntimePendingBoundaryTotals::default();
        for (index, shard) in self.shards.iter().enumerate() {
            let snapshot =
                shard.pending_boundary_snapshot(shard_id_from_index(index), max_items_per_shard);
            totals.add_shard(&snapshot);
            shards.push(snapshot);
        }
        RuntimePendingBoundarySnapshot::new(shards.into_boxed_slice(), totals)
    }

    /// Processes one command on each shard. Returns false if any shard is shutting down.
    pub fn tick_all(&mut self) -> RuntimeResult<bool> {
        let mut alive = true;
        for shard in &mut self.shards {
            if !shard.tick()? {
                alive = false;
            }
        }
        Ok(alive)
    }

    fn preflight_observed_slots(
        slots: &mut [completion_observation::CompletionObservationSlot],
        required: usize,
    ) -> Result<
        &mut [completion_observation::CompletionObservationSlot],
        completion_observation::ObservationPreflightError,
    > {
        use completion_observation::ObservationPreflightError;

        let provided = slots.len();
        let required_slots = slots
            .get_mut(..required)
            .ok_or(ObservationPreflightError::BufferTooShort { required, provided })?;
        for (index, slot) in required_slots.iter().enumerate() {
            if slot.first.is_some() || slot.conflict {
                return Err(ObservationPreflightError::SlotNotEmpty { index });
            }
        }
        Ok(required_slots)
    }

    fn tick_observed_shards(
        &mut self,
        slots: &mut [completion_observation::CompletionObservationSlot],
    ) -> completion_observation::ObservedTickOutcome {
        use completion_observation::{ObservationConflict, ObservedTickOutcome};

        let mut alive = true;
        let mut observation = Ok(());
        for (shard, slot) in self.shards.iter_mut().zip(slots.iter_mut()) {
            let (execution, captured) = shard.tick_observed();
            *slot = captured;
            if slot.conflict {
                observation = Err(ObservationConflict::MultipleCompletions);
            }
            match execution {
                Ok(shard_alive) => alive &= shard_alive,
                Err(error) => {
                    return ObservedTickOutcome {
                        execution: Err(error),
                        observation,
                    };
                }
            }
        }
        ObservedTickOutcome {
            execution: Ok(alive),
            observation,
        }
    }

    /// Processes one command per shard with opt-in, caller-owned observations.
    ///
    /// Preflight rejects a short buffer first, then the lowest occupied required
    /// slot, before processing any shard. Extra tail slots are untouched.
    /// Each required slot captures its shard's first actual run/attempt; callers
    /// must filter that identity. Any second capture marks a sticky conflict,
    /// including a repeated identity, without replacing the first receipt.
    /// Execution errors preserve earlier receipts and stop later shards. A
    /// receipt can coexist with cleanup failure; absence on error does not prove
    /// that no terminal event was accepted. Neither error implies retry safety.
    pub fn tick_all_observed(
        &mut self,
        slots: &mut [completion_observation::CompletionObservationSlot],
    ) -> Result<
        completion_observation::ObservedTickOutcome,
        completion_observation::ObservationPreflightError,
    > {
        let required = self.shards.len();
        let required_slots = Self::preflight_observed_slots(slots, required)?;
        Ok(self.tick_observed_shards(required_slots))
    }

    /// Processes one tick on a specific shard with the given directive.
    ///
    /// This method directs a single shard's behavior for one tick. The directive
    /// determines what work the shard performs:
    ///
    /// - `Continue`: Process one command from the queue normally.
    /// - `Suspend`: Skip command processing; preserve queue depth.
    /// - `Migrate { target }`: Transfer all pending commands to the target shard.
    /// - `Shutdown`: Drain all remaining commands and shut down the shard.
    ///
    /// Returns `Ok(true)` if the shard is alive (continuing), `Ok(false)` if the
    /// shard has shut down, or an error if the shard index is invalid or migration
    /// failed.
    pub fn tick_shard(
        &mut self,
        shard_index: u32,
        directive: ShardDirective,
    ) -> RuntimeResult<bool> {
        let shard_index_usize = usize::try_from(shard_index)
            .map_err(|_| RuntimeError::ShardNotFound { shard: shard_index })?;

        // Validate source shard exists first
        if self.shards.get(shard_index_usize).is_none() {
            return Err(RuntimeError::ShardNotFound { shard: shard_index });
        }

        match directive {
            ShardDirective::Continue => {
                let shard = self
                    .shards
                    .get_mut(shard_index_usize)
                    .ok_or(RuntimeError::ShardNotFound { shard: shard_index })?;
                shard.tick()
            }
            ShardDirective::Suspend => {
                // Suspend: skip processing, preserve queue, return alive
                Ok(true)
            }
            ShardDirective::Migrate { target } => self.migrate_shard(shard_index_usize, target),
            ShardDirective::Shutdown => {
                let shard = self
                    .shards
                    .get_mut(shard_index_usize)
                    .ok_or(RuntimeError::ShardNotFound { shard: shard_index })?;
                shard.drain_pending_and_shutdown()?;
                Ok(false)
            }
            ShardDirective::Cancel => Err(RuntimeError::UnsupportedOperation {
                operation: "tick_shard_cancel",
            }),
            ShardDirective::Barrier => Err(RuntimeError::UnsupportedOperation {
                operation: "tick_shard_barrier",
            }),
        }
    }

    /// Migrates all commands from source shard to target shard.
    ///
    /// Returns `Ok(true)` if the source shard is still alive (has runs or pending commands),
    /// `Ok(false)` if the source shard is empty and can be shut down.
    fn migrate_shard(&mut self, source_idx: usize, target: u32) -> RuntimeResult<bool> {
        let target_usize =
            usize::try_from(target).map_err(|_| RuntimeError::ShardNotFound { shard: target })?;

        // Self-migrate check (already validated in caller, but double-check for safety)
        let source_u32 =
            u32::try_from(source_idx).map_err(|_| RuntimeError::ShardNotFound { shard: target })?;
        if target == source_u32 {
            return Err(RuntimeError::MigrateSelf);
        }

        // Validate target shard exists (already validated in caller, but double-check)
        if self.shards.get(target_usize).is_none() {
            return Err(RuntimeError::ShardNotFound { shard: target });
        }

        // Collect all commands from source shard
        let commands: Vec<ShardCommand> = {
            let shard = self
                .shards
                .get_mut(source_idx)
                .ok_or(RuntimeError::ShardNotFound { shard: source_u32 })?;
            let mut cmds = Vec::new();
            while let Some(cmd) = shard.command_queue.pop() {
                cmds.push(cmd);
            }
            cmds
        };

        // Push all commands to target shard
        {
            let target_shard = self
                .shards
                .get_mut(target_usize)
                .ok_or(RuntimeError::ShardNotFound { shard: target })?;
            for cmd in commands {
                target_shard.enqueue(cmd)?;
            }
        }

        // Return true if source shard still has runs (alive)
        let shard = self
            .shards
            .get_mut(source_idx)
            .ok_or(RuntimeError::ShardNotFound { shard: source_u32 })?;
        let alive = shard.active_run_count() > 0 || !shard.command_queue.is_empty();
        Ok(alive)
    }
}
