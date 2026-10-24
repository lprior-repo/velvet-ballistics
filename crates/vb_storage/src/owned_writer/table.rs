use super::OwnedWriterError as Error;
use super::entry::Entry;
use super::types::{Capacity, Key};

pub(super) struct FixedTable {
    entries: Vec<Entry>,
}

impl FixedTable {
    pub(super) fn new(capacity: Capacity) -> Result<Self, Error> {
        let entries = reserve_entries(capacity.get())?;
        Ok(Self { entries })
    }

    pub(super) fn capacity(&self) -> usize {
        self.entries.len()
    }

    pub(super) fn matching_capacity(&self, requested: Capacity) -> Result<(), Error> {
        if self.capacity() != requested.get() {
            return Err(Error::CapacityMismatch {
                bound: self.capacity(),
                requested: requested.get(),
            });
        }
        Ok(())
    }

    /// Scan every occupied slot before using a free hole, preventing duplicate runs.
    pub(super) fn vacant(&self, run: vb_core::RunId) -> Result<usize, Error> {
        let mut free = None;
        for (index, entry) in self.entries.iter().enumerate() {
            if entry.occupied() {
                if entry.run() == run {
                    return match entry.reason()? {
                        Some(reason) => Err(Error::Quarantined { run, reason }),
                        None => Err(Error::Busy { run }),
                    };
                }
            } else if free.is_none() {
                free = Some(index);
            }
        }
        free.ok_or(Error::Full {
            capacity: self.capacity(),
        })
    }

    pub(super) fn entry(&self, key: Key) -> Result<&Entry, Error> {
        let entry = self
            .entries
            .get(key.index)
            .ok_or(Error::StaleOwnership { run: key.run })?;
        entry.check(key)?;
        Ok(entry)
    }

    pub(super) fn claim(&self, key: Key) -> Result<(), Error> {
        let entry = self
            .entries
            .get(key.index)
            .ok_or(Error::RegistryInvariant)?;
        if entry.occupied() {
            return Err(Error::RegistryInvariant);
        }
        entry.claim(key);
        Ok(())
    }
}

/// First reservation is fallible; resizing cannot grow past its accepted capacity.
pub(super) fn reserve_entries(count: usize) -> Result<Vec<Entry>, Error> {
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(count)
        .map_err(|_| Error::AllocationRefused)?;
    entries.resize_with(count, Entry::empty);
    Ok(entries)
}
