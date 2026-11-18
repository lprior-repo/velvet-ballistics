use crate::shard::types::index_plan::{
    MigrationRunCounts, check_migration_queue_capacity, plan_migration_run_counts,
};

impl ShardCommandQueue {
    pub(crate) fn prepare_migration_target(&mut self, source: &Self) -> RuntimeResult<()> {
        check_migration_queue_capacity(
            self.inner.len(),
            source.migration.original_len,
            self.capacity,
        )?;
        if !source.migration.fenced || !source.migration.fully_staged || !source.inner.is_empty() {
            return Err(migration_invariant("migration_source_not_staged"));
        }
        self.validate_migration_indices()?;
        source.validate_migration_indices()?;
        self.fence_migration()?;
        self.copy_migration_target_indices();
        self.merge_migration_source_indices(source)?;
        self.migration.planned_source_len = Some(source.migration.original_len);
        Ok(())
    }

    fn copy_migration_target_indices(&mut self) {
        self.migration.planned_pending.clear();
        self.migration.planned_depth.clear();
        self.migration.planned_submissions.clear();
        for (&run, &count) in Self::lock_or_recover(&self.pending).iter() {
            let _previous = self.migration.planned_pending.insert(run, count);
        }
        for (&run, &depth) in Self::lock_or_recover(&self.admitted_depth).iter() {
            let _previous = self.migration.planned_depth.insert(run, depth);
        }
        for (&run, &count) in Self::lock_or_recover(&self.pending_submissions).iter() {
            let _previous = self.migration.planned_submissions.insert(run, count);
        }
    }

    fn merge_migration_source_indices(&mut self, source: &Self) -> RuntimeResult<()> {
        let pending = Self::lock_or_recover(&source.pending);
        let depth = Self::lock_or_recover(&source.admitted_depth);
        let submissions = Self::lock_or_recover(&source.pending_submissions);
        for (&run, &count) in pending.iter() {
            let source_counts = MigrationRunCounts {
                pending: count,
                submissions: submissions.get(&run).copied().unwrap_or_default(),
                retained_depth: depth.get(&run).copied(),
            };
            let planned =
                plan_migration_run_counts(run, self.planned_run_counts(run), source_counts, None)?;
            self.set_planned_run_counts(run, planned);
        }
        Ok(())
    }

    fn planned_run_counts(&self, run: RunId) -> MigrationRunCounts {
        MigrationRunCounts {
            pending: self
                .migration
                .planned_pending
                .get(&run)
                .copied()
                .unwrap_or_default(),
            submissions: self
                .migration
                .planned_submissions
                .get(&run)
                .copied()
                .unwrap_or_default(),
            retained_depth: self.migration.planned_depth.get(&run).copied(),
        }
    }

    fn set_planned_run_counts(&mut self, run: RunId, counts: MigrationRunCounts) {
        let _previous = self.migration.planned_pending.insert(run, counts.pending);
        if counts.submissions > 0 {
            let _previous = self
                .migration
                .planned_submissions
                .insert(run, counts.submissions);
        }
        if let Some(depth) = counts.retained_depth {
            replace_or_insert_planned_depth(&mut self.migration.planned_depth, run, depth);
        }
    }

    pub(crate) fn restrict_migration_depth(
        &mut self,
        run: RunId,
        depth: Option<u32>,
    ) -> RuntimeResult<()> {
        if !self.migration.fenced || !self.migration.planned_pending.contains_key(&run) {
            return Err(migration_invariant("migration_depth_unknown_run"));
        }
        let Some(limit) = [depth, self.migration.planned_depth.get(&run).copied()]
            .into_iter()
            .flatten()
            .min()
        else {
            return Ok(());
        };
        self.apply_migration_depth(run, limit)
    }

    fn apply_migration_depth(&mut self, run: RunId, limit: u32) -> RuntimeResult<()> {
        let pending = self
            .migration
            .planned_pending
            .get(&run)
            .copied()
            .unwrap_or_default();
        if pending > limit {
            return Err(crate::RuntimeError::PerRunQueueDepthExceeded {
                run,
                pending: self.pending_for(run),
                limit,
            });
        }
        replace_or_insert_planned_depth(&mut self.migration.planned_depth, run, limit);
        Ok(())
    }
}

// Avoid HashTable::entry reserving before it discovers an occupied key.
fn replace_or_insert_planned_depth(depths: &mut IndexMap<RunId, u32>, run: RunId, depth: u32) {
    if let Some(recorded) = depths.get_mut(&run) {
        *recorded = depth;
    } else {
        let _previous = depths.insert(run, depth);
    }
}

#[cfg(test)]
include!("planned_depth_tests.rs");
