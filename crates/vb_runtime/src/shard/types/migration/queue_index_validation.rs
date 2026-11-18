impl ShardCommandQueue {
    fn validate_migration_indices(&self) -> RuntimeResult<()> {
        let pending = Self::lock_or_recover(&self.pending);
        let depth = Self::lock_or_recover(&self.admitted_depth);
        let submissions = Self::lock_or_recover(&self.pending_submissions);
        if pending.len() > self.capacity
            || depth.len() > pending.len()
            || submissions.len() > pending.len()
        {
            return Err(migration_invariant("migration_index_cardinality"));
        }
        validate_migration_pending_total(&pending, self.len())?;
        for run in depth.keys() {
            if !pending.contains_key(run) {
                return Err(migration_invariant("migration_depth_without_pending"));
            }
        }
        for (run, count) in submissions.iter() {
            if *count == 0 || pending.get(run).is_none_or(|pending| count > pending) {
                return Err(migration_invariant("migration_submissions_without_pending"));
            }
        }
        Ok(())
    }
}

fn validate_migration_pending_total(
    pending: &IndexMap<RunId, u32>,
    queue_len: usize,
) -> RuntimeResult<()> {
    let mut total = 0usize;
    for count in pending.values() {
        if *count == 0 {
            return Err(migration_invariant("migration_pending_zero_count"));
        }
        let count = usize::try_from(*count)
            .map_err(|_| migration_invariant("migration_pending_count_overflow"))?;
        total = total
            .checked_add(count)
            .ok_or_else(|| migration_invariant("migration_pending_count_overflow"))?;
        if total > queue_len {
            return Err(migration_invariant("migration_pending_exceeds_queue"));
        }
    }
    Ok(())
}
