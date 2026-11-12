/// Registry for RAII-based introspection handles.
///
/// Provides epoch-based registration with automatic cleanup on guard drop.
/// Does NOT create global mutable run state - each registry instance is independent.
#[derive(Default)]
pub struct IntrospectionRegistry {
    inner: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<RunId, u64>>>,
    next_epoch: u64,
}

impl IntrospectionRegistry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Acquires the registry's inner mutex, recovering from any prior
    /// poison state. The standard `Mutex::lock` returns `PoisonError`
    /// forever once a holder panics; this helper mirrors the recovery
    /// pattern already used by `action_queue` so the admission gate does
    /// not stay permanently disabled after a panic.
    fn lock_or_recover(
        inner: &std::sync::Arc<std::sync::Mutex<std::collections::HashMap<RunId, u64>>>,
    ) -> std::sync::MutexGuard<'_, std::collections::HashMap<RunId, u64>> {
        match inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Registers a handle for the given run.
    ///
    /// Returns the handle guard on success. Recovers transparently from
    /// a poisoned mutex so a single panicking holder cannot permanently
    /// disable the admission gate (RA-014).
    pub fn register(&mut self, run: RunId) -> RuntimeResult<InspectHandle> {
        let mut guard = Self::lock_or_recover(&self.inner);

        // Check if already registered
        if guard.contains_key(&run) {
            return Err(crate::RuntimeError::RunAlreadyExists);
        }

        let epoch = self.next_epoch;
        self.next_epoch = self
            .next_epoch
            .checked_add(1)
            .ok_or(crate::RuntimeError::IntrospectionEpochExhausted)?;
        guard.insert(run, epoch);

        Ok(InspectHandle {
            run,
            epoch,
            registry: self.inner.clone(),
        })
    }

    /// Registers a handle for the given run, allowing epoch replacement on conflict.
    ///
    /// Returns outcome indicating whether registration succeeded, conflicted, or was replaced.
    pub fn register_with_overlap_policy(
        &mut self,
        run: RunId,
    ) -> RuntimeResult<(InspectHandle, Result<(), RegisterOverlapOutcome>)> {
        let mut guard = Self::lock_or_recover(&self.inner);

        let (outcome, epoch) = if let Some(&old_epoch) = guard.get(&run) {
            // Overlap detected - replace with new epoch
            let new_epoch = self.next_epoch;
            self.next_epoch = self
                .next_epoch
                .checked_add(1)
                .ok_or(crate::RuntimeError::IntrospectionEpochExhausted)?;
            guard.insert(run, new_epoch);
            (
                Err(RegisterOverlapOutcome::Replaced {
                    old_epoch,
                    new_epoch,
                }),
                new_epoch,
            )
        } else {
            // No overlap - insert with new epoch
            let epoch = self.next_epoch;
            self.next_epoch = self
                .next_epoch
                .checked_add(1)
                .ok_or(crate::RuntimeError::IntrospectionEpochExhausted)?;
            guard.insert(run, epoch);
            (Ok(()), epoch)
        };

        Ok((
            InspectHandle {
                run,
                epoch,
                registry: self.inner.clone(),
            },
            outcome,
        ))
    }

    /// Unregisters a handle for the given run.
    ///
    /// Returns whether the handle was found and unregistered.
    pub fn unregister(&mut self, run: RunId) -> RuntimeResult<UnregisterOutcome> {
        let mut guard = Self::lock_or_recover(&self.inner);

        if guard.remove(&run).is_some() {
            Ok(UnregisterOutcome::Unregistered)
        } else {
            Ok(UnregisterOutcome::Missing)
        }
    }

    /// Unregisters all handles.
    ///
    /// Returns the count of handles removed.
    pub fn unregister_all(&mut self) -> RuntimeResult<usize> {
        let mut guard = Self::lock_or_recover(&self.inner);
        let count = guard.len();
        guard.clear();
        Ok(count)
    }

    /// Returns whether a run is currently visible to introspection.
    #[must_use]
    pub fn is_visible(&self, run: RunId) -> bool {
        match self.inner.lock() {
            Ok(guard) => guard.contains_key(&run),
            Err(poisoned) => poisoned.into_inner().contains_key(&run),
        }
    }
}

#[cfg(test)]
impl IntrospectionRegistry {
    /// Returns a clone of the inner `Arc<Mutex<...>>` for tests that need
    /// to poison the registry from another thread. Production code MUST
    /// NOT use this — it is compiled out of release builds.
    pub(crate) fn inner_arc_for_test(
        &self,
    ) -> std::sync::Arc<std::sync::Mutex<std::collections::HashMap<RunId, u64>>> {
        self.inner.clone()
    }
}

/// Snapshot formatting stays cold path (no computation on hot path).
pub struct InspectSnapshotFormatter;

impl InspectSnapshotFormatter {
    /// Formats a snapshot response into a string representation.
    ///
    /// This is a cold-path operation - called only when formatting output,
    /// not during the hot path of inspect operations.
    #[must_use]
    pub fn format_snapshot(response: &InspectResponse) -> String {
        match response {
            InspectResponse::Found(snap) => {
                format!(
                    "InspectSnapshot {{ run: {:?}, correlation: {}, pc: {:?}, executed: {} }}",
                    snap.run, snap.correlation, snap.pc, snap.executed
                )
            }
            InspectResponse::NotFound { run, correlation } => {
                format!(
                    "NotFound {{ run: {:?}, correlation: {} }}",
                    run, correlation
                )
            }
        }
    }
}
