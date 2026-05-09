#![forbid(unsafe_code)]

//! Bounded idempotency tracker for exactly-once action completion semantics.
//!
//! Tracks dispatched and completed action tickets by their idempotency key.
//! Capacity is bounded via ring-buffer eviction — oldest entries are discarded
//! when the tracker is full, which is safe because the durable journal is the
//! authoritative source of truth for crash recovery.

use std::collections::HashMap;

use vb_core::action::{ActionError, ActionTicket};

/// Default capacity for [`IdempotencyTracker::new()`].
const DEFAULT_CAPACITY: usize = 1024;

/// Bounded tracker that records completed action tickets for exactly-once
/// completion detection.
///
/// Uses a HashMap keyed by the ticket's `idempotency_key` for O(1) lookups.
/// When capacity is reached, the oldest entry is evicted (FIFO ring buffer).
#[derive(Debug, Clone)]
pub struct IdempotencyTracker {
    /// Map from idempotency key to the completed ticket.
    completed: HashMap<u128, ActionTicket>,
    /// Insertion order for FIFO eviction.
    order: Vec<u128>,
    /// Maximum number of entries before eviction.
    capacity: usize,
    /// Position of the next slot to overwrite in `order` during eviction.
    cursor: usize,
}

impl IdempotencyTracker {
    /// Creates a new tracker with the given bounded capacity.
    ///
    /// When the tracker reaches capacity, the oldest entry is evicted.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self::with_capacity(capacity)
    }

    /// Creates a new tracker with the default capacity.
    #[must_use]
    pub fn with_default_capacity() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Creates a new tracker with the given bounded capacity.
    ///
    /// When the tracker reaches capacity, the oldest entry is evicted.
    /// A capacity of zero is treated as 1 to ensure the tracker is usable.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let effective_capacity = capacity.max(1);
        Self {
            completed: HashMap::new(),
            order: Vec::new(),
            capacity: effective_capacity,
            cursor: 0,
        }
    }

    /// Returns `true` if the tracker holds no completed entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.completed.is_empty()
    }

    /// Returns the number of completed entries currently tracked.
    #[must_use]
    pub fn len(&self) -> usize {
        self.completed.len()
    }

    /// Marks an action ticket as dispatched.
    ///
    /// Returns `true` if this is a new dispatch (the ticket's idempotency key
    /// was not previously seen), or `false` if it is a duplicate dispatch.
    #[must_use]
    pub fn mark_dispatched(&self, ticket: &ActionTicket) -> bool {
        !self.completed.contains_key(&ticket.idempotency_key)
    }

    /// Records a successful completion for the given ticket.
    ///
    /// Returns `Err(ActionError::CompletionAlreadyRecorded)` if this ticket's
    /// idempotency key was already marked as completed.
    ///
    /// If the tracker is at capacity, the oldest entry is evicted before
    /// inserting the new one.
    pub fn mark_completed(&mut self, ticket: &ActionTicket) -> Result<(), ActionError> {
        if self.completed.contains_key(&ticket.idempotency_key) {
            return Err(ActionError::CompletionAlreadyRecorded);
        }
        self.evict_if_full();
        self.completed.insert(ticket.idempotency_key, *ticket);
        self.order.push(ticket.idempotency_key);
        Ok(())
    }

    /// Returns `true` if the given ticket's idempotency key has been completed.
    #[must_use]
    pub fn is_completed(&self, ticket: &ActionTicket) -> bool {
        self.completed.contains_key(&ticket.idempotency_key)
    }

    /// Returns `true` if a completion for the given idempotency key was already
    /// seen (i.e. this would be a duplicate completion).
    #[must_use]
    pub fn is_duplicate_completion(&self, ticket: &ActionTicket) -> bool {
        self.completed.contains_key(&ticket.idempotency_key)
    }

    /// Evicts the oldest entry if the tracker is at or above capacity.
    fn evict_if_full(&mut self) {
        if self.completed.len() < self.capacity {
            return;
        }
        // Try to evict from the ring cursor position. We bound iterations
        // to the order length to guarantee termination even if every key
        // has already been removed from the HashMap.
        let max_attempts = self.order.len();
        let mut attempts = 0;
        while attempts < max_attempts {
            attempts = match attempts.checked_add(1) {
                Some(n) => n,
                None => break,
            };
            let Some(&key) = self.order.get(self.cursor) else {
                break;
            };
            let removed = self.completed.remove(&key);
            let next = self.cursor.saturating_add(1);
            self.cursor = if next >= self.order.len() { 0 } else { next };
            if removed.is_some() {
                return;
            }
        }
    }
}

