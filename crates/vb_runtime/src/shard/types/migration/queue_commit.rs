impl ShardCommandQueue {
    pub(crate) fn transfer_staged_migration(&mut self, source: &mut Self) -> RuntimeResult<()> {
        self.check_migration_commit_start(source)?;
        for _ in 0..source.migration.original_len {
            match source
                .migration
                .stage
                .transfer_front(QueuePush(&self.inner))
            {
                TransferProgress::Accepted => {}
                _ => return Err(migration_invariant("migration_transfer_refused")),
            }
        }
        self.commit_migration_indices(source)
    }

    fn check_migration_commit_start(&self, source: &Self) -> RuntimeResult<()> {
        if !self.migration.fenced
            || !source.migration.fenced
            || !source.migration.fully_staged
            || self.migration.planned_source_len != Some(source.migration.original_len)
            || self.inner.len() != self.migration.original_len
            || !source.inner.is_empty()
            || source.migration.stage.len() != source.migration.original_len
        {
            return Err(migration_invariant("migration_commit_invariant"));
        }
        check_migration_queue_capacity(
            self.inner.len(),
            source.migration.original_len,
            self.capacity,
        )
    }

    fn commit_migration_indices(&mut self, source: &mut Self) -> RuntimeResult<()> {
        let expected = self
            .migration
            .original_len
            .checked_add(source.migration.original_len)
            .ok_or_else(|| migration_invariant("migration_commit_length_overflow"))?;
        if source.migration.stage.len() != 0 || self.inner.len() != expected {
            return Err(migration_invariant("migration_commit_length_mismatch"));
        }
        self.install_migration_indices(source);
        Ok(())
    }

    fn install_migration_indices(&mut self, source: &mut Self) {
        core::mem::swap(
            &mut *Self::lock_or_recover(&self.pending),
            &mut self.migration.planned_pending,
        );
        core::mem::swap(
            &mut *Self::lock_or_recover(&self.admitted_depth),
            &mut self.migration.planned_depth,
        );
        core::mem::swap(
            &mut *Self::lock_or_recover(&self.pending_submissions),
            &mut self.migration.planned_submissions,
        );
        Self::lock_or_recover(&source.pending).clear();
        Self::lock_or_recover(&source.admitted_depth).clear();
        Self::lock_or_recover(&source.pending_submissions).clear();
        self.migration.fenced = false;
        source.migration.fenced = false;
        self.migration.planned_source_len = None;
    }
}
