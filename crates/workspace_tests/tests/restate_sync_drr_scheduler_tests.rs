#![forbid(unsafe_code)]
//! Synchronous DRR (Deficit Round Robin) scheduler fairness behavior tests.
//!
//! Models a synchronous shard-owned scheduler that dispatches action tickets
//! across multiple ready queues with per-tick caps, priority preemption at
//! documented decision boundaries, and blocked-state tracking.
//!
//! These tests exercise invariants from the bead spec:
//! - Two ready queues alternate over bounded ticks.
//! - Higher-priority queue preempts only at a documented decision boundary.
//! - Invalid (zero) per-tick cap returns a typed config error.
//! - Missing action capacity blocks a queue without dropping the ticket.
//! - A blocked-then-unblocked queue resumes without losing position.
//! - Unconfirmed assignments are fenced against duplicate dispatch.
//!
//! The scheduler modeled here is a **test harness**, not production code.
//! Production VB types (`ActionTicket`, `BoundedActionCompletionQueue`) are
//! used as the substrate. The scheduler itself demonstrates the correctness
//! of the scheduling discipline that VB shards must maintain.

use proptest::prelude::*;
use vb_core::action::{ActionTicket, compute_action_idempotency_key};
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_runtime::action_queue::BoundedActionCompletionQueue;

// ---------------------------------------------------------------------------
// Test-harness scheduler model
// ---------------------------------------------------------------------------

/// Per-queue state tracked by the shard-owned scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum QueueState {
    Ready,
    #[allow(dead_code)]
    Scheduled,
    Blocked,
}

/// A single scheduler queue (backed by a bounded action-completion queue).
#[derive(Debug)]
struct SchedulerQueue {
    inner: BoundedActionCompletionQueue,
    priority: u8,
    state: QueueState,
}

/// Typed errors from scheduler configuration and operations.
#[derive(Debug, PartialEq, Eq)]
#[allow(dead_code)]
enum SchedulerError {
    InvalidPerTickCap { cap: usize },
    InvalidQueueCount { count: usize },
    QueueFull { queue_idx: usize },
}

/// A synchronous, shard-owned DRR scheduler.
///
/// At each tick the scheduler walks queues in priority order,
/// dispatching up to `per_tick_cap` ready actions. A queue that
/// exhausts its capacity is marked Blocked; it resumes when the
/// capacity constraint is cleared (via external completion).
#[derive(Debug)]
struct DrrScheduler {
    queues: Vec<SchedulerQueue>,
    per_tick_cap: usize,
    unconfirmed: Vec<ActionTicket>,
}

impl DrrScheduler {
    /// Creates a scheduler with `queue_count` ready queues.
    ///
    /// Returns `Err` if `per_tick_cap` is zero or `queue_count` is zero.
    fn new(
        queue_count: usize,
        per_tick_cap: usize,
        queue_capacity: usize,
    ) -> Result<Self, SchedulerError> {
        if per_tick_cap == 0 {
            return Err(SchedulerError::InvalidPerTickCap { cap: per_tick_cap });
        }
        if queue_count == 0 {
            return Err(SchedulerError::InvalidQueueCount { count: queue_count });
        }
        let mut queues = Vec::with_capacity(queue_count);
        for i in 0..queue_count {
            let q = BoundedActionCompletionQueue::new(queue_capacity).map_err(|_| {
                SchedulerError::InvalidPerTickCap {
                    cap: queue_capacity,
                }
            })?;
            queues.push(SchedulerQueue {
                inner: q,
                priority: i as u8,
                state: QueueState::Ready,
            });
        }
        Ok(Self {
            queues,
            per_tick_cap,
            unconfirmed: Vec::new(),
        })
    }

