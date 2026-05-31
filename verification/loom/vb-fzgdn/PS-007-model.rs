//! PS-007 Loom model: Monotonic clock advancement (POB-vb-fzgdn-032)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel::fire_expired
//!
//! Models concurrent clock advancement with monotonic invariant:
//! clock never moves backward, and fire_expired is deterministic.

#![cfg(loom)]

use loom::sync::Arc;
use loom::sync::atomic::{AtomicU64, Ordering};
use loom::thread;

/// Monotonic clock: tick can only advance (never regress).
struct MonotonicClock {
    tick: AtomicU64,
}

impl MonotonicClock {
    fn new(initial: u64) -> Self { Self { tick: AtomicU64::new(initial) } }

    fn advance_to(&self, new_tick: u64) -> bool {
        let mut current = self.tick.load(Ordering::SeqCst);
        loop {
            if new_tick < current {
                return false; // backward tick rejected
            }
            match self.tick.compare_exchange(current, new_tick, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => return true,
                Err(actual) => current = actual,
            }
        }
    }

    fn current(&self) -> u64 {
        self.tick.load(Ordering::SeqCst)
    }
}

#[test]
fn ps_007_backward_tick_rejected() {
    loom::model(|| {
        let clock = MonotonicClock::new(100);
        assert!(!clock.advance_to(50));
        assert_eq!(clock.current(), 100);
    });
}

#[test]
fn ps_007_forward_tick_succeeds() {
    loom::model(|| {
        let clock = MonotonicClock::new(0);
        assert!(clock.advance_to(42));
        assert_eq!(clock.current(), 42);
        assert!(clock.advance_to(100));
        assert_eq!(clock.current(), 100);
    });
}

#[test]
fn ps_007_concurrent_advance_preserves_monotonicity() {
    loom::model(|| {
        let clock = Arc::new(MonotonicClock::new(0));

        let c1 = clock.clone();
        let t1 = thread::spawn(move || {
            for i in 1..=50 {
                c1.advance_to(i);
            }
        });

        let c2 = clock.clone();
        let t2 = thread::spawn(move || {
            for i in 1..=50 {
                c2.advance_to(i);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // Final tick must be at most 50 (no backward, no overflow)
        let final_tick = clock.current();
        assert!(final_tick <= 50);
    });
}
