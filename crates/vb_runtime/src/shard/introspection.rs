#![forbid(unsafe_code)]
//! Introspection types for run inspection and registry management.

use std::sync::{Arc, Mutex};

use vb_core::ids::RunId;

use crate::RuntimeResult;
use super::run_state::InspectResponse;

// ============================================================================
// Introspection Registry Outcome types
// ============================================================================

/// Outcome of an unregister operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnregisterOutcome {
    /// The handle was successfully unregistered.
    Unregistered,
    /// The handle was not found (no-op).
    Missing,
}

/// Outcome of a register operation when overlap is detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterOverlapOutcome {
    /// Registration was rejected due to conflict with existing registration.
    Conflict,
    /// Registration replaced the existing one with a new epoch.
    Replaced {
        /// The epoch of the replaced registration.
        old_epoch: u64,
        /// The epoch of the new registration.
        new_epoch: u64,
    },
}

// ============================================================================
// InspectHandle and IntrospectionRegistry
// ============================================================================

/// Epoch-based handle for an introspection registration.
///
/// When dropped, the handle is automatically unregistered from the registry.
#[derive(Debug)]
pub struct InspectHandle {
    run: RunId,
    epoch: u64,
    registry: Arc<Mutex<std::collections::HashMap<RunId, u64>>>,
}

impl InspectHandle {
    /// Returns the run identifier associated with this handle.
    #[must_use]
    pub fn run(&self) -> RunId {
        self.run
    }

    /// Returns the epoch of this handle.
    #[must_use]
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

impl Drop for InspectHandle {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.registry.lock() {
            // Only remove if the epoch matches (handles stale drops correctly)
            if let Some(current_epoch) = guard.get(&self.run)
                && *current_epoch == self.epoch
            {
                guard.remove(&self.run);
            }
        }
    }
}

/// Registry for RAII-based introspection handles.
///
/// Provides epoch-based registration with automatic cleanup on guard drop.
/// Does NOT create global mutable run state - each registry instance is independent.
#[derive(Default)]
pub struct IntrospectionRegistry {
    inner: Arc<Mutex<std::collections::HashMap<RunId, u64>>>,
    next_epoch: u64,
}

impl IntrospectionRegistry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a handle for the given run.
    ///
    /// Returns the handle guard on success.
    pub fn register(&mut self, run: RunId) -> RuntimeResult<InspectHandle> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?;

        // Check if already registered
        if guard.contains_key(&run) {
            return Err(crate::RuntimeError::RunAlreadyExists);
        }

        let epoch = self.next_epoch;
        self.next_epoch = self.next_epoch.saturating_add(1);
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
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?;

        let (outcome, epoch) = if let Some(&old_epoch) = guard.get(&run) {
            // Overlap detected - replace with new epoch
            let new_epoch = self.next_epoch;
            self.next_epoch = self.next_epoch.saturating_add(1);
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
            self.next_epoch = self.next_epoch.saturating_add(1);
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
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?;

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
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| crate::RuntimeError::JournalPoisoned)?;
        let count = guard.len();
        guard.clear();
        Ok(count)
    }

    /// Returns whether a run is currently visible to introspection.
    #[must_use]
    pub fn is_visible(&self, run: RunId) -> bool {
        if let Ok(guard) = self.inner.lock() {
            guard.contains_key(&run)
        } else {
            false
        }
    }
}

// ============================================================================
// InspectSnapshotFormatter
// ============================================================================

/// Snapshot formatting stays cold path (no computation on hot path).
pub struct InspectSnapshotFormatter;

impl InspectSnapshotFormatter {
    /// Formats a snapshot response into a string representation.
    ///
    /// This is a cold-path operation - called only when formatting output,
    /// not during the hot path of inspect operations.
    #[must_use]
    pub fn format_snapshot(run: RunId, response: &InspectResponse) -> String {
        match response {
            InspectResponse::Found(snap) => {
                format!(
                    "InspectSnapshot {{ run: {:?}, correlation: {}, pc: {:?}, executed: {} }}",
                    run, snap.correlation, snap.pc, snap.executed
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
