//! Tests for the bounded trace ring.
#![forbid(unsafe_code)]

use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::action::ActionFailureCode;

use crate::trace::{TraceEvent, TraceRing};

#[test]
fn new_creates_with_configured_capacity() {
    let ring = TraceRing::new(8);
    assert_eq!(ring.capacity(), 8);
}

#[test]
fn push_succeeds_when_ring_has_space() {
    let mut ring = TraceRing::new(4);
    let event = TraceEvent::RunSubmitted { run: RunId::new(1) };
    assert_eq!(ring.push(event), true);
    assert_eq!(ring.dropped(), 0);
}

#[test]
fn push_returns_false_when_ring_is_full() {
    let mut ring = TraceRing::new(1);
    let event1 = TraceEvent::RunSubmitted { run: RunId::new(1) };
    let event2 = TraceEvent::RunSubmitted { run: RunId::new(2) };
    assert_eq!(ring.push(event1), true);
    assert_eq!(ring.push(event2), false);
    assert_eq!(ring.dropped(), 1);
}

#[test]
fn drain_returns_all_pushed_events() {
    let mut ring = TraceRing::new(8);
    let e1 = TraceEvent::RunSubmitted { run: RunId::new(1) };
    let e2 = TraceEvent::StepStarted {
        run: RunId::new(1),
        step: StepIdx::new(0),
    };
    let e3 = TraceEvent::StepEnded {
        run: RunId::new(1),
        step: StepIdx::new(0),
    };
    assert_eq!(ring.push(e1.clone()), true);
    assert_eq!(ring.push(e2.clone()), true);
    assert_eq!(ring.push(e3.clone()), true);
    let events = ring.drain();
    assert_eq!(events.len(), 3);
    assert_eq!(events.get(0), Some(&e1));
    assert_eq!(events.get(1), Some(&e2));
    assert_eq!(events.get(2), Some(&e3));
}

#[test]
fn drain_into_respects_limit() {
    let mut ring = TraceRing::new(8);
    for i in 0..5u64 {
        let event = TraceEvent::RunSubmitted { run: RunId::new(i) };
        assert_eq!(ring.push(event), true);
    }
    let mut vec = Vec::new();
    ring.drain_into(2, &mut vec);
    assert_eq!(vec.len(), 2);
}

#[test]
fn drain_for_run_filters_by_run_id() {
    let mut ring = TraceRing::new(16);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::StepStarted {
            run: RunId::new(2),
            step: StepIdx::new(0)
        }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(2) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::StepEnded {
            run: RunId::new(1),
            step: StepIdx::new(0)
        }),
        true
    );
    let events = ring.drain_for_run(RunId::new(2), 10);
    assert_eq!(events.len(), 2);
    assert_eq!(
        events.get(0),
        Some(&TraceEvent::StepStarted {
            run: RunId::new(2),
            step: StepIdx::new(0)
        })
    );
    assert_eq!(
        events.get(1),
        Some(&TraceEvent::RunSubmitted { run: RunId::new(2) })
    );
}

#[test]
fn drain_for_run_returns_empty_for_nonexistent_run() {
    let mut ring = TraceRing::new(8);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    let events = ring.drain_for_run(RunId::new(99), 10);
    assert_eq!(events.len(), 0);
}

#[test]
fn trace_event_run_id_returns_correct_run_for_all_variants() {
    let run = RunId::new(42);
    let step = StepIdx::new(5);
    let slot = SlotIdx::new(3);
    assert_eq!(TraceEvent::StepStarted { run, step }.run_id(), run);
    assert_eq!(TraceEvent::StepEnded { run, step }.run_id(), run);
    assert_eq!(TraceEvent::SlotWritten { run, slot }.run_id(), run);
    assert_eq!(TraceEvent::ActionScheduled { run, step }.run_id(), run);
    assert_eq!(TraceEvent::ActionCompleted { run, step }.run_id(), run);
    assert_eq!(TraceEvent::RunSubmitted { run }.run_id(), run);
    assert_eq!(TraceEvent::RunFinished { run }.run_id(), run);
    assert_eq!(TraceEvent::RunFailed { run }.run_id(), run);
    assert_eq!(TraceEvent::RunCancelled { run }.run_id(), run);
}

#[test]
fn trace_ring_push_then_drain_preserves_order() {
    let mut ring = TraceRing::new(8);
    let e1 = TraceEvent::RunSubmitted { run: RunId::new(1) };
    let e2 = TraceEvent::StepStarted {
        run: RunId::new(1),
        step: StepIdx::new(0),
    };
    let e3 = TraceEvent::StepEnded {
        run: RunId::new(1),
        step: StepIdx::new(0),
    };
    let e4 = TraceEvent::RunFinished { run: RunId::new(1) };
    assert_eq!(ring.push(e1.clone()), true);
    assert_eq!(ring.push(e2.clone()), true);
    assert_eq!(ring.push(e3.clone()), true);
    assert_eq!(ring.push(e4.clone()), true);
    let events = ring.drain();
    assert_eq!(events.len(), 4);
    assert_eq!(events.get(0), Some(&e1));
    assert_eq!(events.get(1), Some(&e2));
    assert_eq!(events.get(2), Some(&e3));
    assert_eq!(events.get(3), Some(&e4));
}

