use crate::shard::types::rollback_state::MigrationRollback;

impl ShardCommandQueue {
    pub(crate) fn rollback_migration_pair(&mut self, source: &mut Self) -> RuntimeResult<()> {
        if !self.migration.fenced || !source.migration.fenced || !source.migration.fully_staged {
            return Err(migration_invariant("migration_rollback_without_fence"));
        }
        self.prepare_migration_rollback()?;
        let bound = MAX_COMMAND_QUEUE_CAPACITY
            .saturating_mul(2)
            .saturating_add(3);
        for _ in 0..bound {
            let Some(rollback) = self.migration.rollback.as_mut() else {
                return Err(migration_invariant("migration_rollback_state_missing"));
            };
            if rollback.is_complete() {
                return self.finish_migration_pair_restoration(source);
            }
            let progress = rollback.step(
                &mut source.migration.stage,
                &mut self.migration.stage,
                |entry| source.inner.push(entry),
                |entry| self.inner.push(entry),
            );
            if progress == TransferProgress::Refused {
                return Err(migration_invariant("migration_rollback_refused"));
            }
        }
        self.finish_migration_pair_restoration(source)
    }

    fn prepare_migration_rollback(&mut self) -> RuntimeResult<()> {
        if self.migration.rollback.is_some() {
            return Ok(());
        }
        for _ in 0..self.inner.len() {
            if self.migration.stage.fill_one(|| self.inner.pop()) != TransferProgress::Accepted {
                return Err(migration_invariant("migration_rollback_staging_refused"));
            }
        }
        self.migration.rollback =
            MigrationRollback::prepare(&self.migration.stage, self.migration.original_len);
        if self.migration.rollback.is_none() {
            return Err(migration_invariant("migration_rollback_prefix_invalid"));
        }
        Ok(())
    }

    fn finish_migration_pair_restoration(&mut self, source: &mut Self) -> RuntimeResult<()> {
        let complete = self
            .migration
            .rollback
            .as_ref()
            .is_some_and(MigrationRollback::is_complete);
        if !complete
            || self.migration.stage.len() != 0
            || source.migration.stage.len() != 0
            || self.inner.len() != self.migration.original_len
            || source.inner.len() != source.migration.original_len
        {
            return Err(migration_invariant("migration_rollback_incomplete"));
        }
        self.migration.rollback = None;
        self.migration.planned_source_len = None;
        self.migration.fenced = false;
        source.migration.fenced = false;
        Ok(())
    }
}
