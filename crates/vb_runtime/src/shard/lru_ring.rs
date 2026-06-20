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
    /// `RuntimeError::TerminalRunsLruFull` when full after sweep.
    pub fn insert(&mut self, item: T, now: TimerTick) -> Result<(), RuntimeError> {
        if self.position.contains_key(&item) {
            return Ok(());
        }
        self.sweep_expired(now);
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
    pub fn force_insert(&mut self, item: T, now: TimerTick) {
        if self.position.contains_key(&item) {
            return;
        }
        self.sweep_expired(now);
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
    pub fn remove(&mut self, item: &T) {
        if let Some(slot) = self.position.remove(item) {
            self.unlink(slot);
            self.free.push(slot);
        }
    }

    /// Evicts every entry whose insertion tick satisfies `ts + ttl_ticks <= now`.
    /// Increments `expired_evictions` for every removed entry.
    pub fn sweep_expired(&mut self, now: TimerTick) {
        if self.ttl_ticks == 0 {
            return;
        }
        // `u64 -> i128` widening `From` is lossless (i128::MAX >= u64::MAX).
        // `now.get() < ttl_ticks` cannot use `saturating_sub` (would round to
        // 0 and falsely evict every entry), so the cutoff is treated as -1.
        let cutoff: i128 = match now.get().checked_sub(self.ttl_ticks) {
            Some(value) => i128::from(value),
            None => -1,
        };
        // Walk from `head`, evicting every node whose tick is at or before
        // the cutoff. The `while let` is the same control-flow shape used
        // by the pre-existing implementation; the linked-list head is by
        // construction the oldest non-removed entry, so we can stop at the
        // first non-expired node and still evict every expired entry.
        while let Some(head_slot) = self.head {
            let expired_item = self
                .nodes
                .get(head_slot)
                .and_then(Option::as_ref)
                .filter(|node| i128::from(node.ts.get()) <= cutoff)
                .map(|node| node.item);
            match expired_item {
                Some(item) => {
                    self.position.remove(&item);
                    self.unlink(head_slot);
                    self.free.push(head_slot);
                    self.counters.expired_evictions =
                        self.counters.expired_evictions.saturating_add(1);
                }
                None => break,
            }
        }
    }

    /// Allocates a slot from the free list (or appends a new one), links
    /// the new node at the tail of the doubly-linked list, and records
    /// the item in `position`.
    ///
    /// # Invariant (debug-checked)
    /// The slot obtained from `free.pop()` or appended at `nodes.len()`
    /// must be `None` (free). A `debug_assert!` validates this on every
    /// call so a future regression that breaks the free-list accounting
    /// (e.g. pushing a live slot back onto `free`) surfaces as a panic
    /// rather than silently overwriting an existing node and corrupting
    /// the doubly-linked list.
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
        if let Some(slot_ref) = self.nodes.get_mut(slot) {
            debug_assert!(
                slot_ref.is_none(),
                "push_tail invariant: slot {slot} must be free (was Some)"
            );
            *slot_ref = Some(node);
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
    /// # Invariant (debug-checked)
    /// The `prev` and `next` pointers of a live node always reference live
    /// slots. A `debug_assert!` validates this on every call so a future
    /// regression that breaks the invariant (e.g. wrong free-list
    /// accounting) surfaces as a panic rather than a silent stale pointer.
    fn unlink(&mut self, slot: usize) {
        let (prev, next) = match self.nodes.get(slot).and_then(Option::as_ref) {
            Some(node) => (node.prev, node.next),
            None => return,
        };
        if let Some(p) = prev {
            debug_assert!(
                self.nodes.get(p).and_then(Option::as_ref).is_some(),
                "unlink invariant: prev slot {p} must be live"
            );
        }
        if let Some(n) = next {
            debug_assert!(
                self.nodes.get(n).and_then(Option::as_ref).is_some(),
                "unlink invariant: next slot {n} must be live"
            );
        }
        match prev {
            Some(p) => {
                if let Some(p_node) = self.nodes.get_mut(p).and_then(Option::as_mut) {
                    p_node.next = next;
                }
            }
            None => self.head = next,
        }
        match next {
            Some(n) => {
                if let Some(n_node) = self.nodes.get_mut(n).and_then(Option::as_mut) {
                    n_node.prev = prev;
                }
            }
            None => self.tail = prev,
        }
        if let Some(slot_ref) = self.nodes.get_mut(slot) {
            *slot_ref = None;
        }
    }
}