#[test]
fn trace_ring_dropped_increments_on_overflow() {
    let mut ring = TraceRing::new(2);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(2) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(3) }),
        false
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(4) }),
        false
    );
    assert_eq!(ring.dropped(), 2);
}

#[test]
fn trace_ring_drain_returns_empty_after_drain() {
    let mut ring = TraceRing::new(4);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    let first = ring.drain();
    assert_eq!(first.len(), 1);
    let second = ring.drain();
    assert_eq!(second.len(), 0);
}

#[test]
fn trace_event_equality_same_variant_same_fields() {
    let e1 = TraceEvent::ActionScheduled {
        run: RunId::new(5),
        step: StepIdx::new(2),
    };
    let e2 = TraceEvent::ActionScheduled {
        run: RunId::new(5),
        step: StepIdx::new(2),
    };
    assert_eq!(e1, e2);
}

#[test]
fn trace_event_equality_differs_for_different_fields() {
    let e1 = TraceEvent::RunSubmitted { run: RunId::new(1) };
    let e2 = TraceEvent::RunSubmitted { run: RunId::new(2) };
    assert_ne!(e1, e2);
}

#[test]
fn trace_event_clone_preserves_all_fields() {
    let original = TraceEvent::ActionCompleted {
        run: RunId::new(10),
        step: StepIdx::new(3),
    };
    let cloned = original.clone();
    assert_eq!(cloned, original);
}

#[test]
fn trace_ring_drain_into_appends_to_existing_vec() {
    let mut ring = TraceRing::new(4);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(2) }),
        true
    );
    let mut vec = vec![TraceEvent::RunSubmitted { run: RunId::new(0) }];
    ring.drain_into(10, &mut vec);
    assert_eq!(vec.len(), 3);
    assert_eq!(
        vec.get(0),
        Some(&TraceEvent::RunSubmitted { run: RunId::new(0) })
    );
    assert_eq!(
        vec.get(1),
        Some(&TraceEvent::RunSubmitted { run: RunId::new(1) })
    );
    assert_eq!(
        vec.get(2),
        Some(&TraceEvent::RunSubmitted { run: RunId::new(2) })
    );
}

#[test]
fn trace_ring_new_capacity_is_correct() {
    let ring = TraceRing::new(16);
    assert_eq!(ring.capacity(), 16);
}

#[test]
fn trace_ring_dropped_starts_at_zero() {
    let ring = TraceRing::new(4);
    assert_eq!(ring.dropped(), 0);
}

#[test]
fn trace_ring_push_many_events() {
    let mut ring = TraceRing::new(10);
    let mut all_ok = true;
    for i in 0..8u64 {
        if !ring.push(TraceEvent::RunSubmitted { run: RunId::new(i) }) {
            all_ok = false;
        }
    }
    assert_eq!(all_ok, true);
    assert_eq!(ring.dropped(), 0);
    let events = ring.drain();
    assert_eq!(events.len(), 8);
}

#[test]
fn trace_ring_drain_for_run_empty_ring_returns_empty() {
    let mut ring = TraceRing::new(8);
    let events = ring.drain_for_run(RunId::new(1), 10);
    assert_eq!(events.len(), 0);
}

#[test]
fn trace_ring_drain_into_with_zero_limit() {
    let mut ring = TraceRing::new(8);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(2) }),
        true
    );
    let mut vec = Vec::new();
    ring.drain_into(0, &mut vec);
    assert_eq!(vec.len(), 0);
}

#[test]
fn trace_event_run_id_all_variants() {
    let run = RunId::new(42);
    let step = StepIdx::new(1);
    let slot = SlotIdx::new(2);
    assert_eq!(TraceEvent::StepStarted { run, step }.run_id(), run);
    assert_eq!(TraceEvent::StepEnded { run, step }.run_id(), run);
    assert_eq!(TraceEvent::SlotWritten { run, slot }.run_id(), run);
    assert_eq!(TraceEvent::ActionScheduled { run, step }.run_id(), run);
    assert_eq!(TraceEvent::ActionCompleted { run, step }.run_id(), run);
    assert_eq!(TraceEvent::RunSubmitted { run }.run_id(), run);
    assert_eq!(TraceEvent::RunFinished { run }.run_id(), run);
    assert_eq!(TraceEvent::RunFailed { run }.run_id(), run);
    assert_eq!(TraceEvent::RunCancelled { run }.run_id(), run);
}

