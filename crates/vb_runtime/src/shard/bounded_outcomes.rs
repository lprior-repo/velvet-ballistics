#![forbid(unsafe_code)]
//! Bounded map for terminal run outcomes (RQ-W0-10 companion to MEM-01).
//!
//! `BoundedOutcomeIndex` wraps an `IndexMap<RunId, TerminalOutcome>` with a
//! configurable capacity. Insertions past capacity drop the oldest entry by
//! insertion order (FIFO eviction), matching the semantics of the
//! `terminal_runs` LRU ring but without TTL sweeps. The companion
//! `terminal_runs` LRU ring handles TTL; outcomes piggy-back on the same
//! retention policy via the `Shard::terminal_outcome_record` integration.
//!
//! Invariants:
//! - `len() <= capacity` is upheld after every insert.
//! - `get(run)` is O(1) via the inner `IndexMap`.
//! - `remove(run)` is O(1) via the inner `IndexMap`.
//! - `force_record` overflows capacity (force-insert) for the rare legacy
//!   paths that explicitly opt into unbounded growth; the count of overflows
//!   is observable via `overflows()` for operator triage.

use indexmap::IndexMap;
use vb_core::ids::RunId;

use crate::shard::types::TerminalOutcome;

/// Default capacity for terminal outcome map when not configured.
pub const DEFAULT_MAX_TERMINAL_OUTCOMES: usize = 100_000;

/// Bounded insertion-ordered map keyed by `RunId` storing `TerminalOutcome`.
#[derive(Debug, Clone)]
pub struct BoundedOutcomeIndex {
    entries: IndexMap<RunId, TerminalOutcome>,
    capacity: usize,
    overflows: u64,
}

impl BoundedOutcomeIndex {
    /// Creates a bounded outcome index with the given capacity.
    ///
    /// `capacity == 0` is treated as `1` so the map remains usable for
    /// minimal configurations while still preventing unbounded growth.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let effective = capacity.max(1);
        Self {
            entries: IndexMap::with_capacity(effective),
            capacity: effective,
            overflows: 0,
        }
    }

    /// Returns the configured capacity.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the current number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the map holds no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns `true` if the map is at or above capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.entries.len() >= self.capacity
    }

    /// Returns the number of force-insert overflows observed since creation.
    #[must_use]
    pub const fn overflows(&self) -> u64 {
        self.overflows
    }

    /// Records an outcome, dropping the oldest entry if at capacity.
    ///
    /// Idempotent: a later call for the same `run_id` replaces the prior
    /// outcome without consuming additional capacity (the existing entry is
    /// updated in place).
    pub fn record(&mut self, run_id: RunId, outcome: TerminalOutcome) {
        if self.entries.contains_key(&run_id) {
            self.entries.insert(run_id, outcome);
            return;
        }
        if self.entries.len() >= self.capacity {
            self.entries.shift_remove_index(0);
        }
        self.entries.insert(run_id, outcome);
    }

    /// Force-inserts an outcome, growing past capacity if necessary.
    ///
    /// Increments the overflow counter when capacity is exceeded. Used by
    /// legacy paths that prefer unbounded growth over silent drop; the
    /// `overflows()` counter surfaces the deviation so operators can detect
    /// it.
    pub fn force_record(&mut self, run_id: RunId, outcome: TerminalOutcome) {
        if self.entries.contains_key(&run_id) {
            self.entries.insert(run_id, outcome);
            return;
        }
        self.entries.insert(run_id, outcome);
        if self.entries.len() > self.capacity {
            self.overflows = self.overflows.saturating_add(1);
        }
    }

    /// Returns the recorded outcome for `run_id`, if any.
    #[must_use]
    pub fn get(&self, run_id: RunId) -> Option<TerminalOutcome> {
        self.entries.get(&run_id).copied()
    }

    /// Removes the entry for `run_id`, if present.
    ///
    /// Returns `true` if an entry was removed.
    pub fn remove(&mut self, run_id: RunId) -> bool {
        self.entries.shift_remove(&run_id).is_some()
    }

    /// Removes every entry from the map without changing capacity.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for BoundedOutcomeIndex {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_MAX_TERMINAL_OUTCOMES)
    }
}

#[cfg(test)]
#[path = "bounded_outcomes_tests.rs"]
mod tests;
