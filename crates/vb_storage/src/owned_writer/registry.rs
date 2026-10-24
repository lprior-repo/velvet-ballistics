use super::table::FixedTable;
use super::types::{Capacity, Generations, Key};
use super::{OwnedWriterError as Error, OwnedWriterQuarantine as Reason};
use std::sync::{Mutex, MutexGuard, OnceLock, TryLockError};

/// Exactly one instance is initialized inside the actual FjallJournal aggregate.
#[derive(Default)]
pub(crate) struct OwnedWriterRegistry {
    gate: Mutex<Generations>,
    table: OnceLock<FixedTable>,
}

impl OwnedWriterRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> Result<MutexGuard<'_, Generations>, Error> {
        classify_lock(self.gate.try_lock())
    }

    pub(super) fn acquire(&self, run: vb_core::RunId, capacity: Capacity) -> Result<Key, Error> {
        let mut generations = self.lock()?;
        generations.ensure_available()?;
        // This gate is the only initializer. OnceLock::get never waits, and set
        // cannot encounter another initializer while this gate is held.
        if self.table.get().is_none() {
            let table = FixedTable::new(capacity)?;
            self.table
                .set(table)
                .map_err(|_| Error::RegistryInvariant)?;
        }
        let table = self.table.get().ok_or(Error::RegistryInvariant)?;
        table.matching_capacity(capacity)?;
        let index = table.vacant(run)?;
        let key = Key {
            run,
            index,
            generation: generations.issue()?,
        };
        table.claim(key)?;
        Ok(key)
    }

    fn entry(&self, key: Key) -> Result<&super::entry::Entry, Error> {
        self.table
            .get()
            .ok_or(Error::StaleOwnership { run: key.run })?
            .entry(key)
    }

    pub(super) fn validate(&self, key: Key) -> Result<(), Error> {
        let generations = self.lock()?;
        generations.ensure_available()?;
        if let Some(reason) = self.entry(key)?.reason()? {
            return Err(Error::Quarantined {
                run: key.run,
                reason,
            });
        }
        Ok(())
    }

    pub(super) fn quarantine(&self, key: Key, reason: Reason) -> Result<Reason, Error> {
        self.entry(key)?.quarantine(key, reason)
    }

    pub(super) fn release(&self, key: Key) -> Result<(), Error> {
        let _guard = self.lock()?;
        self.entry(key)?.release(key)
    }

    /// A failed nonblocking Drop release leaves the original table entry intact.
    pub(super) fn release_on_drop(&self, key: Key) {
        match self.release(key) {
            Ok(()) => {}
            Err(Error::Contended | Error::Poisoned | Error::Quarantined { .. }) => {
                // Retain ownership/uncertainty until full store teardown.
            }
            Err(Error::StaleOwnership { .. } | Error::WrongStore) => {
                // A failed identity check never changes the replacement owner.
            }
            Err(
                Error::InvalidCapacity { .. }
                | Error::CapacityMismatch { .. }
                | Error::AllocationRefused
                | Error::Full { .. }
                | Error::Busy { .. }
                | Error::GenerationExhausted
                | Error::RegistryInvariant,
            ) => {
                // No other refusal can make the checked release publish a free slot.
            }
        }
    }
}

#[cfg(test)]
#[path = "registry/tests.rs"]
mod tests;

fn classify_lock(
    result: std::sync::TryLockResult<MutexGuard<'_, Generations>>,
) -> Result<MutexGuard<'_, Generations>, Error> {
    match result {
        Ok(guard) => Ok(guard),
        Err(TryLockError::WouldBlock) => Err(Error::Contended),
        Err(TryLockError::Poisoned(_)) => Err(Error::Poisoned),
    }
}