#[test]
fn trace_event_equality_step_started() {
    let e1 = TraceEvent::StepStarted {
        run: RunId::new(1),
        step: StepIdx::new(0),
    };
    let e2 = TraceEvent::StepStarted {
        run: RunId::new(1),
        step: StepIdx::new(0),
    };
    assert_eq!(e1, e2);
}

#[test]
fn trace_event_equality_step_ended_differs_step() {
    let e1 = TraceEvent::StepEnded {
        run: RunId::new(1),
        step: StepIdx::new(0),
    };
    let e2 = TraceEvent::StepEnded {
        run: RunId::new(1),
        step: StepIdx::new(1),
    };
    assert_ne!(e1, e2);
}

#[test]
fn trace_event_equality_slot_written() {
    let e1 = TraceEvent::SlotWritten {
        run: RunId::new(3),
        slot: SlotIdx::new(5),
    };
    let e2 = TraceEvent::SlotWritten {
        run: RunId::new(3),
        slot: SlotIdx::new(5),
    };
    assert_eq!(e1, e2);
}

#[test]
fn trace_event_equality_run_finished() {
    let e1 = TraceEvent::RunFinished { run: RunId::new(7) };
    let e2 = TraceEvent::RunFinished { run: RunId::new(7) };
    assert_eq!(e1, e2);
}

#[test]
fn trace_event_equality_run_failed_differs_run() {
    let e1 = TraceEvent::RunFailed { run: RunId::new(1) };
    let e2 = TraceEvent::RunFailed { run: RunId::new(2) };
    assert_ne!(e1, e2);
}

#[test]
fn trace_event_equality_run_cancelled() {
    let e1 = TraceEvent::RunCancelled { run: RunId::new(7) };
    let e2 = TraceEvent::RunCancelled { run: RunId::new(7) };
    assert_eq!(e1, e2);
}

#[test]
fn trace_event_different_variants_not_equal() {
    let run = RunId::new(1);
    let e1 = TraceEvent::RunSubmitted { run };
    let e2 = TraceEvent::RunFinished { run };
    assert_ne!(e1, e2);
}

#[test]
fn trace_ring_push_returns_false_at_capacity_boundary() {
    let mut ring = TraceRing::new(3);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(2) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(3) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(4) }),
        false
    );
    assert_eq!(ring.dropped(), 1);
}

#[test]
fn trace_ring_drain_for_run_filters_correctly() {
    let mut ring = TraceRing::new(8);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(2) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunFinished { run: RunId::new(1) }),
        true
    );
    let events = ring.drain_for_run(RunId::new(1), 10);
    assert_eq!(events.len(), 2);
    assert_eq!(
        events.get(0),
        Some(&TraceEvent::RunSubmitted { run: RunId::new(1) })
    );
    assert_eq!(
        events.get(1),
        Some(&TraceEvent::RunFinished { run: RunId::new(1) })
    );
}

#[test]
fn trace_ring_drain_for_run_respects_limit() {
    let mut ring = TraceRing::new(10);
    for i in 0..5u64 {
        assert_eq!(
            ring.push(TraceEvent::StepStarted {
                run: RunId::new(1),
                step: StepIdx::new(i as u16)
            }),
            true
        );
    }
    let events = ring.drain_for_run(RunId::new(1), 3);
    assert_eq!(events.len(), 3);
}

#[test]
fn trace_ring_at_exact_capacity_accepts_all_events_without_drops() {
    let mut ring = TraceRing::new(64);
    for i in 0..64u64 {
        assert_eq!(
            ring.push(TraceEvent::RunSubmitted { run: RunId::new(i) }),
            true
        );
    }
    assert_eq!(ring.dropped(), 0);
    let events = ring.drain();
    assert_eq!(events.len(), 64);
}

#[test]
fn trace_ring_overflow_counts_dropped_events_without_silent_loss() {
    let mut ring = TraceRing::new(4);
    for i in 0..10u64 {
        if i < 4 {
            assert!(ring.push(TraceEvent::RunSubmitted { run: RunId::new(i) }));
        } else {
            assert!(!ring.push(TraceEvent::RunSubmitted { run: RunId::new(i) }));
        }
    }
    assert_eq!(ring.dropped(), 6);
    let events = ring.drain();
    assert_eq!(events.len(), 4);
}

#[test]
fn trace_ring_capacity_one_accepts_one_rejects_second() {
    let mut ring = TraceRing::new(1);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(2) }),
        false
    );
    assert_eq!(ring.dropped(), 1);
    let events = ring.drain();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events.first(),
        Some(&TraceEvent::RunSubmitted { run: RunId::new(1) })
    );
}

