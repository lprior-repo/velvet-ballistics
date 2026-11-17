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
    /// - `Migrate { target }`: Temporarily rejects a nonempty source queue with
    ///   `UnsupportedOperation`; empty-source migration keeps its existing result.
    ///   Pending-command transfer requires reservation-preserving custody first.
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
        tick_validated_shard(self, shard_index_usize, shard_index, directive)
    }

    /// Validates migration and temporarily refuses nonempty source queues.
    ///
    /// This containment does not transfer queued commands. Source and target
    /// remain unchanged on refusal, including queued quota reservations.
    ///
    /// Returns `Ok(true)` if the source shard is still alive (has runs or pending commands),
    /// `Ok(false)` if the source shard is empty and can be shut down.
    fn migrate_shard(&mut self, source_idx: usize, target: u32) -> RuntimeResult<bool> {
        let target_usize =
            usize::try_from(target).map_err(|_| RuntimeError::ShardNotFound { shard: target })?;
        let source_u32 =
            u32::try_from(source_idx).map_err(|_| RuntimeError::ShardNotFound { shard: target })?;
        if target == source_u32 {
            return Err(RuntimeError::MigrateSelf);
        }
        if self.shards.get(target_usize).is_none() {
            return Err(RuntimeError::ShardNotFound { shard: target });
        }
        let commands = self.drain_migration_commands(source_idx, source_u32)?;
        self.enqueue_migrated_commands(target_usize, target, commands)?;
        let shard = self
            .shards
            .get_mut(source_idx)
            .ok_or(RuntimeError::ShardNotFound { shard: source_u32 })?;
        Ok(shard.active_run_count() > 0 || !shard.command_queue.is_empty())
    }

    fn drain_migration_commands(
        &mut self,
        source_idx: usize,
        source_id: u32,
    ) -> RuntimeResult<Vec<ShardCommand>> {
        let shard = self
            .shards
            .get_mut(source_idx)
            .ok_or(RuntimeError::ShardNotFound { shard: source_id })?;
        // Temporary containment: popping loses queued reservation custody,
        // and a later target enqueue failure cannot restore drained commands.
        if !shard.command_queue.is_empty() {
            return Err(RuntimeError::UnsupportedOperation {
                operation: "migrate_nonempty_command_queue",
            });
        }
        Ok(Vec::new())
    }

    fn enqueue_migrated_commands(
        &mut self,
        target_index: usize,
        target_id: u32,
        commands: Vec<ShardCommand>,
    ) -> RuntimeResult<()> {
        let target = self
            .shards
            .get_mut(target_index)
            .ok_or(RuntimeError::ShardNotFound { shard: target_id })?;
        for command in commands {
            target.enqueue(command)?;
        }
        Ok(())
    }
}

fn tick_validated_shard(
    runtime: &mut Runtime,
    shard_index: usize,
    shard_id: u32,
    directive: ShardDirective,
) -> RuntimeResult<bool> {
    validate_tick_target(runtime, shard_index, shard_id)?;
    apply_shard_directive(runtime, shard_index, shard_id, directive)
}

fn validate_tick_target(runtime: &Runtime, shard_index: usize, shard_id: u32) -> RuntimeResult<()> {
    if runtime.shards.get(shard_index).is_some() {
        Ok(())
    } else {
        Err(RuntimeError::ShardNotFound { shard: shard_id })
    }
}

fn apply_shard_directive(
    runtime: &mut Runtime,
    shard_index: usize,
    shard_id: u32,
    directive: ShardDirective,
) -> RuntimeResult<bool> {
    match directive {
        ShardDirective::Continue => tick_shard_continue(runtime, shard_index, shard_id),
        ShardDirective::Suspend => {
            // Suspend: skip processing, preserve queue, return alive
            Ok(true)
        }
        ShardDirective::Migrate { target } => runtime.migrate_shard(shard_index, target),
        ShardDirective::Shutdown => tick_shard_shutdown(runtime, shard_index, shard_id),
        ShardDirective::Cancel => Err(RuntimeError::UnsupportedOperation {
            operation: "tick_shard_cancel",
        }),
        ShardDirective::Barrier => Err(RuntimeError::UnsupportedOperation {
            operation: "tick_shard_barrier",
        }),
    }
}

fn tick_shard_continue(
    runtime: &mut Runtime,
    shard_index: usize,
    shard_id: u32,
) -> RuntimeResult<bool> {
    runtime
        .shards
        .get_mut(shard_index)
        .ok_or(RuntimeError::ShardNotFound { shard: shard_id })?
        .tick()
}

fn tick_shard_shutdown(
    runtime: &mut Runtime,
    shard_index: usize,
    shard_id: u32,
) -> RuntimeResult<bool> {
    let shard = runtime
        .shards
        .get_mut(shard_index)
        .ok_or(RuntimeError::ShardNotFound { shard: shard_id })?;
    shard.drain_pending_and_shutdown()?;
    Ok(false)
}
