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

/// Runtime journal selection using explicit storage durability profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeJournalConfig {
    profile: DurabilityProfile,
}

impl RuntimeJournalConfig {
    /// Creates a runtime journal configuration for a durability profile.
    #[must_use]
    pub const fn new(profile: DurabilityProfile) -> Self {
        Self { profile }
    }

    /// Selected durability profile.
    #[must_use]
    pub const fn profile(self) -> DurabilityProfile {
        self.profile
    }

    /// Builds a shared runtime journal for this profile.
    ///
    /// Returns `Err(RuntimeError::UnsupportedDurabilityProfile { .. })`
    /// for any future `DurabilityProfile` variant the runtime does not
    /// yet implement. This replaces the prior silent downgrade to
    /// `Volatile` (master §45 contract: missing transitions return a
    /// typed error rather than silently absorbing into the
    /// least-durable profile).
    pub fn shared_journal(
        self,
        journal: Arc<FjallJournal>,
        queue: Arc<JournalWriterQueue>,
    ) -> RuntimeResult<SharedRuntimeJournal> {
        match self.profile {
            DurabilityProfile::Volatile => Ok(VolatileRuntimeJournal::shared()),
            DurabilityProfile::Journaled => Ok(QueuedStorageRuntimeJournal::shared_journaled(
                journal, queue,
            )),
            DurabilityProfile::Strict => Ok(StorageRuntimeJournal::shared_strict(journal)),
            _ => Err(RuntimeError::UnsupportedDurabilityProfile {
                profile_debug: format!("{:?}", self.profile),
            }),
        }
    }

    //    /// Compile-time exhaustiveness assertion for `shared_journal`.
    //    #[allow(dead_code, unreachable_patterns)]
    //    const _: () = {
    //        fn _check_exhaustiveness(
    //            profile: DurabilityProfile,
    //            journal: std::sync::Arc<FjallJournal>,
    //            queue: std::sync::Arc<JournalWriterQueue>,
    //        ) -> RuntimeResult<SharedRuntimeJournal> {
    //            match profile {
    //                DurabilityProfile::Volatile => Ok(VolatileRuntimeJournal::shared()),
    //                DurabilityProfile::Journaled => Ok(QueuedStorageRuntimeJournal::shared_journaled(
    //                    journal, queue,
    //                )),
    //                DurabilityProfile::Strict => Ok(StorageRuntimeJournal::shared_strict(journal)),
    //                _ => Err(RuntimeError::UnsupportedDurabilityProfile {
    //                    profile_debug: String::new(),
    //                }),
    //            }
    //        }
    //// };
    //}
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

    fn append_sequenced_batch(
        &self,
        events: &[RuntimeJournalEvent],
        _start_seq: EventSeq,
    ) -> RuntimeResult<()> {
        let mut stored = self
            .events
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?;
        let available =
            self.capacity
                .checked_sub(stored.len())
                .ok_or(RuntimeError::JournalFull {
                    capacity: self.capacity,
                })?;
        if events.len() > available {
            return Err(RuntimeError::JournalFull {
                capacity: self.capacity,
            });
        }
        stored
            .try_reserve(events.len())
            .map_err(|_| RuntimeError::JournalFull {
                capacity: self.capacity,
            })?;
        stored.extend(events.iter().cloned());
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

/// Runtime journal adapter that appends lifecycle events into `vb_storage`.
pub struct StorageRuntimeJournal {
    catalog: Option<Arc<vb_storage::executor::ExecutorCatalog>>,
    journal: Arc<FjallJournal>,
    profile: DurabilityProfile,
}
