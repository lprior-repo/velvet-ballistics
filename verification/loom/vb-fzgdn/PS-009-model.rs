//! PS-009 Loom model: Zero-duration timer concurrent fire (POB-vb-fzgdn-041)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs
//!
//! Models: when deadline == current tick, timer fires immediately without races.
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

/// Timer entry bound to production `TimerEntry`.
#[derive(Debug, Clone, Copy)]
struct TimerEntry {
    run: RunId,
    deadline: u64,
    kind: PendingTimerKind,
}

/// Timer wheel model using HashMap for run-indexed lookup.
struct TimerWheelModel {
    by_run: HashMap<RunId, TimerEntry>,
}

impl TimerWheelModel {
    fn new() -> Self {
        Self {
            by_run: HashMap::new(),
        }
    }

    fn insert(&mut self, run: RunId, deadline: u64, kind: PendingTimerKind) {
        self.by_run.remove(&run);
        self.by_run.insert(
            run,
            TimerEntry {
                run,
                deadline,
                kind,
            },
        );
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
fn ps_009_zero_duration_fires_immediately() {
    loom::model(|| {
        let mut wheel = TimerWheelModel::new();
        wheel.insert(RunId::new(1), 0, PendingTimerKind::Wait);
        let fired = wheel.fire_expired(0);
        assert_eq!(fired.len(), 1);
        assert_eq!(wheel.len(), 0);
    });
}

#[test]
fn ps_009_concurrent_insert_and_fire() {
    loom::model(|| {
        let wheel = Arc::new(Mutex::new(TimerWheelModel::new()));

        let w1 = wheel.clone();
        let t1 = thread::spawn(move || {
            let mut guard = w1.lock().unwrap();
            guard.insert(RunId::new(1), 50, PendingTimerKind::Wait);
        });

        let w2 = wheel.clone();
        let t2 = thread::spawn(move || {
            let mut guard = w2.lock().unwrap();
            guard.fire_expired(100)
        });

        t1.join().unwrap();
        let fired = t2.join().unwrap();
        let guard = wheel.lock().unwrap();

        // Concurrent insert + fire: the timer is observed by exactly one
        // operation regardless of ordering. Both serializations are valid:
        //   1. insert then fire -> fired_len=1, wheel_len=0
        //   2. fire then insert -> fired_len=0, wheel_len=1
        // The mutex serializes the operations, so the sum is always 1 and
        // the wheel is never left in a corrupted or duplicated state.
        assert_eq!(fired.len() + guard.len(), 1);
    });
}

#[test]
fn ps_009_deadline_equal_to_now_fires() {
    loom::model(|| {
        let mut wheel = TimerWheelModel::new();
        wheel.insert(RunId::new(1), 42, PendingTimerKind::Ask);
        wheel.insert(RunId::new(2), 43, PendingTimerKind::Wait);
        // Fire at exactly deadline=42 should include that timer
        let fired = wheel.fire_expired(42);
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].run, RunId::new(1));
        assert_eq!(wheel.len(), 1);
    });
}
