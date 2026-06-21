#![forbid(unsafe_code)]
//! Bounded LRU ring with TTL-based eviction.
//!
//! `LruRing<T>` stores at most `capacity` entries tagged with their
//! insertion `TimerTick`. On `insert`, entries whose
//! `ts + ttl_ticks <= now` are evicted lazily from the oldest end of
//! the ring; if the ring is still full, `insert` returns
//! `RuntimeError::TerminalRunsLruFull` and increments
//! `capacity_overflows`. The ring never silently drops entries.
//!
//! Backed by a slot-based arena (`Vec<Option<Node<T>>>`) holding a
//! doubly-linked list in insertion order plus a `HashMap<T, usize>`
//! position index. `remove` is O(1) and `sweep_expired` walks from the
//! linked-list head so FIFO expiration order is preserved. Internal
//! invariant violations surface as [`LruRingError`]; see
//! [`invariants`] for the linked-list machinery.

mod invariants;

pub use invariants::LruRingError;

use std::collections::HashMap;
use std::hash::Hash;

use crate::RuntimeError;
use crate::shard::timer::TimerTick;

/// Default maximum number of entries in a terminal-runs ring.
pub const DEFAULT_MAX_TERMINAL_RUNS: usize = 100_000;

/// Default TTL in ticks for terminal-runs entries.
pub const DEFAULT_TERMINAL_RUNS_TTL_TICKS: u64 = 86_400;

/// Diagnostic counters for a `LruRing`. Saturate at `u64::MAX`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LruRingCounters {
    /// Number of entries removed by TTL sweep.
    pub expired_evictions: u64,
    /// Number of refused inserts (capacity reached) or forced inserts that grew past `capacity`.
    pub capacity_overflows: u64,
}

/// Doubly-linked-list node stored in one arena slot.
#[derive(Debug, Clone, Copy)]
struct Node<T>
where
    T: Copy + Eq + Hash,
{
    item: T,
    ts: TimerTick,
    prev: Option<usize>,
    next: Option<usize>,
}

/// Bounded LRU ring keyed by insertion order with TTL-based eviction.
#[derive(Debug)]
pub struct LruRing<T>
where
    T: Copy + Eq + Hash,
{
    capacity: usize,
    ttl_ticks: u64,
    /// Index of the oldest live slot (front of the insertion order).
    head: Option<usize>,
    /// Index of the newest live slot (back of the insertion order).
    tail: Option<usize>,
    /// LIFO free list of slot indices available for reuse.
    free: Vec<usize>,
    /// Slot-based arena. `None` = free, `Some(node)` = live.
    nodes: Vec<Option<Node<T>>>,
    /// Item → slot index. O(1) lookup for `contains` and `remove`.
    position: HashMap<T, usize>,
    counters: LruRingCounters,
}

