//! PS-009 Loom model: Zero-duration timer concurrent fire (POB-vb-fzgdn-041)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs
//!
//! Models: when deadline == current tick, timer fires immediately without races.

#![cfg(loom)]

use loom::sync::Arc;
use loom::sync::Mutex;
use loom::thread;

#[derive(Debug, Clone, Copy)]
struct TimerEntry {
    run_id: u64,
    deadline: u64,
}

struct TimerWheelModel {
    entries: Vec<TimerEntry>,
}

impl TimerWheelModel {
    fn new() -> Self { Self { entries: Vec::new() } }

    fn insert(&mut self, run_id: u64, deadline: u64) {
        self.entries.retain(|e| e.run_id != run_id);
        self.entries.push(TimerEntry { run_id, deadline });
    }

    fn fire_expired(&mut self, now: u64) -> Vec<TimerEntry> {
        let mut fired = Vec::new();
        self.entries.retain(|e| {
            if e.deadline <= now {
                fired.push(*e);
                false
            } else {
                true
            }
        });
        fired
    }

    fn len(&self) -> usize { self.entries.len() }
}

#[test]
fn ps_009_zero_duration_fires_immediately() {
    loom::model(|| {
        let mut wheel = TimerWheelModel::new();
        wheel.insert(1, 0);
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
            guard.insert(1, 50);
        });

        let w2 = wheel.clone();
        let t2 = thread::spawn(move || {
            let mut guard = w2.lock().unwrap();
            let _ = guard.fire_expired(100);
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let guard = wheel.lock().unwrap();
        // If inserted at deadline=50 and fired at now=100, entry should be gone
        assert_eq!(guard.len(), 0);
    });
}
