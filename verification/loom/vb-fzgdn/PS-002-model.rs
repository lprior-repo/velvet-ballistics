//! PS-002 Loom model: PendingTimer matches_authority concurrent reads (POB-vb-fzgdn-010)
//! Production binding: crates/vb_runtime/src/shard/types.rs PendingTimer::matches_authority
//!
//! Models concurrent reads of PendingTimer state; matches_authority is a pure
//! read operation that must be consistent under concurrent access.
//!
//! BOUND to production types:
//! - `PendingTimerKind` from `vb_runtime::shard::types` for Wait/Ask variant
//! - `std::time::Instant` modeled as `u64` ticks for loom determinism
//! - Generation field modeled with loom atomic for concurrent access

#![cfg(loom)]

use loom::sync::Arc;
use loom::sync::atomic::{AtomicU64, Ordering};
use loom::thread;

use vb_runtime::shard::types::PendingTimerKind;

/// Concurrent-safe timer authority data using atomics.
/// Models the internal state of `PendingTimer`:
/// - generation: u64 freshness token
/// - deadline: u64 tick value (bound to std::time::Instant)
/// - kind: PendingTimerKind
struct TimerAuthority {
    generation: AtomicU64,
    deadline: u64,
    kind: PendingTimerKind,
}

impl TimerAuthority {
    fn new(generation: u64, deadline: u64, kind: PendingTimerKind) -> Self {
        Self {
            generation: AtomicU64::new(generation),
            deadline,
            kind,
        }
    }
}

/// Model of matches_authority check using atomic reads.
/// Matches the production `PendingTimer::matches_authority` semantics:
/// all three fields must match for the timer to be considered valid.
fn matches_authority(auth: &TimerAuthority, expected_gen: u64, expected_deadline: u64, expected_kind: PendingTimerKind) -> bool {
    let current = auth.generation.load(Ordering::SeqCst);
    current == expected_gen && auth.deadline == expected_deadline && auth.kind == expected_kind
}

#[test]
fn ps_002_concurrent_authority_reads_consistent() {
    loom::model(|| {
        let auth = Arc::new(TimerAuthority::new(42, 100, PendingTimerKind::Wait));

        let a1 = auth.clone();
        let t1 = thread::spawn(move || {
            for _ in 0..100 {
                let result = matches_authority(&a1, 42, 100, PendingTimerKind::Wait);
                assert!(result);
            }
        });

        let a2 = auth.clone();
        let t2 = thread::spawn(move || {
            for _ in 0..100 {
                let result = matches_authority(&a2, 99, 100, PendingTimerKind::Wait);
                assert!(!result);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();
    });
}

#[test]
fn ps_002_stale_generation_detected() {
    loom::model(|| {
        let auth = TimerAuthority::new(5, 50, PendingTimerKind::Ask);
        // Simulate generation change by storing new value
        auth.generation.store(10, Ordering::SeqCst);
        assert!(!matches_authority(&auth, 5, 50, PendingTimerKind::Ask));
        assert!(matches_authority(&auth, 10, 50, PendingTimerKind::Ask));
    });
}

#[test]
fn ps_002_kind_mismatch_detected() {
    loom::model(|| {
        let auth = TimerAuthority::new(5, 50, PendingTimerKind::Wait);
        assert!(matches_authority(&auth, 5, 50, PendingTimerKind::Wait));
        assert!(!matches_authority(&auth, 5, 50, PendingTimerKind::Ask));
    });
}
