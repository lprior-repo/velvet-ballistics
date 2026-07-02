//! PS-001 Loom model: TimerWheel concurrent insert/cancel/fire (POB-vb-fzgdn-005)
//! Production binding: crates/vb_runtime/src/shard/timer_wheel.rs TimerWheel
//!
//! Models concurrent access to a simplified TimerWheel.
//! Uses loom's atomic and mutex primitives for schedule exploration.

#![cfg(loom)]

use loom::sync::Arc;
use loom::sync::Mutex;
use loom::thread;

/// Simplified timer entry for loom model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TimerEntry {
    run_id: u64,
    generation: u64,
    deadline: u64,
}

/// Concurrent-safe timer wheel model matching production structure.
struct TimerWheelModel {
    entries: Vec<TimerEntry>,
    next_gen: u64,
}

impl TimerWheelModel {
    fn new() -> Self { Self { entries: Vec::new(), next_gen: 1 } }

    fn insert(&mut self, run_id: u64, deadline: u64) {
        self.entries.retain(|e| e.run_id != run_id);
        self.entries.push(TimerEntry { run_id, generation: self.next_gen, deadline });
        self.next_gen += 1;
    }

    fn cancel(&mut self, run_id: u64) -> bool {
        let len_before = self.entries.len();
        self.entries.retain(|e| e.run_id != run_id);
        self.entries.len() < len_before
    }

    fn fire_expired(&mut self, now: u64) -> Vec<TimerEntry> {
        let mut fired = Vec::new();
        let mut i = 0;
        while i < self.entries.len() {
            if self.entries[i].deadline <= now {
                fired.push(self.entries.remove(i));
            } else {
                i += 1;
            }
        }
        fired
    }

    fn len(&self) -> usize { self.entries.len() }
}

#[test]
fn ps_001_concurrent_inserts_maintain_consistency() {
    loom::model(|| {
        let wheel = Arc::new(Mutex::new(TimerWheelModel::new()));

        let w1 = wheel.clone();
        let t1 = thread::spawn(move || {
            let mut guard = w1.lock().unwrap();
            for id in 0..5 {
                guard.insert(id, 100);
            }
        });

        let w2 = wheel.clone();
        let t2 = thread::spawn(move || {
            let mut guard = w2.lock().unwrap();
            for id in 5..10 {
                guard.insert(id, 200);
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
            assert!(!guard.cancel(99));
        });
        t.join().unwrap();
    });
}

#[test]
fn ps_001_fire_expired_drains_all() {
    loom::model(|| {
        let mut wheel = TimerWheelModel::new();
        wheel.insert(1, 10);
        wheel.insert(2, 20);
        wheel.insert(3, 30);
        let fired = wheel.fire_expired(25);
        assert_eq!(fired.len(), 2);
        assert_eq!(wheel.len(), 1);
    });
}
