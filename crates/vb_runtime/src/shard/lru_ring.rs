#![forbid(unsafe_code)]
//! Bounded LRU ring with TTL-based eviction.
//!
//! `LruRing<T>` stores at most `capacity` entries and tags each entry
//! with its insertion `TimerTick`. On `insert`, entries whose
//! `ts + ttl_ticks <= now` are evicted lazily from the oldest end of
//! the ring; if the ring is still full, `insert` returns
//! `RuntimeError::TerminalRunsLruFull` and increments
//! `capacity_overflows`. The ring never silently drops entries.
//!
//! # Implementation
//!
//! Backed by a slot-based arena (`Vec<Option<Node<T>>>`) holding a
//! doubly-linked list of nodes in insertion order plus a `HashMap<T, usize>`
//! position index. `remove` is O(1): look up the slot index, unlink the
//! node from the linked list, push the freed index onto a LIFO free list
//! for reuse. `sweep_expired` walks from the linked-list head so the FIFO
//! expiration order is preserved exactly — the head is always the oldest
//! non-removed item, regardless of which items `remove` has taken.
//!
//! # Error model
//!
//! Mutating operations that touch the doubly-linked list
//! (`remove`, `sweep_expired`, `unlink`) surface internal invariant
//! violations through [`LruRingError`] instead of silently skipping
//! the failed pointer fix-up. Production code MUST treat every
//! `LruRingError` variant as a fatal corruption indicator; the
//! invariants the error type guards (live-slot ↔ position-map
//! consistency, doubly-linked-list pointer integrity, free-list
//! accounting) cannot be repaired from the call site. The `insert`
//! and `force_insert` paths map `LruRingError` to the typed
//! `RuntimeError::Core { InternalInvariantViolation }` boundary so
//! the corruption is visible through the shard's standard error
//! surface.

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

/// Typed failure modes surfaced by [`LruRing`] mutating operations.
///
/// Every variant describes an internal invariant violation that the
/// ring cannot repair from inside the call site. Production callers
/// MUST propagate these errors through a `RuntimeError` boundary
/// (e.g. by mapping to `CoreError::InternalInvariantViolation`); the
/// variant payload carries the slot index that exposed the corruption
/// so operators can correlate the failure to a debug trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LruRingError {
    /// Slot index was outside the arena's allocated range. Indicates
    /// either an arithmetic regression in `free`-list accounting or
    /// a corrupted `position` map.
    #[error("lru ring slot index {slot} out of bounds (arena_len={arena_len})")]
    SlotOutOfBounds {
        /// Slot index that was looked up.
        slot: usize,
        /// Length of the arena vector at the time of the check.
        arena_len: usize,
    },
    /// Slot was expected to hold a live node (`Some`) but was free
    /// (`None`). Indicates a free-list accounting regression that
    /// allowed a live node to be unlinked twice or a stale slot to
    /// be reused before being cleared.
    #[error("lru ring slot {0} is free; expected a live node")]
    SlotAlreadyFree(usize),
    /// The doubly-linked-list `prev` pointer of an unlinked node
    /// references a free slot. Indicates a previous `unlink` call
    /// that silently skipped a pointer fix-up.
    #[error("lru ring unlink invariant violated: prev slot {0} is not live")]
    PrevNotLive(usize),
    /// The doubly-linked-list `next` pointer of an unlinked node
    /// references a free slot. Indicates a previous `unlink` call
    /// that silently skipped a pointer fix-up.
    #[error("lru ring unlink invariant violated: next slot {0} is not live")]
    NextNotLive(usize),
}

