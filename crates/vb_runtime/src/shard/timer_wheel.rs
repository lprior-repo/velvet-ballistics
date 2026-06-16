#![forbid(unsafe_code)]
//! Timer wheel for wait/ask deadline tracking.
//!
//! Uses `BTreeMap<Instant, Vec<TimerEntry>>` as the primary time-index
//! and `HashMap<RunId, TimerEntry>` as the run-index.
//! This gives O(log n) insert/cancel and O(k) fire where k is expired timers.

#[cfg(kani)]
use std::collections::BTreeMap as Map;
use std::collections::BTreeMap;
#[cfg(not(kani))]
use std::collections::HashMap as Map;
use std::time::Instant;

use vb_core::ids::RunId;

use super::types::PendingTimerKind;

/// A single timer entry keyed by its deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerEntry {
    /// The run this timer belongs to.
    pub run: RunId,
    /// Freshness token incremented on replacement.
    pub generation: u64,
    /// The deadline that keyed this entry.
    pub deadline: Instant,
    /// The kind of timer (Wait or Ask).
    pub kind: PendingTimerKind,
}

/// Timer wheel mutation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerWheelError {
    /// Replacing this run's timer would overflow the freshness generation.
    GenerationExhausted,
    /// The timer wheel has reached its maximum capacity.
    CapacityExceeded,
}

/// Dual-index timer data structure for O(log n) operations.
#[derive(Debug)]
pub struct TimerWheel {
    /// Time-indexed entries for efficient fire_expired.
    by_deadline: BTreeMap<Instant, Vec<TimerEntry>>,
    /// Run-indexed entries for O(1) cancel/lookup.
    by_run: Map<RunId, TimerEntry>,
    /// Maximum number of timers allowed.
    capacity: usize,
}

impl TimerWheel {
    /// Creates an empty timer wheel with a default capacity of 65536.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(65_536)
    }

    /// Creates an empty timer wheel with the specified maximum capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            by_deadline: BTreeMap::new(),
            by_run: Map::new(),
            capacity,
        }
    }

    /// Inserts a timer for the given run with the specified deadline.
    ///
    /// If a timer already exists for this run, it is replaced.
    pub fn insert(
        &mut self,
        run: RunId,
        deadline: Instant,
        kind: PendingTimerKind,
    ) -> Result<(), TimerWheelError> {
        if self.by_run.len() >= self.capacity && !self.by_run.contains_key(&run) {
            return Err(TimerWheelError::CapacityExceeded);
        }
        let generation = self.next_generation(run)?;
        self.cancel(run);
        let entry = TimerEntry {
            run,
            generation,
            deadline,
            kind,
        };
        self.by_deadline.entry(deadline).or_default().push(entry);
        self.by_run.insert(run, entry);
        Ok(())
    }

    fn next_generation(&self, run: RunId) -> Result<u64, TimerWheelError> {
        match self.by_run.get(&run).copied() {
            Some(entry) => entry
                .generation
                .checked_add(1)
                .ok_or(TimerWheelError::GenerationExhausted),
            None => Ok(1),
        }
    }

    /// Cancels the timer for the given run, if one exists.
    ///
    /// Returns true if a timer was removed.
    pub fn cancel(&mut self, run: RunId) -> bool {
        let Some(entry) = self.by_run.remove(&run) else {
            return false;
        };
        if let Some(entries) = self.by_deadline.get_mut(&entry.deadline) {
            entries.retain(|e| e.run != run);
            if entries.is_empty() {
                self.by_deadline.remove(&entry.deadline);
            }
        }
        true
    }

    /// Fires all timers whose deadlines have passed.
    ///
    /// Returns the fired entries in deadline order.
    ///
    /// Bounded output: pre-allocates output `Vec` to the current pending timer count,
    /// capping the worst-case allocation to `self.by_run.len()`. The intermediate
    /// `expired_keys` buffer is bounded by the number of distinct deadline instants
    /// that have expired — at most `self.by_deadline.len()` but typically far fewer.
    pub fn fire_expired(&mut self, now: Instant) -> Vec<TimerEntry> {
        let capacity = self.by_run.len();
        let mut fired = Vec::with_capacity(capacity);
        let expired_keys: Vec<Instant> = self
            .by_deadline
            .range(..=now)
            .map(|(&deadline, _)| deadline)
            .collect();

        for deadline in expired_keys {
            if let Some(entries) = self.by_deadline.remove(&deadline) {
                for entry in entries {
                    if self.by_run.get(&entry.run).copied() == Some(entry) {
                        self.by_run.remove(&entry.run);
                    }
                    fired.push(entry);
                }
            }
        }
        fired
    }

    /// Returns the next deadline, if any timers are pending.
    #[must_use]
    pub fn next_deadline(&self) -> Option<Instant> {
        self.by_deadline.first_key_value().map(|(k, _)| *k)
    }

    /// Returns true if no timers are pending.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_run.is_empty()
    }

    /// Returns the number of pending timers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_run.len()
    }

    /// Gets the kind of timer for a run, if one exists.
    #[must_use]
    pub fn get_kind(&self, run: RunId) -> Option<PendingTimerKind> {
        self.by_run.get(&run).map(|entry| entry.kind)
    }

    /// Gets the current timer entry for a run, if one exists.
    #[must_use]
    pub fn get_entry(&self, run: RunId) -> Option<TimerEntry> {
        self.by_run.get(&run).copied()
    }
}

impl Default for TimerWheel {
    fn default() -> Self {
        Self::new()
    }
}
