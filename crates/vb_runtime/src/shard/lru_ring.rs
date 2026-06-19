#![forbid(unsafe_code)]
//! Bounded LRU ring with TTL-based eviction.
//!
//! `LruRing<T>` stores at most `capacity` entries. Each entry is tagged
//! with the `TimerTick` value at insertion. On `insert`, expired entries
//! (those whose `ts + ttl_ticks <= now`) are evicted lazily from the
//! oldest end of the ring. After the sweep, if the ring is still at
//! capacity, `insert` returns `RuntimeError::TerminalRunsLruFull` — the
//! caller decides what to do.
//!
//! The ring never silently drops entries: every eviction is either
//! (a) an explicit TTL sweep that increments `expired_evictions`, or
//! (b) a refused insert that increments `capacity_overflows`.
//!
//! `force_insert` is provided for callers that prefer "bounded with
//! observable counter" over "Err-on-overflow"; the existing
//! `Shard::terminal_runs_insert` uses that path so the legacy public
//! signature is preserved.

use std::collections::VecDeque;
use std::hash::Hash;

use indexmap::IndexSet;

use crate::RuntimeError;
use crate::shard::timer::TimerTick;

/// Default maximum number of entries in a terminal-runs ring.
pub const DEFAULT_MAX_TERMINAL_RUNS: usize = 100_000;

/// Default TTL in ticks for terminal-runs entries.
pub const DEFAULT_TERMINAL_RUNS_TTL_TICKS: u64 = 86_400;

/// Diagnostic counters for a `LruRing`.
///
/// Both counters saturate at `u64::MAX`; they never wrap and never
/// panic. Production code reads them via `ShardCounters` aggregation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LruRingCounters {
    /// Number of entries removed by TTL sweep.
    pub expired_evictions: u64,
    /// Number of refused inserts (capacity reached) or forced inserts
    /// that grew the ring past `capacity`.
    pub capacity_overflows: u64,
}

/// Bounded LRU ring keyed by insertion order with TTL-based eviction.
#[derive(Debug)]
pub struct LruRing<T>
where
    T: Copy + Eq + Hash,
{
    capacity: usize,
    ttl_ticks: u64,
    order: VecDeque<(T, TimerTick)>,
    members: IndexSet<T>,
    counters: LruRingCounters,
}

impl<T> LruRing<T>
where
    T: Copy + Eq + Hash,
{
    /// Creates a ring with the given bounded capacity and TTL (in ticks).
    #[must_use]
    pub fn new(capacity: usize, ttl_ticks: u64) -> Self {
        let bounded_capacity = if capacity == 0 { 1 } else { capacity };
        Self {
            capacity: bounded_capacity,
            ttl_ticks,
            order: VecDeque::with_capacity(bounded_capacity),
            members: IndexSet::with_capacity(bounded_capacity),
            counters: LruRingCounters::default(),
        }
    }

    /// Returns the configured capacity.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the configured TTL in ticks.
    #[must_use]
    pub const fn ttl_ticks(&self) -> u64 {
        self.ttl_ticks
    }

    /// Returns the current number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Returns true when the ring holds no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Returns true when the ring holds `capacity` entries.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.members.len() >= self.capacity
    }

    /// Returns true when the given item is in the ring.
    #[must_use]
    pub fn contains(&self, item: &T) -> bool {
        self.members.contains(item)
    }

    /// Returns a snapshot of the diagnostic counters.
    #[must_use]
    pub const fn counters(&self) -> LruRingCounters {
        self.counters
    }

    /// Removes every entry from the ring (does not change capacity or TTL).
    pub fn clear(&mut self) {
        self.order.clear();
        self.members.clear();
    }

    /// Inserts `item`, sweeping TTL-expired entries first.
    ///
    /// * If `item` is already present, returns `Ok(())` without bumping
    ///   the insertion tick (idempotent membership is preserved).
    /// * If the ring has room after the sweep, inserts and returns `Ok`.
    /// * If the ring is still full, increments `capacity_overflows` and
    ///   returns `Err(RuntimeError::TerminalRunsLruFull)`.
    pub fn insert(&mut self, item: T, now: TimerTick) -> Result<(), RuntimeError> {
        if self.members.contains(&item) {
            return Ok(());
        }
        self.sweep_expired(now);
        if self.members.len() >= self.capacity {
            self.counters.capacity_overflows = self.counters.capacity_overflows.saturating_add(1);
            return Err(RuntimeError::TerminalRunsLruFull {
                capacity: self.capacity,
            });
        }
        self.order.push_back((item, now));
        self.members.insert(item);
        Ok(())
    }

    /// Forces insertion, sweeping TTL-expired entries first.
    ///
    /// If the ring is at capacity after the sweep, increments
    /// `capacity_overflows` and grows the ring past `capacity` rather
    /// than returning an error. Use this when the caller prefers
    /// "never drop" semantics and tracks overflow via counters.
    pub fn force_insert(&mut self, item: T, now: TimerTick) {
        if self.members.contains(&item) {
            return;
        }
        self.sweep_expired(now);
        let before = self.members.len();
        self.order.push_back((item, now));
        self.members.insert(item);
        if self.members.len() > before && self.members.len() > self.capacity {
            self.counters.capacity_overflows = self.counters.capacity_overflows.saturating_add(1);
        }
    }

    /// Removes the entry for `item` if present. No-op otherwise.
    pub fn remove(&mut self, item: &T) {
        if self.members.swap_remove(item) {
            // Re-scan the order deque to drop the matching (item, ts) tuple.
            // O(n) but rare on the terminal-runs path.
            if let Some(position) = self.order.iter().position(|(value, _)| value == item) {
                self.order.remove(position);
            }
        }
    }
    /// Evicts every entry whose insertion tick satisfies `ts + ttl_ticks <= now`.
    /// Increments `expired_evictions` for every removed entry.
    pub fn sweep_expired(&mut self, now: TimerTick) {
        if self.ttl_ticks == 0 {
            return;
        }
        // Compute the cutoff as the latest tick value still considered
        // alive. When `now.get() < ttl_ticks` we cannot use
        // `saturating_sub` (it would round to 0 and falsely evict every
        // entry), so the cutoff is treated as -1 (no expiration).
        let cutoff: i128 = if (now.get() as i128) < (self.ttl_ticks as i128) {
            -1
        } else {
            (now.get() - self.ttl_ticks) as i128
        };
        while let Some(&(value, ts)) = self.order.front() {
            if (ts.get() as i128) <= cutoff {
                self.order.pop_front();
                self.members.swap_remove(&value);
                self.counters.expired_evictions = self.counters.expired_evictions.saturating_add(1);
            } else {
                break;
            }
        }
    }
}