impl<T> LruRing<T>
where
    T: Copy + Eq + Hash,
{
    /// Creates a ring with the given bounded capacity and TTL (in ticks).
    ///
    /// Returns [`RuntimeError::LruRingCapacityZero`] when `capacity == 0`
    /// so a misconfigured capacity is surfaced at the runtime boundary
    /// instead of being silently rewritten to `1` (the legacy
    /// `LruRing::new` behaviour that masked configuration bugs).
    pub fn try_new(capacity: usize, ttl_ticks: u64) -> Result<Self, RuntimeError> {
        if capacity == 0 {
            return Err(RuntimeError::LruRingCapacityZero);
        }
        Ok(Self {
            capacity,
            ttl_ticks,
            head: None,
            tail: None,
            free: Vec::new(),
            nodes: Vec::with_capacity(capacity),
            position: HashMap::with_capacity(capacity),
            counters: LruRingCounters::default(),
        })
    }

    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    #[must_use]
    pub const fn ttl_ticks(&self) -> u64 {
        self.ttl_ticks
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.position.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.position.is_empty()
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.position.len() >= self.capacity
    }

    #[must_use]
    pub fn contains(&self, item: &T) -> bool {
        self.position.contains_key(item)
    }

    #[must_use]
    pub const fn counters(&self) -> LruRingCounters {
        self.counters
    }

    /// Removes every entry from the ring (does not change capacity or TTL).
    pub fn clear(&mut self) {
        self.head = None;
        self.tail = None;
        self.free.clear();
        for slot in self.nodes.iter_mut() {
            *slot = None;
        }
        self.position.clear();
    }

    /// Inserts `item`, sweeping TTL-expired entries first.
    ///
    /// Idempotent if `item` is already present. Returns
    /// `RuntimeError::TerminalRunsLruFull` when full after sweep, or
    /// `RuntimeError::Core { InternalInvariantViolation }` if the
    /// underlying [`LruRingError`] surfaces through the sweep path.
    pub fn insert(&mut self, item: T, now: TimerTick) -> Result<(), RuntimeError> {
        if self.position.contains_key(&item) {
            return Ok(());
        }
        self.sweep_expired(now).map_err(LruRingError::into_runtime_error)?;
        if self.position.len() >= self.capacity {
            self.counters.capacity_overflows = self.counters.capacity_overflows.saturating_add(1);
            return Err(RuntimeError::TerminalRunsLruFull {
                capacity: self.capacity,
            });
        }
        self.push_tail(item, now);
        Ok(())
    }

    /// Forces insertion past capacity, tracking overflow via counters.
    ///
    /// Preserves the legacy `()`-returning contract relied on by the
    /// terminal-runs counter path (`finish_run`, `fail_run_state`,
    /// `handle_cancel`, `handle_kill`). Internal invariant violations
    /// surfaced by the embedded `sweep_expired` call are mapped to the
    /// typed `RuntimeError::Core { InternalInvariantViolation }` path
    /// via [`LruRingError::into_runtime_error`] and logged through
    /// `tracing::error!` so the corruption is observable on every
    /// subsequent shard operation without changing the public signature.
    pub fn force_insert(&mut self, item: T, now: TimerTick) {
        if self.position.contains_key(&item) {
            return;
        }
        if let Err(error) = self.sweep_expired(now) {
            tracing::error!(
                target: "vb_runtime::lru_ring",
                error = %error,
                "force_insert encountered lru ring invariant violation during sweep"
            );
        }
        self.push_tail(item, now);
        if self.position.len() > self.capacity {
            self.counters.capacity_overflows = self.counters.capacity_overflows.saturating_add(1);
        }
    }

    /// Removes the entry for `item` if present. No-op otherwise.
    ///
    /// O(1): the `position` map gives the slot index, the slot is unlinked
    /// from the doubly-linked list, and the freed index is pushed onto
    /// the free list for reuse.
    ///
    /// # Errors
    ///
    /// Returns [`LruRingError`] when the position map and the
    /// arena disagree (e.g. the slot recorded for `item` is free or
    /// out of bounds, or the doubly-linked-list pointers reference
    /// a non-live slot). Callers MUST propagate these errors; they
    /// indicate internal corruption that cannot be repaired silently.
    #[must_use = "lru ring remove must propagate internal invariant errors"]
    pub fn remove(&mut self, item: &T) -> Result<(), LruRingError> {
        let slot = match self.position.remove(item) {
            Some(found) => found,
            None => return Ok(()),
        };
        self.unlink(slot)?;
        self.free.push(slot);
        Ok(())
    }

    /// Evicts every entry whose insertion tick satisfies `ts + ttl_ticks <= now`.
    /// Increments `expired_evictions` for every removed entry.
    ///
    /// # Errors
    ///
    /// Returns [`LruRingError`] when the doubly-linked-list pointers
    /// reference a non-live slot, indicating internal corruption.
    /// Callers MUST propagate these errors.
    pub fn sweep_expired(&mut self, now: TimerTick) -> Result<(), LruRingError> {
        if self.ttl_ticks == 0 {
            return Ok(());
        }
        let cutoff: i128 = match now.get().checked_sub(self.ttl_ticks) {
            Some(value) => i128::from(value),
            None => -1,
        };
        loop {
            let (head_slot, expired_item) = match self.head_node_expired(cutoff)? {
                Some(pair) => pair,
                None => break,
            };
            self.position.remove(&expired_item);
            self.unlink(head_slot)?;
            self.free.push(head_slot);
            self.counters.expired_evictions =
                self.counters.expired_evictions.saturating_add(1);
        }
        Ok(())
    }
}