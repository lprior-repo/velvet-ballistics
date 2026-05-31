//! PS-002 Loom model: PendingTimer matches_authority concurrent reads (POB-vb-fzgdn-010)
//! Production binding: crates/vb_runtime/src/shard/types.rs PendingTimer::matches_authority
//!
//! Models concurrent reads of PendingTimer state; matches_authority is a pure
//! read operation that must be consistent under concurrent access.

#![cfg(loom)]

use loom::sync::Arc;
use loom::sync::atomic::{AtomicU64, Ordering};
use loom::thread;

/// Concurrent-safe timer authority data using atomics.
struct TimerAuthority {
    generation: AtomicU64,
}

/// Model of matches_authority check using atomic reads.
fn matches_authority(auth: &TimerAuthority, expected_gen: u64) -> bool {
    let current = auth.generation.load(Ordering::SeqCst);
    current == expected_gen
}

#[test]
fn ps_002_concurrent_authority_reads_consistent() {
    loom::model(|| {
        let auth = Arc::new(TimerAuthority { generation: AtomicU64::new(42) });

        let a1 = auth.clone();
        let t1 = thread::spawn(move || {
            for _ in 0..100 {
                let result = matches_authority(&a1, 42);
                assert!(result);
            }
        });

        let a2 = auth.clone();
        let t2 = thread::spawn(move || {
            for _ in 0..100 {
                let result = matches_authority(&a2, 99);
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
        let auth = TimerAuthority { generation: AtomicU64::new(5) };
        auth.generation.store(10, Ordering::SeqCst);
        assert!(!matches_authority(&auth, 5));
        assert!(matches_authority(&auth, 10));
    });
}
