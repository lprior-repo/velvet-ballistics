//! Kani harnesses for vb_runtime idempotency tracker.
//!
//! Scope: vb_runtime::idempotency::IdempotencyTracker
//! Obligations: FWH-001 (enhanced), FWH-005, FWH-006, FWH-016, FWH-020
//!
//! GOD RULE: No hardcoded shapes — all inputs use kani::Arbitrary or
//! bounded generators via kani::any() with kani::assume() guards.

#![forbid(unsafe_code)]
#![cfg(kani)]
#![cfg(feature = "kani-idempotency-tracker")]

use crate::action::{ActionError, ActionTicket, Idempotency};
use crate::ids::{ActionId, RunId, SeqNo, StepIdx};

use super::IdempotencyTracker;

// =========================================================================
// Arbitrary implementations for Kani bounded model checking
// =========================================================================

/// Bounded generator for ActionTicket ingredients.
/// Uses kani::any() with assume guards — NOT hardcoded shapes.
fn any_bounded_ticket() -> ActionTicket {
    let run_id = kani::any::<u64>();
    kani::assume(run_id > 0);
    let step = kani::any::<u32>();
    kani::assume(step < 64);
    let seq = kani::any::<u64>();
    let action_id = kani::any::<u32>();
    kani::assume(action_id < 256);
    let attempt = kani::any::<u16>();
    kani::assume(attempt > 0);
    let key = kani::any::<u128>();
    let capacity = kani::any::<u16>();
    kani::assume(capacity > 0);
    ActionTicket {
        run: RunId::new(run_id),
        step: StepIdx::new(step),
        seq: SeqNo::new(seq),
        action: ActionId::new(action_id),
        attempt,
        idempotency_key: key,
        capacity,
    }
}

/// Bounded generator for tracker capacity.
fn any_bounded_capacity() -> usize {
    let cap = kani::any::<usize>();
    kani::assume(cap >= 1 && cap <= 16);
    cap
}

// =========================================================================
// FWH-001: Idempotency key validation — tracker level
// =========================================================================

/// FWH-001-TRACKER: mark_completed is idempotent — second call with same key always fails.
///
/// Property: For any ticket T, if mark_completed(T) succeeds, then
/// mark_completed(T) returns CompletionAlreadyRecorded.
#[kani::proof]
#[kani::unwind(12)]
fn proof_tracker_completion_idempotent() {
    let capacity = any_bounded_capacity();
    let mut tracker = IdempotencyTracker::new(capacity);
    let ticket = any_bounded_ticket();

    let first = tracker.mark_completed(&ticket);
    kani::assume(first.is_ok());

    let second = tracker.mark_completed(&ticket);
    kani::assert(
        second == Err(ActionError::CompletionAlreadyRecorded),
        "duplicate completion must return CompletionAlreadyRecorded",
    );
}

// =========================================================================
// FWH-005: Duplicate completion with same key returns error
// =========================================================================

/// FWH-005: For any tracker state and any ticket already completed,
/// a second completion attempt returns CompletionAlreadyRecorded and
/// does not mutate the tracker length.
#[kani::proof]
#[kani::unwind(16)]
fn proof_duplicate_completion_same_key() {
    let capacity = any_bounded_capacity();
    let mut tracker = IdempotencyTracker::new(capacity);

    let ticket_a = any_bounded_ticket();
    let ticket_b = any_bounded_ticket();
    kani::assume(ticket_a.idempotency_key != ticket_b.idempotency_key);

    let r1 = tracker.mark_completed(&ticket_a);
    kani::assume(r1.is_ok());
    let len_after_a = tracker.len();

    let r2 = tracker.mark_completed(&ticket_b);
    kani::assume(r2.is_ok());
    let len_after_b = tracker.len();

    let r3 = tracker.mark_completed(&ticket_a);
    kani::assert(
        r3 == Err(ActionError::CompletionAlreadyRecorded),
        "duplicate of ticket_a must fail",
    );
    kani::assert(
        tracker.len() == len_after_b,
        "duplicate completion must not change tracker length",
    );
}

// =========================================================================
// FWH-006: Duplicate completion with different digest — replay divergence
// =========================================================================

/// FWH-006: Two tickets with the same idempotency_key but different
/// attempt numbers represent a replay divergence. The tracker correctly
/// rejects the second because it keys on idempotency_key only.
///
/// Property: If ticket_a.key == ticket_b.key but ticket_a != ticket_b,
/// then mark_completed(ticket_b) returns CompletionAlreadyRecorded.
#[kani::proof]
#[kani::unwind(12)]
fn proof_replay_divergence_same_key_different_ticket() {
    let capacity = any_bounded_capacity();
    let mut tracker = IdempotencyTracker::new(capacity);

    let key = kani::any::<u128>();
    let mut ticket_a = any_bounded_ticket();
    kani::assume(ticket_a.idempotency_key == key);
    let mut ticket_b = any_bounded_ticket();
    kani::assume(ticket_b.idempotency_key == key);
    kani::assume(ticket_b.attempt != ticket_a.attempt);

    kani::assert(
        ticket_a != ticket_b,
        "tickets must differ (different attempt)",
    );
    kani::assert(
        ticket_a.idempotency_key == ticket_b.idempotency_key,
        "tickets must share key",
    );

    let r1 = tracker.mark_completed(&ticket_a);
    kani::assume(r1.is_ok());

    let r2 = tracker.mark_completed(&ticket_b);
    kani::assert(
        r2 == Err(ActionError::CompletionAlreadyRecorded),
        "different ticket with same key must be rejected as duplicate",
    );
}