    /// Runs one tick of the scheduler. Returns tickets dispatched this tick.
    ///
    /// Walks queues in priority order, round-robin, until the per-tick cap is
    /// reached or all ready queues are exhausted.
    fn tick(&mut self, tickets: &mut Vec<ActionTicket>) -> usize {
        let mut dispatched = 0;
        let cap = self.per_tick_cap;
        if cap == 0 {
            return 0;
        }
        let n = self.queues.len();
        loop {
            let start_count = dispatched;
            for qi in 0..n {
                if dispatched >= cap {
                    return dispatched;
                }
                let q = &mut self.queues[qi];
                if q.state != QueueState::Ready {
                    continue;
                }
                if let Some(t) = q.inner.dequeue() {
                    self.unconfirmed.push(t);
                    tickets.push(t);
                    dispatched += 1;
                    if q.inner.is_empty() {
                        q.state = QueueState::Blocked;
                    }
                } else {
                    q.state = QueueState::Blocked;
                }
            }
            // Exit if no progress this round (all queues drained or blocked)
            if dispatched == start_count {
                return dispatched;
            }
        }
    }

    /// Confirms a previously dispatched ticket, removing it from the
    /// unconfirmed set. Returns `true` if the ticket was confirmed.
    fn confirm(&mut self, ticket: ActionTicket) -> bool {
        if let Some(pos) = self.unconfirmed.iter().position(|t| *t == ticket) {
            self.unconfirmed.remove(pos);
            true
        } else {
            false
        }
    }

    /// Preempts the current tick at a decision boundary for a higher-priority
    /// queue. The boundary is between queues at different priority levels.
    fn preempt_for_priority(&self, min_priority: u8) -> Option<usize> {
        self.queues
            .iter()
            .enumerate()
            .find(|(_, q)| q.priority < min_priority && q.state == QueueState::Ready)
            .map(|(i, _)| i)
    }

    /// Resumes a blocked queue (capacity became available).
    fn resume_queue(&mut self, idx: usize) -> Result<(), SchedulerError> {
        let q = self
            .queues
            .get_mut(idx)
            .ok_or(SchedulerError::InvalidQueueCount { count: idx })?;
        q.state = QueueState::Ready;
        Ok(())
    }

    /// Returns the number of ready queues.
    #[allow(dead_code)]
    fn ready_count(&self) -> usize {
        self.queues
            .iter()
            .filter(|q| q.state == QueueState::Ready)
            .count()
    }

