use vb_core::ids::{RunId, StepIdx};
use vb_runtime::trace::{TraceEvent, TraceRing};

fn make_event(run: RunId, step: StepIdx) -> TraceEvent {
    TraceEvent::StepStarted { run, step }
}

#[test]
fn trace_ring_new_with_zero_capacity_has_one_slot() {
    let ring = TraceRing::new(0);
    assert_eq!(ring.capacity(), 1);
}

#[test]
fn trace_ring_zero_capacity_first_push_succeeds() {
    let mut ring = TraceRing::new(0);
    let event = make_event(RunId::new(1), StepIdx::new(1));
    let result = ring.push(event);
    assert_eq!(result, true);
}

#[test]
fn trace_ring_zero_capacity_pending_and_history_after_push() {
    let mut ring = TraceRing::new(0);
    let event = make_event(RunId::new(1), StepIdx::new(1));
    assert_eq!(ring.push(event), true);
    assert_eq!(ring.pending_len(), 1);
    assert_eq!(ring.history_len(), 1);
}

#[test]
fn trace_ring_zero_capacity_drain_returns_event() {
    let mut ring = TraceRing::new(0);
    let event = make_event(RunId::new(1), StepIdx::new(1));
    assert_eq!(ring.push(event.clone()), true);
    let drained = ring.drain();
    assert_eq!(drained, vec![event]);
}

#[test]
fn trace_ring_zero_capacity_drain_clears_pending_keeps_history() {
    let mut ring = TraceRing::new(0);
    let event = make_event(RunId::new(1), StepIdx::new(1));
    assert_eq!(ring.push(event), true);
    let drained = ring.drain();
    assert_eq!(drained.len(), 1);
    assert_eq!(ring.pending_is_empty(), true);
    assert_eq!(ring.history_len(), 1);
}

#[test]
fn trace_ring_zero_capacity_refill_after_drain() {
    let mut ring = TraceRing::new(0);
    let event1 = make_event(RunId::new(1), StepIdx::new(1));
    assert_eq!(ring.push(event1), true);
    let drained = ring.drain();
    assert_eq!(drained.len(), 1);
    let event2 = make_event(RunId::new(2), StepIdx::new(1));
    let result = ring.push(event2.clone());
    assert_eq!(result, true);
    assert_eq!(ring.pending_len(), 1);
    assert_eq!(ring.history_len(), 1);
    let drained = ring.drain();
    assert_eq!(drained, vec![event2.clone()]);
    assert_eq!(ring.pending_is_empty(), true);
    let snapshot = ring.snapshot_for_run(RunId::new(2), 1);
    assert_eq!(snapshot, vec![event2]);
}

#[test]
fn trace_ring_zero_capacity_overflow_drops_second_event() {
    let mut ring = TraceRing::new(0);
    let event1 = make_event(RunId::new(1), StepIdx::new(1));
    assert_eq!(ring.push(event1.clone()), true);
    let event2 = make_event(RunId::new(2), StepIdx::new(1));
    let result = ring.push(event2);
    assert_eq!(result, false);
    assert_eq!(ring.dropped(), 1);
    assert_eq!(ring.pending_len(), 1);
    assert_eq!(ring.history_len(), 1);
    let drained = ring.drain();
    assert_eq!(drained, vec![event1.clone()]);
    let snap1 = ring.snapshot_for_run(RunId::new(1), 1);
    assert_eq!(snap1, vec![event1]);
    let snap2 = ring.snapshot_for_run(RunId::new(2), 1);
    assert_eq!(snap2, Vec::<TraceEvent>::new());
}
