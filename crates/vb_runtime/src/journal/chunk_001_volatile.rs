/// In-memory journal useful for tests and volatile embeddings.
#[derive(Debug)]
pub struct VolatileRuntimeJournal {
    events: Mutex<Vec<RuntimeJournalEvent>>,
    capacity: usize,
}

impl VolatileRuntimeJournal {
    /// Default maximum number of in-memory journal events retained by a volatile journal.
    pub const DEFAULT_CAPACITY: usize = 65_536;

    /// Creates an empty volatile journal.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            capacity: Self::DEFAULT_CAPACITY,
        }
    }

    /// Creates an empty volatile journal with an explicit event capacity.
    #[must_use]
    pub const fn with_capacity(capacity: NonZeroUsize) -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            capacity: capacity.get(),
        }
    }

    /// Creates a shared volatile journal.
    #[must_use]
    pub fn shared() -> SharedRuntimeJournal {
        Arc::new(Self::new())
    }

    /// Returns a point-in-time copy of appended events.
    pub fn snapshot(&self) -> RuntimeResult<Vec<RuntimeJournalEvent>> {
        let events = self
            .events
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?;
        Ok(events.clone())
    }

    fn reserve_one_event(
        events: &mut Vec<RuntimeJournalEvent>,
        capacity: usize,
    ) -> RuntimeResult<()> {
        events
            .try_reserve(1)
            .map_err(|_| crate::RuntimeError::JournalFull { capacity })
    }
}

impl Default for VolatileRuntimeJournal {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeJournal for VolatileRuntimeJournal {
    fn append(&self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        let mut events = self
            .events
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?;
        if events.len() >= self.capacity {
            return Err(RuntimeError::JournalFull {
                capacity: self.capacity,
            });
        }
        Self::reserve_one_event(&mut events, self.capacity)?;
        events.push(event);
        Ok(())
    }
    fn probe(&self) -> RuntimeResult<()> {
        // Verify the mutex is not poisoned.
        let _guard = self
            .events
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?;
        Ok(())
    }
}
