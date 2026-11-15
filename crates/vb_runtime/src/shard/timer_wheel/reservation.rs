use super::{Instant, RunId, TimerEntry, TimerWheel};
use crate::{RuntimeError, RuntimeResult};

impl TimerWheel {
    pub(crate) fn reserve_run_slot(&mut self, run: RunId, capacity: usize) -> RuntimeResult<()> {
        if self.by_run.contains_key(&run) {
            return Ok(());
        }
        if self.by_run.len() >= capacity {
            return Err(RuntimeError::ActiveRunCapacityExceeded { capacity });
        }
        self.by_run
            .try_reserve(1)
            .map_err(|_| RuntimeError::ActiveRunCapacityExceeded { capacity })
    }

    pub(crate) fn prepare_entry(
        &mut self,
        run: RunId,
        deadline: Instant,
        capacity: usize,
    ) -> RuntimeResult<()> {
        self.reserve_run_slot(run, capacity)?;
        if let Some(entries) = self.by_deadline.get_mut(&deadline) {
            if entries.iter().any(|entry| entry.run == run) {
                return Ok(());
            }
            return entries
                .try_reserve(1)
                .map_err(|_| RuntimeError::ActiveRunCapacityExceeded { capacity });
        }
        self.prepare_new_bucket(deadline, capacity)
    }

    fn prepare_new_bucket(&mut self, deadline: Instant, capacity: usize) -> RuntimeResult<()> {
        let mut entries = Vec::new();
        entries
            .try_reserve(1)
            .map_err(|_| RuntimeError::ActiveRunCapacityExceeded { capacity })?;
        self.discard_prepared_bucket();
        // Stage the tree node before durable scheduling. Only the vector's
        // recoverable reservation can fail; std tree-node allocation is infallible.
        self.by_deadline.insert(deadline, entries);
        self.prepared_bucket = Some(deadline);
        Ok(())
    }

    fn discard_prepared_bucket(&mut self) {
        if let Some(deadline) = self.prepared_bucket.take()
            && self.by_deadline.get(&deadline).is_some_and(Vec::is_empty)
        {
            self.by_deadline.remove(&deadline);
        }
    }

    pub(crate) fn insert_exact_generation(
        &mut self,
        entry: TimerEntry,
        capacity: usize,
    ) -> RuntimeResult<()> {
        self.prepare_entry(entry.run, entry.deadline, capacity)?;
        let entries = self
            .by_deadline
            .get_mut(&entry.deadline)
            .ok_or(RuntimeError::InvalidTimerFire)?;
        entries.retain(|current| current.run != entry.run);
        entries.push(entry);
        if self
            .by_run
            .get(&entry.run)
            .is_some_and(|old| old.deadline != entry.deadline)
        {
            self.cancel(entry.run);
        }
        self.by_run.insert(entry.run, entry);
        Ok(())
    }

    pub(crate) fn due_peek(&self, now: Instant) -> Option<TimerEntry> {
        self.by_deadline.range(..=now).find_map(|(_, entries)| {
            entries
                .iter()
                .copied()
                .filter(|entry| self.by_run.get(&entry.run).copied() == Some(*entry))
                .min_by_key(|entry| entry.run.get())
        })
    }
}