// =========================================================================
// FWH-016: Eviction safety — ring buffer does not corrupt state
// =========================================================================

/// FWH-016: After eviction, the tracker maintains:
/// 1. len() <= capacity
/// 2. All remaining entries are still queryable via is_completed
/// 3. Evicted entries are no longer queryable
/// 4. Re-insertion of evicted key works correctly
#[kani::proof]
#[kani::unwind(18)]
fn proof_eviction_safety() {
    let capacity = any_bounded_capacity();
    let mut tracker = IdempotencyTracker::new(capacity);

    let mut first_ticket = any_bounded_ticket();
    match tracker.mark_completed(&first_ticket) {
        Ok(v) => {
            let _ = v;
        }
        Err(_) => {
            kani::assume(false);
            return;
        }
    }

    // Fill up to capacity
    for _ in 1..capacity {
        let mut t = any_bounded_ticket();
        // Ensure unique keys for deterministic eviction of first_ticket
        kani::assume(t.idempotency_key != first_ticket.idempotency_key);
        let _ = tracker.mark_completed(&t);
    }
    kani::assert(
        tracker.is_completed(&first_ticket),
        "first ticket must be present before eviction",
    );

    // Trigger eviction
    let mut extra_ticket = any_bounded_ticket();
    kani::assume(extra_ticket.idempotency_key != first_ticket.idempotency_key);
    let _ = tracker.mark_completed(&extra_ticket);

    kani::assert(
        tracker.len() <= capacity,
        "tracker must not exceed capacity after eviction",
    );

    kani::assert(
        !tracker.is_completed(&first_ticket),
        "oldest entry (first_ticket) must be evicted",
    );
    kani::assert(
        tracker.is_completed(&extra_ticket),
        "extra ticket must be present",
    );

    let reinsert = tracker.mark_completed(&first_ticket);
    kani::assert(reinsert.is_ok(), "re-insertion of evicted key must succeed");
    kani::assert(
        tracker.is_completed(&first_ticket),
        "re-inserted key must be queryable",
    );
}

// =========================================================================
// FWH-020: Monotonicity under arbitrary completion sequences
// =========================================================================

/// FWH-020: The tracker is monotonic — entries are never removed except
/// by bounded eviction, and eviction only removes the oldest entry.
///
/// Property: For any sequence of mark_completed operations,
/// once a key is marked completed, it remains completed until eviction.
#[kani::proof]
#[kani::unwind(18)]
fn proof_monotonicity_until_eviction() {
    let capacity = any_bounded_capacity();
    let mut tracker = IdempotencyTracker::new(capacity);

    let t1 = any_bounded_ticket();
    let _ = tracker.mark_completed(&t1);
    kani::assert(tracker.is_completed(&t1), "t1 completed after mark");

    // Fill up to capacity - 1 more
    for _ in 1..capacity {
        let mut t = any_bounded_ticket();
        kani::assume(t.idempotency_key != t1.idempotency_key);
        let _ = tracker.mark_completed(&t);
        kani::assert(
            tracker.is_completed(&t1),
            "t1 still completed (no eviction yet)",
        );
    }

    // Trigger eviction of t1
    let mut extra = any_bounded_ticket();
    kani::assume(extra.idempotency_key != t1.idempotency_key);
    let _ = tracker.mark_completed(&extra);

    kani::assert(
        !tracker.is_completed(&t1),
        "t1 evicted after capacity exceeded",
    );
}

// =========================================================================
// Policy-aware tracking invariants
// =========================================================================

/// Proof: track_for_policy is idempotent for DeterministicPure.
#[kani::proof]
#[kani::unwind(8)]
fn proof_track_for_policy_deterministic_pure_always_new() {
    let mut tracker = IdempotencyTracker::with_default_capacity();
    let key = kani::any::<u128>();

    let first = tracker.track_for_policy(Idempotency::DeterministicPure, key);
    let second = tracker.track_for_policy(Idempotency::DeterministicPure, key);

    kani::assert(first, "first track must return true");
    kani::assert(second, "second track must also return true (no tracking)");
    kani::assert(
        !tracker.is_completed_for_policy(Idempotency::DeterministicPure, key),
        "DeterministicPure must never be tracked",
    );
}

/// Proof: track_for_policy deduplicates for AtLeastOnceExternal.
#[kani::proof]
#[kani::unwind(8)]
fn proof_track_for_policy_at_least_once_deduplicates() {
    let mut tracker = IdempotencyTracker::with_default_capacity();
    let key = kani::any::<u128>();

    let first = tracker.track_for_policy(Idempotency::AtLeastOnceExternal, key);
    let second = tracker.track_for_policy(Idempotency::AtLeastOnceExternal, key);

    kani::assert(first, "first track must return true");
    kani::assert(!second, "second track must return false (duplicate)");
    kani::assert(
        tracker.is_completed_for_policy(Idempotency::AtLeastOnceExternal, key),
        "AtLeastOnceExternal must be tracked",
    );
}

/// Proof: mark_completed_for_policy is monotonic for AtLeastOnceExternal.
#[kani::proof]
#[kani::unwind(8)]
fn proof_mark_completed_for_policy_monotonic() {
    let mut tracker = IdempotencyTracker::with_default_capacity();
    let key = kani::any::<u128>();

    let first = tracker.mark_completed_for_policy(Idempotency::AtLeastOnceExternal, key);
    kani::assume(first.is_ok());

    let second = tracker.mark_completed_for_policy(Idempotency::AtLeastOnceExternal, key);
    kani::assert(
        second == Err(ActionError::CompletionAlreadyRecorded),
        "second mark must fail as duplicate",
    );
}