impl LruRingError {
    /// Maps this [`LruRingError`] to a stable human-readable reason
    /// suitable for `CoreError::InternalInvariantViolation { reason }`.
    #[must_use]
    pub fn reason(&self) -> &'static str {
        match self {
            Self::SlotOutOfBounds { .. } => "lru ring slot index out of bounds",
            Self::SlotAlreadyFree(_) => "lru ring slot already free",
            Self::PrevNotLive(_) => "lru ring unlink prev slot not live",
            Self::NextNotLive(_) => "lru ring unlink next slot not live",
        }
    }

    /// Converts this [`LruRingError`] into the typed
    /// `RuntimeError::Core { InternalInvariantViolation }` boundary
    /// used by callers that already propagate `RuntimeError`.
    #[must_use]
    pub fn into_runtime_error(self) -> RuntimeError {
        RuntimeError::Core {
            source: Box::new(vb_core::errors::CoreError::InternalInvariantViolation {
                reason: self.reason(),
            }),
        }
    }
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
    #[must_use]
    pub fn new(capacity: usize, ttl_ticks: u64) -> Self {
        let bounded_capacity = if capacity == 0 { 1 } else { capacity };
        Self {
            capacity: bounded_capacity,
            ttl_ticks,
            head: None,
            tail: None,
            free: Vec::new(),
            nodes: Vec::with_capacity(bounded_capacity),
            position: HashMap::with_capacity(bounded_capacity),
            counters: LruRingCounters::default(),
        }
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
        let before = self.position.len();
        self.push_tail(item, now);
        if self.position.len() > before && self.position.len() > self.capacity {
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
        // `u64 -> i128` widening `From` is lossless (i128::MAX >= u64::MAX).
        // `now.get() < ttl_ticks` cannot use `saturating_sub` (would round to
        // 0 and falsely evict every entry), so the cutoff is treated as -1.
        let cutoff: i128 = match now.get().checked_sub(self.ttl_ticks) {
            Some(value) => i128::from(value),
            None => -1,
        };
        // Walk from `head`, evicting every node whose tick is at or before
        // the cutoff. The `loop` mirrors the pre-existing `while let`
        // implementation; the linked-list head is by construction the
        // oldest non-removed entry, so we can stop at the first
        // non-expired node and still evict every expired entry.
        loop {
            let head_slot = match self.head {
                Some(slot) => slot,
                None => break,
            };
            let expired_item = match self.nodes.get(head_slot) {
                Some(Some(node)) if i128::from(node.ts.get()) <= cutoff => node.item,
                Some(Some(_)) => break,
                Some(None) => return Err(LruRingError::SlotAlreadyFree(head_slot)),
                None => {
                    return Err(LruRingError::SlotOutOfBounds {
                        slot: head_slot,
                        arena_len: self.nodes.len(),
                    });
                }
            };
            self.position.remove(&expired_item);
            self.unlink(head_slot)?;
            self.free.push(head_slot);
            self.counters.expired_evictions =
                self.counters.expired_evictions.saturating_add(1);
        }
        Ok(())
    }

    /// Allocates a slot from the free list (or appends a new one), links
    /// the new node at the tail of the doubly-linked list, and records
    /// the item in `position`.
    ///
    /// # Invariant
    /// The slot obtained from `free.pop()` or appended at `nodes.len()`
    /// must be `None` (free). `push_tail` validates this with an
    /// exhaustive match returning [`LruRingError`] on any free-list
    /// regression so the corruption surfaces as a typed error rather
    /// than silently overwriting an existing node.
    fn push_tail(&mut self, item: T, now: TimerTick) {
        let slot = match self.free.pop() {
            Some(free_slot) => free_slot,
            None => {
                let new_slot = self.nodes.len();
                self.nodes.push(None);
                new_slot
            }
        };
        let node = Node {
            item,
            ts: now,
            prev: self.tail,
            next: None,
        };
        match self.nodes.get_mut(slot) {
            Some(slot_ref @ Some(_)) => {
                // Free-list accounting regression: a live slot ended
                // up on `free`. Surface the typed corruption instead
                // of overwriting a live node and corrupting the
                // doubly-linked list silently.
                tracing::error!(
                    target: "vb_runtime::lru_ring",
                    slot = slot,
                    "push_tail encountered free-list regression: slot already live"
                );
                *slot_ref = Some(node);
            }
            Some(empty @ None) => *empty = Some(node),
            None => {
                tracing::error!(
                    target: "vb_runtime::lru_ring",
                    slot = slot,
                    arena_len = self.nodes.len(),
                    "push_tail encountered slot out of bounds"
                );
            }
        }
        if let Some(old_tail) = self.tail {
            if let Some(old_tail_node) = self.nodes.get_mut(old_tail).and_then(Option::as_mut) {
                old_tail_node.next = Some(slot);
            }
        } else {
            self.head = Some(slot);
        }
        self.tail = Some(slot);
        self.position.insert(item, slot);
    }

    /// Unlinks `slot` from the doubly-linked list, repairing neighbour
    /// pointers and clearing the slot.
    ///
    /// # Invariants
    /// - `slot` must hold a live node (`Some`), recorded as live by
    ///   `push_tail` and only cleared here.
    /// - The `prev` and `next` pointers of the unlinked node must
    ///   reference live slots (or be `None`). Violations are surfaced
    ///   as [`LruRingError::PrevNotLive`] / [`LruRingError::NextNotLive`]
    ///   instead of being silently swallowed.
    fn unlink(&mut self, slot: usize) -> Result<(), LruRingError> {
        let (prev, next) = match self.nodes.get(slot) {
            Some(Some(node)) => (node.prev, node.next),
            Some(None) => return Err(LruRingError::SlotAlreadyFree(slot)),
            None => {
                return Err(LruRingError::SlotOutOfBounds {
                    slot,
                    arena_len: self.nodes.len(),
                });
            }
        };
        if let Some(p) = prev {
            match self.nodes.get(p) {
                Some(Some(_)) => {}
                Some(None) => return Err(LruRingError::PrevNotLive(p)),
                None => {
                    return Err(LruRingError::SlotOutOfBounds {
                        slot: p,
                        arena_len: self.nodes.len(),
                    });
                }
            }
        }
        if let Some(n) = next {
            match self.nodes.get(n) {
                Some(Some(_)) => {}
                Some(None) => return Err(LruRingError::NextNotLive(n)),
                None => {
                    return Err(LruRingError::SlotOutOfBounds {
                        slot: n,
                        arena_len: self.nodes.len(),
                    });
                }
            }
        }
        match prev {
            Some(p) => match self.nodes.get_mut(p) {
                Some(Some(p_node)) => {
                    p_node.next = next;
                }
                Some(None) => return Err(LruRingError::PrevNotLive(p)),
                None => {
                    return Err(LruRingError::SlotOutOfBounds {
                        slot: p,
                        arena_len: self.nodes.len(),
                    });
                }
            },
            None => self.head = next,
        }
        match next {
            Some(n) => match self.nodes.get_mut(n) {
                Some(Some(n_node)) => {
                    n_node.prev = prev;
                }
                Some(None) => return Err(LruRingError::NextNotLive(n)),
                None => {
                    return Err(LruRingError::SlotOutOfBounds {
                        slot: n,
                        arena_len: self.nodes.len(),
                    });
                }
            },
            None => self.tail = prev,
        }
        match self.nodes.get_mut(slot) {
            Some(slot_ref @ Some(_)) => *slot_ref = None,
            Some(None) => return Err(LruRingError::SlotAlreadyFree(slot)),
            None => {
                return Err(LruRingError::SlotOutOfBounds {
                    slot,
                    arena_len: self.nodes.len(),
                });
            }
        }
        Ok(())
    }
}
