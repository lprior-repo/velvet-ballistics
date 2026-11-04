#![forbid(unsafe_code)]

//! Bounded idempotency tracker for exactly-once action completion semantics.
//!
//! Tracks dispatched and completed action tickets by their idempotency key.
//! Capacity is bounded via ring-buffer eviction — oldest entries are discarded
//! when the tracker is full, which is safe because the durable journal is the
//! authoritative source of truth for crash recovery.

use std::collections::VecDeque;
#[cfg(kani)]
use std::collections::{BTreeMap as Map, BTreeSet as Set};
#[cfg(not(kani))]
use std::collections::{HashMap as Map, HashSet as Set};

use vb_core::action::{ActionError, ActionTicket, Idempotency};

/// Default capacity for [`IdempotencyTracker::with_default_capacity()`].
const DEFAULT_CAPACITY: usize = 1024;

/// Bounded tracker that records completed action tickets for exactly-once
/// completion detection.
///
/// Uses a map keyed by the ticket's `idempotency_key` for lookups.
/// When capacity is reached, the oldest entry is evicted (FIFO ring buffer).
///
/// ## Policy-aware tracking
///
/// The tracker distinguishes idempotency classes to avoid unnecessary storage:
/// - `DeterministicPure` / `IdempotentExternal`: skip tracking entirely
///   (retry is safe because the action is inherently idempotent or pure)
/// - `AtLeastOnceExternal`: track in `at_least_once_completed` set
///   (retry must be deduplicated by idempotency key)
#[derive(Debug, Clone)]
pub struct IdempotencyTracker {
    /// Map from idempotency key to the completed ticket (for all classes).
    completed: Map<u128, ActionTicket>,
    /// Insertion order for FIFO eviction. Entries are removed here exactly
    /// when their key is evicted from `completed`, so this never outgrows
    /// the map.
    order: Vec<u128>,
    /// Maximum number of entries before eviction.
    capacity: usize,
    /// Position of the next slot to overwrite in `order` during eviction.
    cursor: usize,
    /// Set of idempotency keys for `AtLeastOnceExternal` actions that have
    /// completed. Kept separate so policy decision can be made without
    /// consulting the full `completed` map.
    at_least_once_completed: Set<u128>,
    /// Insertion order for FIFO eviction of `at_least_once_completed`.
    /// Entries are removed here exactly when their key leaves the set, so
    /// the policy set stays within the same capacity bound as the map.
    policy_order: VecDeque<u128>,
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
            completed: Map::new(),
            order: Vec::new(),
            capacity: effective_capacity,
            cursor: 0,
            at_least_once_completed: Set::new(),
            policy_order: VecDeque::new(),
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

    /// Records a successful completion for the given idempotency key and policy.
    ///
    /// For `AtLeastOnceExternal`, also records in the `at_least_once_completed`
    /// set so `is_completed_for_policy` returns `true`. For other policies,
    /// the `at_least_once_completed` set is unchanged.
    ///
    /// Returns `Err(ActionError::CompletionAlreadyRecorded)` if this key was
    /// already marked as completed under this policy.
    pub fn mark_completed_for_policy(
        &mut self,
        policy: Idempotency,
        key: u128,
    ) -> Result<(), ActionError> {
        match policy {
            Idempotency::DeterministicPure | Idempotency::IdempotentExternal => {
                // These policies don't use the at_least_once_completed set.
                // mark_completed is not called for these in practice, but we
                // provide a no-op here for API symmetry.
                Ok(())
            }
            Idempotency::AtLeastOnceExternal => {
                if self.at_least_once_completed.contains(&key) {
                    return Err(ActionError::CompletionAlreadyRecorded);
                }
                self.evict_policy_if_full();
                self.at_least_once_completed.insert(key);
                self.policy_order.push_back(key);
                Ok(())
            }
            _ => Err(ActionError::NonIdempotentReplayBlocked),
        }
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

    /// Tracks a dispatched action under the given idempotency policy.
    ///
    /// Returns `true` if this is a new dispatch (not yet tracked), or `false`
    /// if it is a duplicate for this policy class.
    ///
    /// Policy-specific behaviour:
    /// - `DeterministicPure` / `IdempotentExternal`: always returns `true`,
    ///   does NOT record anything (safe to retry without deduplication).
    /// - `AtLeastOnceExternal`: records the key; returns `false` if already
    ///   seen. The caller MUST check the return value and skip re-dispatch.
    #[must_use]
    pub fn track_for_policy(&mut self, policy: Idempotency, key: u128) -> bool {
        match policy {
            Idempotency::DeterministicPure | Idempotency::IdempotentExternal => {
                // Safe to retry without deduplication — skip tracking.
                true
            }
            Idempotency::AtLeastOnceExternal => {
                // Must deduplicate — record and check for duplicates.
                if self.at_least_once_completed.contains(&key) {
                    false
                } else {
                    self.evict_policy_if_full();
                    self.at_least_once_completed.insert(key);
                    self.policy_order.push_back(key);
                    true
                }
            }
            _ => false,
        }
    }

    /// Returns whether an `AtLeastOnceExternal` action with the given key
    /// has been tracked as completed.
    ///
    /// Always returns `false` for `DeterministicPure` and `IdempotentExternal`
    /// since those are never tracked.
    #[must_use]
    pub fn is_completed_for_policy(&self, policy: Idempotency, key: u128) -> bool {
        match policy {
            Idempotency::DeterministicPure | Idempotency::IdempotentExternal => false,
            Idempotency::AtLeastOnceExternal => self.at_least_once_completed.contains(&key),
            _ => false,
        }
    }

    /// Evicts the oldest entry if the tracker is at or above capacity.
    ///
    /// The evicted key is removed from `order` as well as from `completed`,
    /// so the order log stays exactly as long as the map and tracker memory
    /// remains bounded by capacity.
    fn evict_if_full(&mut self) {
        if self.completed.len() < self.capacity {
            return;
        }
        // We bound iterations to the order length to guarantee termination
        // even if every key has already been removed from the map.
        let max_attempts = self.order.len();
        let mut attempts = 0;
        while attempts < max_attempts {
            attempts = match attempts.checked_add(1) {
                Some(n) => n,
                None => break,
            };
            if self.cursor >= self.order.len() {
                self.cursor = 0;
                continue;
            }
            let index = self.cursor;
            let Some(&key) = self.order.get(index) else {
                let next = index.saturating_add(1);
                self.cursor = if next >= self.order.len() { 0 } else { next };
                continue;
            };
            if self.completed.remove(&key).is_some() {
                self.order.remove(index);
                // Removal shifts later entries left; resume at the entry
                // that followed the evicted key to preserve FIFO order.
                self.cursor = if index < self.order.len() { index } else { 0 };
                return;
            }
            let next = index.saturating_add(1);
            self.cursor = if next >= self.order.len() { 0 } else { next };
        }
    }

    /// Evicts oldest policy keys until `at_least_once_completed` is below
    /// capacity, keeping the policy set within the same bound as the map.
    fn evict_policy_if_full(&mut self) {
        while self.at_least_once_completed.len() >= self.capacity {
            let Some(oldest) = self.policy_order.pop_front() else {
                break;
            };
            self.at_least_once_completed.remove(&oldest);
        }
    }
}

impl Default for IdempotencyTracker {
    fn default() -> Self {
        Self::with_default_capacity()
    }
}

#[cfg(test)]
#[path = "idempotency/tests.rs"]
mod tests;