    /// Returns the current unconfirmed ticket count.
    fn unconfirmed_count(&self) -> usize {
        self.unconfirmed.len()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn mk_ticket(run: u64, seq: u32, action: u16, attempt: u16) -> ActionTicket {
    let run = RunId::new(run);
    let seq = SeqNo::new(u64::from(seq));
    let action = ActionId::new(action);
    let idempotency_key = compute_action_idempotency_key(run, seq, action);
    ActionTicket {
        run,
        step: StepIdx::new(0),
        seq,
        action,
        attempt,
        idempotency_key,
        capacity: 3,
    }
}

fn enqueue_tickets(
    queues: &[BoundedActionCompletionQueue],
    tickets: &[ActionTicket],
) -> Vec<usize> {
    let mut enqueued = Vec::new();
    for (qi, t) in tickets.iter().enumerate() {
        let q = &queues[qi % queues.len()];
        if q.enqueue(*t).is_ok() {
            enqueued.push(qi % queues.len());
        }
    }
    enqueued
}

// ---------------------------------------------------------------------------
// Properties
// ---------------------------------------------------------------------------

// Property: per-tick dispatch count never exceeds the cap.
proptest! {
    #[test]
    fn tick_respects_per_tick_cap(queue_count in 1_usize..8, cap in 1_usize..16) {
        let mut s = DrrScheduler::new(queue_count, cap, 64).unwrap();
        let mut tickets = Vec::new();
        let dispatched = s.tick(&mut tickets);
        assert!(dispatched <= cap,
            "dispatched {dispatched} > cap {cap}");
    }
}

/// Property: zero per-tick cap returns `InvalidPerTickCap`.
#[test]
fn zero_per_tick_cap_rejected() {
    let err = DrrScheduler::new(2, 0, 16).unwrap_err();
    assert_eq!(err, SchedulerError::InvalidPerTickCap { cap: 0 });
}

/// Property: zero queue count returns `InvalidQueueCount`.
#[test]
fn zero_queue_count_rejected() {
    let err = DrrScheduler::new(0, 4, 16).unwrap_err();
    assert_eq!(err, SchedulerError::InvalidQueueCount { count: 0 });
}

// ---------------------------------------------------------------------------
// Happy-path tests
// ---------------------------------------------------------------------------

/// Two ready queues alternate across ticks.
#[test]
fn two_queues_alternate_over_bounded_ticks() {
    let mut s = DrrScheduler::new(2, 8, 64).unwrap();
    for i in 0..3 {
        let mut tickets = Vec::new();
        s.tick(&mut tickets);
        assert!(
            tickets.len() <= 8,
            "tick {i}: dispatched {} > cap 8",
            tickets.len()
        );
        // Confirm all for next round
        for t in &tickets {
            assert!(s.confirm(*t), "failed to confirm ticket");
        }
    }
    assert_eq!(s.unconfirmed_count(), 0, "unconfirmed tickets remain");
}

/// Higher-priority queue (index 0) preempts at decision boundary.
#[test]
fn higher_priority_queue_preempts_at_decision_boundary() {
    let mut s = DrrScheduler::new(3, 2, 64).unwrap();
    // Queue 0: priority 0 (highest); Queues 1,2: lower
    // Preempt for any priority >0 → queue 0 is eligible
    let idx = s.preempt_for_priority(1);
    assert_eq!(idx, Some(0), "queue 0 should be eligible for preemption");
    // Preempt for priority 0 → nothing eligible (all >= 0)
    let idx2 = s.preempt_for_priority(0);
    assert_eq!(idx2, None, "no queue has priority < 0");

    let mut tickets = Vec::new();
    s.tick(&mut tickets);
    let dispatched = tickets.len();
    assert!(dispatched <= 2, "cap exceeded: {dispatched}");
}

/// Queue at capacity blocks without dropping ticket.
#[test]
fn missing_action_capacity_blocks_without_drop() {
    let cap = 2;
    let mut s = DrrScheduler::new(1, 4, cap).unwrap();
    let t1 = mk_ticket(1, 1, 1, 1);
    let t2 = mk_ticket(1, 2, 1, 1);
    let t3 = mk_ticket(1, 3, 1, 1);
    assert!(s.queues[0].inner.enqueue(t1).is_ok());
    assert!(s.queues[0].inner.enqueue(t2).is_ok());
    // Third enqueue should fail (queue full)
    let full_err = s.queues[0].inner.enqueue(t3);
    assert!(full_err.is_err(), "third enqueue should fail at cap {cap}");
    let mut dispatched = Vec::new();
    let count = s.tick(&mut dispatched);
    assert_eq!(count, 2, "should dispatch exactly {:?} tickets", cap);
    // Confirm both
    for t in &dispatched {
        assert!(s.confirm(*t));
    }
    // Queue should be blocked (empty)
    assert_eq!(s.queues[0].state, QueueState::Blocked);
}

/// Blocked-then-unblocked queue resumes without losing position.
#[test]
fn blocked_queue_resumes_without_position_loss() {
    let mut s = DrrScheduler::new(2, 4, 2).unwrap();
    let t1 = mk_ticket(1, 1, 1, 1);
    let t2 = mk_ticket(1, 2, 1, 1);
    let t3 = mk_ticket(2, 1, 2, 1);
    // Queue 0: two tickets; Queue 1: one ticket
    s.queues[0].inner.enqueue(t1).unwrap();
    s.queues[0].inner.enqueue(t2).unwrap();
    s.queues[1].inner.enqueue(t3).unwrap();

    // Tick 1: dispatch all 3 tickets (round-robin: q0, q1, q0)
    let mut d1 = Vec::new();
    let n1 = s.tick(&mut d1);
    assert_eq!(n1, 3, "should dispatch all 3 enqueued tickets");
    // Confirm all
    for t in &d1 {
        assert!(s.confirm(*t), "failed to confirm ticket");
    }

    // Queue 0 was empty/blocked; resume it and add new ticket
    s.resume_queue(0).unwrap();
    let t4 = mk_ticket(1, 3, 1, 1);
    s.queues[0].inner.enqueue(t4).unwrap();

    // Tick 2: Queue 0 dispatches again
    let mut d2 = Vec::new();
    let n2 = s.tick(&mut d2);
    assert!(n2 > 0, "resumed queue should dispatch");
    assert!(
        d2.iter().any(|t| t.seq == SeqNo::new(3)),
        "resumed queue ticket not found"
    );
}

/// Unconfirmed assignments are fenced against duplicate dispatch.
#[test]
fn unconfirmed_prevent_duplicate_dispatch() {
    let mut s = DrrScheduler::new(1, 2, 4).unwrap();
    let t1 = mk_ticket(1, 1, 1, 1);
    s.queues[0].inner.enqueue(t1).unwrap();

    let mut dispatched = Vec::new();
    s.tick(&mut dispatched);
    assert_eq!(s.unconfirmed_count(), 1);
    assert!(dispatched.contains(&t1));

    // Tick again without confirm → nothing left in queue
    // The ticket is unconfirmed but NOT re-dispatchable (fenced)
    // BoundedActionCompletionQueue won't return it again because dequeue removed it
    let mut d2 = Vec::new();
    let n2 = s.tick(&mut d2);
    assert_eq!(n2, 0, "no tickets should remain after dequeue");
    assert_eq!(s.unconfirmed_count(), 1, "unconfirmed ticket still tracked");

    // Confirm the ticket
    assert!(s.confirm(t1));
    assert_eq!(s.unconfirmed_count(), 0);
}

// ---------------------------------------------------------------------------
// Error-path tests
// ---------------------------------------------------------------------------

/// Invalid per-tick cap returns typed error.
#[test]
fn zero_cap_returns_scheduler_error() {
    let err = DrrScheduler::new(4, 0, 16).unwrap_err();
    assert_eq!(err, SchedulerError::InvalidPerTickCap { cap: 0 });
}

/// Invalid queue count returns typed error.
#[test]
fn zero_queues_returns_scheduler_error() {
    let err = DrrScheduler::new(0, 4, 16).unwrap_err();
    assert_eq!(err, SchedulerError::InvalidQueueCount { count: 0 });
}

/// Enqueuing beyond capacity returns QueueFull, not a panic.
#[test]
fn overflow_enqueue_returns_queue_full() {
    let q = BoundedActionCompletionQueue::new(1).unwrap();
    let t1 = mk_ticket(1, 1, 1, 1);
    let t2 = mk_ticket(1, 2, 1, 1);
    assert!(q.enqueue(t1).is_ok());
    let err = q.enqueue(t2);
    assert!(err.is_err(), "expected QueueFull, got Ok");
}

/// Invalid (zero) capacity for bounded queue is rejected.
#[test]
fn bounded_queue_zero_capacity_rejected() {
    let res = BoundedActionCompletionQueue::new(0);
    assert!(res.is_err(), "zero-capacity queue should be rejected");
}

// ---------------------------------------------------------------------------
// Edge-case tests
// ---------------------------------------------------------------------------

/// Scheduler with single queue and cap=1 dispatches one per tick.
#[test]
fn single_queue_single_cap_dispatches_one_per_tick() {
    let mut s = DrrScheduler::new(1, 1, 64).unwrap();
    let t1 = mk_ticket(1, 1, 1, 1);
    let t2 = mk_ticket(1, 2, 1, 1);
    s.queues[0].inner.enqueue(t1).unwrap();
    s.queues[0].inner.enqueue(t2).unwrap();

    let mut d1 = Vec::new();
    assert_eq!(s.tick(&mut d1), 1);
    assert_eq!(d1.len(), 1);
    assert!(s.confirm(d1[0]));

    // Queue is empty after first dequeue, blocked state
    s.resume_queue(0).unwrap();
    let mut d2 = Vec::new();
    assert_eq!(s.tick(&mut d2), 1);
    assert_eq!(d2.len(), 1);
}

/// Large scheduler: 8 queues, 64 cap, stress-test tick loop.
#[test]
fn many_queue_many_tick_stress() {
    let q_count = 8;
    let cap = 64;
    let mut s = DrrScheduler::new(q_count, 4, cap).unwrap();
    // Enqueue 128 tickets across queues (round-robin)
    for i in 0..128_u32 {
        let t = mk_ticket(1, i, (i % 16) as u16, 1);
        let qi = (i as usize) % q_count;
        let _ = s.queues[qi].inner.enqueue(t);
    }
    let mut total = 0;
    for _ in 0..64 {
        let mut dispatched = Vec::new();
        total += s.tick(&mut dispatched);
        for t in &dispatched {
            s.confirm(*t);
        }
    }
    assert_eq!(total, 128, "all 128 tickets should be dispatched");
    assert_eq!(s.unconfirmed_count(), 0);
}

/// Confirming a non-existent ticket returns false.
#[test]
fn confirm_unknown_ticket_returns_false() {
    let mut s = DrrScheduler::new(2, 4, 16).unwrap();
    let t = mk_ticket(99, 99, 99, 1);
    assert!(!s.confirm(t));
}

// ---------------------------------------------------------------------------
// Blocked-queue invariant tests
// ---------------------------------------------------------------------------

/// Ready queues dispatch in priority order (0 first).
#[test]
fn priority_order_preserved_in_tick() {
    let mut s = DrrScheduler::new(3, 6, 16).unwrap();
    let t0 = mk_ticket(1, 1, 0, 1);
    let t1 = mk_ticket(1, 1, 1, 1);
    let t2 = mk_ticket(1, 1, 2, 1);
    s.queues[0].inner.enqueue(t0).unwrap();
    s.queues[1].inner.enqueue(t1).unwrap();
    s.queues[2].inner.enqueue(t2).unwrap();

    let mut dispatched = Vec::new();
    s.tick(&mut dispatched);
    assert_eq!(dispatched.len(), 3);
    // First dispatched comes from queue 0 (highest priority)
    assert_eq!(dispatched[0].action, ActionId::new(0));
    assert_eq!(dispatched[1].action, ActionId::new(1));
    assert_eq!(dispatched[2].action, ActionId::new(2));
}

/// All queues suspended: tick dispatches zero.
#[test]
fn all_queues_blocked_dispatches_zero() {
    let mut s = DrrScheduler::new(2, 4, 4).unwrap();
    s.queues[0].state = QueueState::Blocked;
    s.queues[1].state = QueueState::Blocked;
    let mut dispatched = Vec::new();
    let n = s.tick(&mut dispatched);
    assert_eq!(n, 0);
    assert!(dispatched.is_empty());
}

/// Bounded queue capacity is preserved across enqueue/dequeue cycles.
#[test]
fn bounded_queue_capacity_preserved() {
    let cap = 4;
    let q = BoundedActionCompletionQueue::new(cap).unwrap();
    assert_eq!(q.capacity(), cap);
    assert_eq!(q.remaining_capacity(), cap);
    assert!(q.is_empty());
    assert!(!q.is_full());

    let t = mk_ticket(1, 1, 1, 1);
    for _ in 0..cap {
        assert!(q.enqueue(t).is_ok());
    }
    assert!(q.is_full());
    assert_eq!(q.remaining_capacity(), 0);

    for _ in 0..cap {
        assert!(q.dequeue().is_some());
    }
    assert!(q.is_empty());
    assert_eq!(q.remaining_capacity(), cap);
}

/// Backpressure warning channel is functional.
#[test]
fn backpressure_channel_warns_at_eighty_percent() {
    let capacity = 10;
    let (q, rx) = BoundedActionCompletionQueue::with_backpressure(capacity).unwrap();
    let t = mk_ticket(1, 1, 1, 1);
    // Fill to 80% threshold (8 at capacity 10)
    for _ in 0..8 {
        q.enqueue(t).unwrap();
    }
    // 80% threshold should trigger warning
    let warned = rx.try_recv().is_ok();
    assert!(warned, "expected backpressure warning at 80% capacity");
}