impl Default for IdempotencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

    fn make_ticket(key: u128) -> ActionTicket {
        ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(1),
            action: ActionId::new(1),
            attempt: 1,
            idempotency_key: key,
            capacity: 1,
        }
    }

    #[test]
    fn idempotency_tracker_new_is_empty() {
        let tracker = IdempotencyTracker::new();
        assert!(tracker.is_empty());
        assert_eq!(tracker.len(), 0);
    }

    #[test]
    fn idempotency_tracker_record_completion_succeeds() {
        let mut tracker = IdempotencyTracker::new();
        let ticket = make_ticket(42);
        assert_eq!(tracker.mark_completed(&ticket), Ok(()));
        assert!(tracker.is_completed(&ticket));
        assert_eq!(tracker.len(), 1);
    }

    #[test]
    fn idempotency_tracker_duplicate_completion_returns_error() {
        let mut tracker = IdempotencyTracker::new();
        let ticket = make_ticket(99);
        assert_eq!(tracker.mark_completed(&ticket), Ok(()));
        assert_eq!(
            tracker.mark_completed(&ticket),
            Err(vb_core::action::ActionError::CompletionAlreadyRecorded)
        );
    }

    #[test]
    fn idempotency_tracker_different_keys_are_independent() {
        let mut tracker = IdempotencyTracker::new();
        let ticket_a = make_ticket(1);
        let ticket_b = make_ticket(2);
        let ticket_c = make_ticket(3);
        assert_eq!(tracker.mark_completed(&ticket_a), Ok(()));
        assert_eq!(tracker.mark_completed(&ticket_b), Ok(()));
        assert!(tracker.is_completed(&ticket_a));
        assert!(tracker.is_completed(&ticket_b));
        assert!(!tracker.is_completed(&ticket_c));
        assert_eq!(tracker.len(), 2);
    }

    #[test]
    fn idempotency_tracker_default_matches_new() {
        let default = IdempotencyTracker::default();
        let new = IdempotencyTracker::new();
        assert_eq!(default.len(), new.len());
        assert_eq!(default.is_empty(), new.is_empty());
        assert_eq!(default.capacity, new.capacity);
    }

    #[test]
    fn idempotency_tracker_mark_dispatched_new_is_true() {
        let tracker = IdempotencyTracker::new();
        let ticket = make_ticket(10);
        assert!(tracker.mark_dispatched(&ticket));
    }

    #[test]
    fn idempotency_tracker_mark_dispatched_duplicate_is_false() {
        let mut tracker = IdempotencyTracker::new();
        let ticket = make_ticket(10);
        assert_eq!(tracker.mark_completed(&ticket), Ok(()));
        assert!(!tracker.mark_dispatched(&ticket));
    }

    #[test]
    fn idempotency_tracker_is_duplicate_completion_true_after_record() {
        let mut tracker = IdempotencyTracker::new();
        let ticket = make_ticket(55);
        assert!(!tracker.is_duplicate_completion(&ticket));
        assert_eq!(tracker.mark_completed(&ticket), Ok(()));
        assert!(tracker.is_duplicate_completion(&ticket));
    }

    #[test]
    fn idempotency_tracker_eviction_oldest_removed() {
        let mut tracker = IdempotencyTracker::with_capacity(2);
        let ticket_a = make_ticket(1);
        let ticket_b = make_ticket(2);
        let ticket_c = make_ticket(3);

        assert_eq!(tracker.mark_completed(&ticket_a), Ok(()));
        assert_eq!(tracker.mark_completed(&ticket_b), Ok(()));
        // At capacity. Adding ticket_c should evict ticket_a.
        assert_eq!(tracker.mark_completed(&ticket_c), Ok(()));
        assert!(!tracker.is_completed(&ticket_a));
        assert!(tracker.is_completed(&ticket_b));
        assert!(tracker.is_completed(&ticket_c));
        assert_eq!(tracker.len(), 2);
    }

    #[test]
    fn idempotency_tracker_capacity_one_evicts_on_second_insert() {
        let mut tracker = IdempotencyTracker::with_capacity(1);
        let ticket_a = make_ticket(10);
        let ticket_b = make_ticket(20);
        assert_eq!(tracker.mark_completed(&ticket_a), Ok(()));
        assert_eq!(tracker.mark_completed(&ticket_b), Ok(()));
        assert!(!tracker.is_completed(&ticket_a));
        assert!(tracker.is_completed(&ticket_b));
        assert_eq!(tracker.len(), 1);
    }
}
