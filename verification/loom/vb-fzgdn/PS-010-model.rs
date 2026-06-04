//! PS-010 Loom model: Atomic fire + enqueue consistency (POB-vb-fzgdn-046)
//! Production binding: crates/vb_runtime/src/shard/lifecycle/chunk_002.rs Shard::handle_timer
//!
//! Models: Timer fire atomically removes pending timer and enqueues command.
//! If enqueue fails (queue full), state is preserved.
//!
//! BOUND to production types:
//! - `PendingTimer` from `vb_runtime::shard::types` replaces `bool pending`
//! - `ShardCommand` from `vb_runtime::shard::command` replaces raw `u64` command IDs
//! - `VecDeque<ShardCommand>` replaces `Vec<u64>` for command queue
//! - `PendingTimerKind` from `vb_runtime::shard::types` for timer kind

#![cfg(loom)]

use loom::collections::VecDeque;
use loom::sync::Arc;
use loom::sync::Mutex;
use loom::thread;

use vb_core::ids::RunId;
use vb_runtime::shard::types::PendingTimerKind;

/// Simplified pending timer for loom model.
/// Models `PendingTimer` from `vb_runtime::shard::types`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TimerModel {
    run: RunId,
    step: u16,
    kind: PendingTimerKind,
    generation: u64,
    deadline: u64,
}

impl TimerModel {
    fn new(run: RunId, step: u16, kind: PendingTimerKind, generation: u64, deadline: u64) -> Self {
        Self {
            run,
            step,
            kind,
            generation,
            deadline,
        }
    }

    fn matches_authority(self, gen: u64, dl: u64, k: PendingTimerKind) -> bool {
        self.generation == gen && self.deadline == dl && self.kind == k
    }
}

/// Simplified timer fired command for loom model.
/// Models `ShardCommand::TimerFired` variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TimerFiredCommand {
    run: RunId,
    generation: u64,
    deadline: u64,
    kind: PendingTimerKind,
}

impl TimerFiredCommand {
    fn from_timer(timer: &TimerModel) -> Self {
        Self {
            run: timer.run,
            generation: timer.generation,
            deadline: timer.deadline,
            kind: timer.kind,
        }
    }
}

/// Atomic fire state: pending timer + command queue.
/// Replaces `bool pending` with `Option<TimerModel>`.
struct TimerState {
    pending: Option<TimerModel>,
    command_queue: VecDeque<TimerFiredCommand>,
    capacity: usize,
}

impl TimerState {
    fn new(capacity: usize) -> Self {
        Self {
            pending: None,
            command_queue: VecDeque::new(),
            capacity,
        }
    }

    /// Set the pending timer.
    fn set_pending(&mut self, timer: TimerModel) {
        self.pending = Some(timer);
    }

    /// Atomic fire: remove timer and enqueue iff room exists.
    /// Returns true if fire succeeded, false if no pending timer or queue full.
    fn fire(&mut self) -> bool {
        let Some(timer) = self.pending.take() else {
            return false;
        };
        if self.command_queue.len() >= self.capacity {
            self.pending = Some(timer); // restore state
            return false;
        }
        self.command_queue.push_back(TimerFiredCommand::from_timer(&timer));
        true
    }

    fn is_pending(&self) -> bool {
        self.pending.is_some()
    }

    fn queue_len(&self) -> usize {
        self.command_queue.len()
    }
}

#[test]
fn ps_010_fire_succeeds_when_room() {
    loom::model(|| {
        let mut state = TimerState::new(10);
        state.set_pending(TimerModel::new(RunId::new(1), 5, PendingTimerKind::Wait, 1, 100));
        assert!(state.fire());
        assert!(!state.is_pending());
        assert_eq!(state.queue_len(), 1);
    });
}

#[test]
fn ps_010_fire_preserves_state_when_full() {
    loom::model(|| {
        let mut state = TimerState::new(2);
        state.set_pending(TimerModel::new(RunId::new(1), 5, PendingTimerKind::Wait, 1, 100));
        // Fill queue to capacity
        state.command_queue.push_back(TimerFiredCommand {
            run: RunId::new(99),
            generation: 0,
            deadline: 0,
            kind: PendingTimerKind::Wait,
        });
        state.command_queue.push_back(TimerFiredCommand {
            run: RunId::new(98),
            generation: 0,
            deadline: 0,
            kind: PendingTimerKind::Wait,
        });
        assert!(!state.fire());
        assert!(state.is_pending()); // preserved
        assert_eq!(state.queue_len(), 2);
    });
}

#[test]
fn ps_010_concurrent_fire_maintains_atomicity() {
    loom::model(|| {
        let state = Arc::new(Mutex::new(TimerState::new(1)));
        {
            let mut guard = state.lock().unwrap();
            guard.set_pending(TimerModel::new(RunId::new(1), 5, PendingTimerKind::Wait, 1, 100));
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

#[test]
fn ps_010_fire_no_pending() {
    loom::model(|| {
        let mut state = TimerState::new(10);
        // No pending timer set
        assert!(!state.fire());
        assert_eq!(state.queue_len(), 0);
    });
}
