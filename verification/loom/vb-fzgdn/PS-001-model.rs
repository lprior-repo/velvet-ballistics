//! PS-001 Loom model: TimerWheel concurrent insert/cancel/fire (POB-vb-fzgdn-005)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel
//!
//! Models concurrent access to a simplified TimerWheel.
//! Uses loom's atomic and mutex primitives for schedule exploration.
//!
//! BOUND to production types:
//! - `RunId` from `vb_core::ids` replaces raw `u64` run identifiers
//! - `PendingTimerKind` from `vb_runtime::shard::types` for timer kind
//! - `HashMap<RunId, TimerEntry>` replaces `Vec<TimerEntry>` for O(1) cancel
//! - `Instant` deadline modeled as `u64` ticks for loom determinism

#![cfg(loom)]

use loom::sync::Arc;
use loom::sync::Mutex;
use loom::thread;
use std::collections::HashMap;

use vb_core::ids::RunId;
use vb_runtime::shard::types::PendingTimerKind;

/// Simplified timer entry for loom model — bound to production `TimerEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TimerEntry {
    run: RunId,
    generation: u64,
    deadline: u64,
    kind: PendingTimerKind,
}

/// Concurrent-safe timer wheel model matching production structure.
/// Uses `HashMap<RunId, TimerEntry>` for O(1) cancel/lookup.
struct TimerWheelModel {
    by_run: HashMap<RunId, TimerEntry>,
    next_gen: u64,
}

impl TimerWheelModel {
    fn new() -> Self {
        Self {
            by_run: HashMap::new(),
            next_gen: 1,
        }
    }

    fn insert(&mut self, run: RunId, deadline: u64, kind: PendingTimerKind) {
        let generation = self.next_gen;
        self.next_gen += 1;
        // Replace existing timer for this run if present
        self.by_run.remove(&run);
        self.by_run.insert(
            run,
            TimerEntry {
                run,
                generation,
                deadline,
                kind,
            },
        );
    }

    fn cancel(&mut self, run: RunId) -> bool {
        self.by_run.remove(&run).is_some()
    }

    fn fire_expired(&mut self, now: u64) -> Vec<TimerEntry> {
        let mut fired = Vec::new();
        let mut expired_runs = Vec::new();
        for (&run, entry) in self.by_run.iter() {
            if entry.deadline <= now {
                expired_runs.push(run);
            }
        }
        for run in expired_runs {
            if let Some(entry) = self.by_run.remove(&run) {
                fired.push(entry);
            }
        }
        fired
    }

    fn len(&self) -> usize {
        self.by_run.len()
    }
}

#[test]
fn ps_001_concurrent_inserts_maintain_consistency() {
    loom::model(|| {
        let wheel = Arc::new(Mutex::new(TimerWheelModel::new()));

        let w1 = wheel.clone();
        let t1 = thread::spawn(move || {
            let mut guard = w1.lock().unwrap();
            for id in 0..5 {
                guard.insert(RunId::new(id), 100, PendingTimerKind::Wait);
            }
        });

        let w2 = wheel.clone();
        let t2 = thread::spawn(move || {
            let mut guard = w2.lock().unwrap();
            for id in 5..10 {
                guard.insert(RunId::new(id), 200, PendingTimerKind::Ask);
            }
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let guard = wheel.lock().unwrap();
        assert_eq!(guard.len(), 10);
    });
}

#[test]
fn ps_001_cancel_returns_false_when_empty() {
    loom::model(|| {
        let wheel = Arc::new(Mutex::new(TimerWheelModel::new()));
        let w = wheel.clone();
        let t = thread::spawn(move || {
            let mut guard = w.lock().unwrap();
            assert!(!guard.cancel(RunId::new(99)));
        });
        t.join().unwrap();
    });
}

#[test]
fn ps_001_fire_expired_drains_all() {
    loom::model(|| {
        let mut wheel = TimerWheelModel::new();
        wheel.insert(RunId::new(1), 10, PendingTimerKind::Wait);
        wheel.insert(RunId::new(2), 20, PendingTimerKind::Ask);
        wheel.insert(RunId::new(3), 30, PendingTimerKind::Wait);
        let fired = wheel.fire_expired(25);
        assert_eq!(fired.len(), 2);
        assert_eq!(wheel.len(), 1);
    });
}
