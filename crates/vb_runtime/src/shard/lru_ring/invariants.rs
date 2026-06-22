//! Linked-list invariant machinery for [`LruRing`].
//!
//! This module holds the doubly-linked-list operations that must keep
//! the ring's internal pointers and free-list accounting consistent.
//! Public mutating methods on [`LruRing`] live in the parent module;
//! the helpers here are exposed `pub(super)` so the public path can call
//! them without leaking internal types into the public API.

use super::LruRing;
use crate::shard::timer::TimerTick;
use std::hash::Hash;

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
    pub fn into_runtime_error(self) -> crate::RuntimeError {
        crate::RuntimeError::Core {
            source: Box::new(vb_core::errors::CoreError::InternalInvariantViolation {
                reason: self.reason(),
            }),
        }
    }
}

impl<T> LruRing<T>
where
    T: Copy + Eq + Hash,
{
    /// Inspects the head of the ring for TTL expiry.
    ///
    /// Returns `Ok(Some((slot, item)))` when the head exists and is
    /// expired at `cutoff`, `Ok(None)` when the head exists but is
    /// not yet expired (or the ring is empty), and `Err(...)` when the
    /// head slot is invalid.
    pub(super) fn head_node_expired(
        &self,
        cutoff: i128,
    ) -> Result<Option<(usize, T)>, LruRingError> {
        let head_slot = match self.head {
            Some(slot) => slot,
            None => return Ok(None),
        };
        match self.nodes.get(head_slot) {
            Some(Some(node)) if i128::from(node.ts.get()) <= cutoff => {
                Ok(Some((head_slot, node.item)))
            }
            Some(Some(_)) => Ok(None),
            Some(None) => Err(LruRingError::SlotAlreadyFree(head_slot)),
            None => Err(LruRingError::SlotOutOfBounds {
                slot: head_slot,
                arena_len: self.nodes.len(),
            }),
        }
    }

    /// Allocates a slot from the free list, or appends a new slot to
    /// the arena.
    fn allocate_slot(&mut self) -> usize {
        match self.free.pop() {
            Some(free_slot) => free_slot,
            None => {
                let new_slot = self.nodes.len();
                self.nodes.push(None);
                new_slot
            }
        }
    }

    /// Writes `node` to `slot`, logging free-list regressions and
    /// overwriting any live slot so the entry is not silently dropped.
    fn write_node_at(&mut self, slot: usize, node: super::Node<T>) {
        match self.nodes.get_mut(slot) {
            Some(slot_ref @ Some(_)) => {
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
    }

    /// Updates `head`/`tail` and the previous tail's `next` pointer so
    /// that `slot` is linked at the tail of the insertion order.
    fn link_new_tail(&mut self, slot: usize) {
        if let Some(old_tail) = self.tail {
            if let Some(old_tail_node) = self.nodes.get_mut(old_tail).and_then(Option::as_mut) {
                old_tail_node.next = Some(slot);
            }
        } else {
            self.head = Some(slot);
        }
        self.tail = Some(slot);
    }

    /// Allocates a slot from the free list (or appends a new one), links
    /// the new node at the tail of the doubly-linked list, and records
    /// the item in `position`.
    ///
    /// # Invariant
    /// The slot obtained from `free.pop()` or appended at `nodes.len()`
    /// must be `None` (free). `push_tail` validates this with an
    /// exhaustive match logging a free-list regression so the corruption
    /// surfaces through diagnostics rather than silently overwriting an
    /// existing node.
    pub(super) fn push_tail(&mut self, item: T, now: TimerTick) {
        let slot = self.allocate_slot();
        let node = super::Node {
            item,
            ts: now,
            prev: self.tail,
            next: None,
        };
        self.write_node_at(slot, node);
        self.link_new_tail(slot);
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
    pub(super) fn unlink(&mut self, slot: usize) -> Result<(), LruRingError> {
        let (prev, next) = self.unlink_read_pointers(slot)?;
        self.unlink_repair_neighbors(prev, next)?;
        self.unlink_drop_slot(slot)
    }

    /// Reads `slot`'s prev/next pointers, validating that `slot` is
    /// live and that the prev/next pointers (when `Some`) reference
    /// live nodes. Must be called before any mutation so that
    /// pointer-fix-up errors surface before the linked list is rewired.
    fn unlink_read_pointers(
        &self,
        slot: usize,
    ) -> Result<(Option<usize>, Option<usize>), LruRingError> {
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
        self.unlink_assert_neighbor_live(prev, LruRingError::PrevNotLive)?;
        self.unlink_assert_neighbor_live(next, LruRingError::NextNotLive)?;
        Ok((prev, next))
    }

    /// Validates that `slot` (when `Some`) points to a live arena
    /// slot, returning `free_err` for free slots or `SlotOutOfBounds`
    /// for out-of-range indices.
    fn unlink_assert_neighbor_live(
        &self,
        slot: Option<usize>,
        free_err: fn(usize) -> LruRingError,
    ) -> Result<(), LruRingError> {
        let Some(idx) = slot else { return Ok(()) };
        match self.nodes.get(idx) {
            Some(Some(_)) => Ok(()),
            Some(None) => Err(free_err(idx)),
            None => Err(LruRingError::SlotOutOfBounds {
                slot: idx,
                arena_len: self.nodes.len(),
            }),
        }
    }

    /// Repairs the doubly-linked list after `slot` is removed: updates
    /// prev's next pointer (or head) and next's prev pointer (or tail)
    /// to skip `slot`.
    fn unlink_repair_neighbors(
        &mut self,
        prev: Option<usize>,
        next: Option<usize>,
    ) -> Result<(), LruRingError> {
        if let Some(p) = prev {
            let p_node = self.unlink_take_mut(p, LruRingError::PrevNotLive)?;
            p_node.next = next;
        } else {
            self.head = next;
        }
        if let Some(n) = next {
            let n_node = self.unlink_take_mut(n, LruRingError::NextNotLive)?;
            n_node.prev = prev;
        } else {
            self.tail = prev;
        }
        Ok(())
    }

    /// Mutably borrows the live node at `slot`, returning the error
    /// variant `free_err` if the slot is free, or
    /// `LruRingError::SlotOutOfBounds` if the index is past the arena's
    /// length.
    fn unlink_take_mut(
        &mut self,
        slot: usize,
        free_err: fn(usize) -> LruRingError,
    ) -> Result<&mut super::Node<T>, LruRingError> {
        let arena_len = self.nodes.len();
        match self.nodes.get_mut(slot) {
            Some(slot_ref) => match slot_ref {
                Some(node) => Ok(node),
                None => Err(free_err(slot)),
            },
            None => Err(LruRingError::SlotOutOfBounds { slot, arena_len }),
        }
    }

    /// Clears `slot` after the linked list has been repaired.
    fn unlink_drop_slot(&mut self, slot: usize) -> Result<(), LruRingError> {
        match self.nodes.get_mut(slot) {
            Some(slot_ref @ Some(_)) => {
                *slot_ref = None;
                Ok(())
            }
            Some(None) => Err(LruRingError::SlotAlreadyFree(slot)),
            None => Err(LruRingError::SlotOutOfBounds {
                slot,
                arena_len: self.nodes.len(),
            }),
        }
    }
}