#[test]
fn trace_ring_history_only_stores_successfully_pushed_events() {
    let mut ring = TraceRing::new(3);
    for i in 0..5u64 {
        if i < 3 {
            assert!(ring.push(TraceEvent::RunFinished { run: RunId::new(i) }));
        } else {
            assert!(!ring.push(TraceEvent::RunFinished { run: RunId::new(i) }));
        }
    }
    assert_eq!(ring.dropped(), 2);
    let snapshot_0 = ring.snapshot_for_run(RunId::new(0), 10);
    assert_eq!(snapshot_0.len(), 1);
    let snapshot_2 = ring.snapshot_for_run(RunId::new(2), 10);
    assert_eq!(snapshot_2.len(), 1);
    let snapshot_3 = ring.snapshot_for_run(RunId::new(3), 10);
    assert_eq!(snapshot_3.len(), 0);
}

#[test]
fn trace_ring_history_evicts_when_drained_and_refilled() {
    let mut ring = TraceRing::new(2);
    assert_eq!(
        ring.push(TraceEvent::RunFinished { run: RunId::new(0) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunFinished { run: RunId::new(1) }),
        true
    );
    assert_eq!(ring.drain().len(), 2);
    assert_eq!(
        ring.push(TraceEvent::RunFinished { run: RunId::new(2) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunFinished { run: RunId::new(3) }),
        true
    );
    let snap_0 = ring.snapshot_for_run(RunId::new(0), 10);
    let snap_2 = ring.snapshot_for_run(RunId::new(2), 10);
    assert_eq!(snap_0.len(), 0);
    assert_eq!(snap_2.len(), 1);
}

#[test]
fn trace_ring_capacity_zero_rejects_all_events() {
    let mut ring = TraceRing::new(0);
    let result = ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) });
    assert_eq!(result, false);
    assert_eq!(ring.dropped(), 1);
}

#[test]
fn trace_ring_drain_for_run_with_zero_limit_returns_empty_without_consuming() {
    let mut ring = TraceRing::new(8);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(2) }),
        true
    );
    let events = ring.drain_for_run(RunId::new(1), 0);
    assert_eq!(events.len(), 0);
    let remaining = ring.drain();
    assert_eq!(remaining.len(), 2);
}

#[test]
fn trace_ring_fill_drain_fill_drain_alternating_preserves_data() {
    let mut ring = TraceRing::new(4);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(2) }),
        true
    );
    let first = ring.drain();
    assert_eq!(first.len(), 2);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(3) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(4) }),
        true
    );
    let second = ring.drain();
    assert_eq!(second.len(), 2);
    assert_eq!(
        second.get(0),
        Some(&TraceEvent::RunSubmitted { run: RunId::new(3) })
    );
    assert_eq!(
        second.get(1),
        Some(&TraceEvent::RunSubmitted { run: RunId::new(4) })
    );
}

#[test]
fn trace_ring_snapshot_for_run_with_limit_one_returns_at_most_one() {
    let mut ring = TraceRing::new(10);
    for i in 0..5u64 {
        assert_eq!(
            ring.push(TraceEvent::StepStarted {
                run: RunId::new(1),
                step: StepIdx::new(i as u16)
            }),
            true
        );
    }
    let events = ring.snapshot_for_run(RunId::new(1), 1);
    assert_eq!(events.len(), 1);
}

#[test]
fn trace_ring_drain_into_with_limit_exceeding_ring_returns_all() {
    let mut ring = TraceRing::new(8);
    for i in 0..3u64 {
        assert_eq!(
            ring.push(TraceEvent::RunSubmitted { run: RunId::new(i) }),
            true
        );
    }
    let mut vec = Vec::new();
    ring.drain_into(100, &mut vec);
    assert_eq!(vec.len(), 3);
}

#[test]
fn trace_ring_history_survives_ring_drain() {
    let mut ring = TraceRing::new(8);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(2) }),
        true
    );
    assert_eq!(ring.drain().len(), 2);
    let snap = ring.snapshot_for_run(RunId::new(1), 10);
    assert_eq!(snap.len(), 1);
    assert_eq!(
        snap.get(0),
        Some(&TraceEvent::RunSubmitted { run: RunId::new(1) })
    );
}

#[test]
fn trace_ring_action_failed_event_carries_correct_code() {
    let mut ring = TraceRing::new(8);
    let event = TraceEvent::ActionFailed {
        run: RunId::new(42),
        step: StepIdx::new(3),
        code: ActionFailureCode::Timeout,
    };
    assert_eq!(ring.push(event.clone()), true);
    let events = ring.drain();
    assert_eq!(events.len(), 1);
    assert_eq!(events.get(0), Some(&event));
    if let Some(TraceEvent::ActionFailed { code, .. }) = events.get(0) {
        assert_eq!(*code, ActionFailureCode::Timeout);
    }
}
