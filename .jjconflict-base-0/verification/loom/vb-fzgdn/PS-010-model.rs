//! PS-010 Loom model: Atomic fire + enqueue consistency (POB-vb-fzgdn-046)
//! Production binding: crates/vb_runtime/src/shard/lifecycle/chunk_002.rs Shard::handle_timer
//!
//! Models: Timer fire atomically removes pending timer and enqueues command.
//! If enqueue fails (queue full), state is preserved.

#![cfg(loom)]

use loom::sync::Arc;
use loom::sync::Mutex;
use loom::thread;

#[derive(Debug, Clone)]
struct TimerState {
    pending: bool,
    command_queue: Vec<u64>,
    capacity: usize,
}

impl TimerState {
    fn new(capacity: usize) -> Self {
        Self { pending: false, command_queue: Vec::new(), capacity }
    }

    /// Atomic fire: remove timer and enqueue iff room exists.
    fn fire(&mut self) -> bool {
        if !self.pending {
            return false;
        }
        if self.command_queue.len() >= self.capacity {
            return false; // queue full → preserve state
        }
        self.pending = false;
        self.command_queue.push(1);
        true
    }
}

#[test]
fn ps_010_fire_succeeds_when_room() {
    loom::model(|| {
        let mut state = TimerState::new(10);
        state.pending = true;
        assert!(state.fire());
        assert!(!state.pending);
        assert_eq!(state.command_queue.len(), 1);
    });
}

#[test]
fn ps_010_fire_preserves_state_when_full() {
    loom::model(|| {
        let mut state = TimerState::new(2);
        state.pending = true;
        state.command_queue.push(0);
        state.command_queue.push(0); // full at capacity=2
        assert!(!state.fire());
        assert!(state.pending); // preserved
    });
}

#[test]
fn ps_010_concurrent_fire_maintains_atomicity() {
    loom::model(|| {
        let state = Arc::new(Mutex::new(TimerState::new(1)));

        {
            let mut guard = state.lock().unwrap();
            guard.pending = true;
        }

        let s1 = state.clone();
        let t1 = thread::spawn(move || {
            let mut guard = s1.lock().unwrap();
            guard.fire()
        });

        let s2 = state.clone();
        let t2 = thread::spawn(move || {
            let mut guard = s2.lock().unwrap();
            guard.fire()
        });

        let r1 = t1.join().unwrap();
        let r2 = t2.join().unwrap();

        // At most one fire can succeed (capacity=1)
        assert!(!(r1 && r2), "only one fire can succeed when capacity=1");
    });
}
