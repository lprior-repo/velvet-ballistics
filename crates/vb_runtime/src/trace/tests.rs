#![forbid(unsafe_code)]

use crate::trace::{RunId, SlotIdx, StepIdx, TraceEvent, TraceRing};
use vb_core::action::ActionFailureCode;

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
fn len_and_is_empty_track_pending_ring_events() {
    let mut ring = TraceRing::new(4);
    assert_eq!(ring.len(), 0);
    assert_eq!(ring.is_empty(), true);

    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(2) }),
        true
    );

    assert_eq!(ring.len(), 2);
    assert_eq!(ring.is_empty(), false);

    let mut drained = Vec::new();
    ring.drain_into(1, &mut drained);
    assert_eq!(ring.len(), 1);
    assert_eq!(ring.is_empty(), false);

    let remaining = ring.drain();
    assert_eq!(remaining.len(), 1);
    assert_eq!(ring.len(), 0);
    assert_eq!(ring.is_empty(), true);
}

#[test]
fn len_and_is_empty_ignore_retained_snapshot_history_after_drain() {
    let mut ring = TraceRing::new(4);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(7) }),
        true
    );

    let drained = ring.drain();
    assert_eq!(drained.len(), 1);

    let snapshot = ring.snapshot_for_run(RunId::new(7), 4);
    assert_eq!(snapshot.len(), 1);
    assert_eq!(ring.len(), 0);
    assert_eq!(ring.is_empty(), true);
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
fn drain_for_run_preserves_non_target_events() {
    let mut ring = TraceRing::new(8);
    let run_one_start = TraceEvent::RunSubmitted { run: RunId::new(1) };
    let run_two = TraceEvent::RunSubmitted { run: RunId::new(2) };
    let run_one_finish = TraceEvent::RunFinished { run: RunId::new(1) };
    assert_eq!(ring.push(run_one_start.clone()), true);
    assert_eq!(ring.push(run_two.clone()), true);
    assert_eq!(ring.push(run_one_finish.clone()), true);

    assert_eq!(ring.drain_for_run(RunId::new(2), 8), vec![run_two]);
    assert_eq!(ring.drain(), vec![run_one_start, run_one_finish]);
}

#[test]
fn trace_event_run_id_returns_correct_run_for_all_variants() {
    let run = RunId::new(42);
    let step = StepIdx::new(5);
    let slot = SlotIdx::new(3);
    assert_eq!(TraceEvent::StepStarted { run, step }.run_id(), run);
    assert_eq!(TraceEvent::StepEnded { run, step }.run_id(), run);
    assert_eq!(
        TraceEvent::SlotWritten {
            run,
            slot,
            value: vec![]
        }
        .run_id(),
        run
    );
    assert_eq!(TraceEvent::ActionScheduled { run, step }.run_id(), run);
    assert_eq!(TraceEvent::ActionCompleted { run, step }.run_id(), run);
    assert_eq!(TraceEvent::RunSubmitted { run }.run_id(), run);
    assert_eq!(TraceEvent::RunFinished { run }.run_id(), run);
    assert_eq!(TraceEvent::RunFailed { run }.run_id(), run);
    assert_eq!(TraceEvent::RunCancelled { run }.run_id(), run);
}

#[test]
fn trace_ring_push_then_drain_preserves_order() {
    // Given a trace ring with 8 slots
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
    // When pushing 4 events and draining
    assert_eq!(ring.push(e1.clone()), true);
    assert_eq!(ring.push(e2.clone()), true);
    assert_eq!(ring.push(e3.clone()), true);
    assert_eq!(ring.push(e4.clone()), true);
    let events = ring.drain();
    // Then the order is preserved (FIFO)
    assert_eq!(events.len(), 4);
    assert_eq!(events.get(0), Some(&e1));
    assert_eq!(events.get(1), Some(&e2));
    assert_eq!(events.get(2), Some(&e3));
    assert_eq!(events.get(3), Some(&e4));
}

#[test]
fn trace_ring_dropped_increments_on_overflow() {
    // Given a ring with capacity 2
    let mut ring = TraceRing::new(2);
    // When pushing 4 events
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
    // Then dropped count is 2
    assert_eq!(ring.dropped(), 2);
}

#[test]
fn trace_ring_drain_returns_empty_after_drain() {
    // Given a ring with one event
    let mut ring = TraceRing::new(4);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    // When draining
    let first = ring.drain();
    assert_eq!(first.len(), 1);
    // Then a second drain returns empty
    let second = ring.drain();
    assert_eq!(second.len(), 0);
}

#[test]
fn trace_event_equality_same_variant_same_fields() {
    // Given two identical trace events
    let e1 = TraceEvent::ActionScheduled {
        run: RunId::new(5),
        step: StepIdx::new(2),
    };
    let e2 = TraceEvent::ActionScheduled {
        run: RunId::new(5),
        step: StepIdx::new(2),
    };
    // Then they are equal
    assert_eq!(e1, e2);
}

#[test]
fn trace_event_equality_differs_for_different_fields() {
    // Given two trace events with different run IDs
    let e1 = TraceEvent::RunSubmitted { run: RunId::new(1) };
    let e2 = TraceEvent::RunSubmitted { run: RunId::new(2) };
    // Then they are not equal
    assert_ne!(e1, e2);
}

#[test]
fn trace_event_clone_preserves_all_fields() {
    // Given a trace event
    let original = TraceEvent::ActionCompleted {
        run: RunId::new(10),
        step: StepIdx::new(3),
    };
    // When cloning
    let cloned = original.clone();
    // Then the clone is equal
    assert_eq!(cloned, original);
}

#[test]
fn trace_ring_drain_into_appends_to_existing_vec() {
    // Given a ring with 2 events and a vec with 1 existing event
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
    // When draining into the vec
    ring.drain_into(10, &mut vec);
    // Then the vec has 3 events (1 existing + 2 new)
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
    // Given a new trace ring with capacity 16
    let ring = TraceRing::new(16);
    // When checking capacity
    // Then it is 16
    assert_eq!(ring.capacity(), 16);
}

#[test]
fn trace_ring_dropped_starts_at_zero() {
    // Given a new trace ring
    let ring = TraceRing::new(4);
    // When checking dropped count
    // Then it is 0
    assert_eq!(ring.dropped(), 0);
}

#[test]
fn trace_ring_push_many_events() {
    // Given a ring with capacity 10
    let mut ring = TraceRing::new(10);
    // When pushing 8 events
    let mut all_ok = true;
    for i in 0..8u64 {
        if !ring.push(TraceEvent::RunSubmitted { run: RunId::new(i) }) {
            all_ok = false;
        }
    }
    // Then all pushes succeed
    assert_eq!(all_ok, true);
    assert_eq!(ring.dropped(), 0);
    // And draining returns 8 events
    let events = ring.drain();
    assert_eq!(events.len(), 8);
}

#[test]
fn trace_ring_drain_for_run_empty_ring_returns_empty() {
    // Given an empty ring
    let mut ring = TraceRing::new(8);
    // When draining for a specific run
    let events = ring.drain_for_run(RunId::new(1), 10);
    // Then result is empty
    assert_eq!(events.len(), 0);
}

#[test]
fn trace_ring_drain_into_with_zero_limit() {
    // Given a ring with events
    let mut ring = TraceRing::new(8);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(2) }),
        true
    );
    // When draining with limit 0
    let mut vec = Vec::new();
    ring.drain_into(0, &mut vec);
    // Then no events are drained
    assert_eq!(vec.len(), 0);
}

#[test]
fn trace_event_run_id_all_variants() {
    // Given trace events for a specific run
    let run = RunId::new(42);
    let step = StepIdx::new(1);
    let slot = SlotIdx::new(2);
    // When checking run_id for each variant
    // Then they all return the correct run
    assert_eq!(TraceEvent::StepStarted { run, step }.run_id(), run);
    assert_eq!(TraceEvent::StepEnded { run, step }.run_id(), run);
    assert_eq!(
        TraceEvent::SlotWritten {
            run,
            slot,
            value: vec![]
        }
        .run_id(),
        run
    );
    assert_eq!(TraceEvent::ActionScheduled { run, step }.run_id(), run);
    assert_eq!(TraceEvent::ActionCompleted { run, step }.run_id(), run);
    assert_eq!(TraceEvent::RunSubmitted { run }.run_id(), run);
    assert_eq!(TraceEvent::RunFinished { run }.run_id(), run);
    assert_eq!(TraceEvent::RunFailed { run }.run_id(), run);
    assert_eq!(TraceEvent::RunCancelled { run }.run_id(), run);
}

#[test]
fn trace_event_equality_step_started() {
    // Given two identical StepStarted events
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
    // Given two StepEnded events with different steps
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
    // Given two identical SlotWritten events
    let e1 = TraceEvent::SlotWritten {
        run: RunId::new(3),
        slot: SlotIdx::new(5),
        value: vec![],
    };
    let e2 = TraceEvent::SlotWritten {
        run: RunId::new(3),
        slot: SlotIdx::new(5),
        value: vec![],
    };
    assert_eq!(e1, e2);
}

#[test]
fn trace_event_equality_run_finished() {
    // Given two identical RunFinished events
    let e1 = TraceEvent::RunFinished { run: RunId::new(7) };
    let e2 = TraceEvent::RunFinished { run: RunId::new(7) };
    assert_eq!(e1, e2);
}

#[test]
fn trace_event_equality_run_failed_differs_run() {
    // Given two RunFailed events with different runs
    let e1 = TraceEvent::RunFailed { run: RunId::new(1) };
    let e2 = TraceEvent::RunFailed { run: RunId::new(2) };
    assert_ne!(e1, e2);
}

#[test]
fn trace_event_equality_run_cancelled() {
    // Given two identical RunCancelled events
    let e1 = TraceEvent::RunCancelled { run: RunId::new(7) };
    let e2 = TraceEvent::RunCancelled { run: RunId::new(7) };
    assert_eq!(e1, e2);
}

#[test]
fn trace_event_different_variants_not_equal() {
    // Given two events with same run but different variants
    let run = RunId::new(1);
    let e1 = TraceEvent::RunSubmitted { run };
    let e2 = TraceEvent::RunFinished { run };
    assert_ne!(e1, e2);
}

#[test]
fn trace_ring_push_returns_false_at_capacity_boundary() {
    // Given a ring with capacity 3
    let mut ring = TraceRing::new(3);
    // When filling to capacity
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
    // Then the next push fails
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(4) }),
        false
    );
    assert_eq!(ring.dropped(), 1);
}

#[test]
fn trace_ring_drain_for_run_filters_correctly() {
    // Given a ring with events for runs 1, 2, 1
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
    // When draining for run 1
    let events = ring.drain_for_run(RunId::new(1), 10);
    // Then only run 1 events are returned
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
    // Given a ring with 5 events for run 1
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
    // When draining with limit 3
    let events = ring.drain_for_run(RunId::new(1), 3);
    // Then only 3 events are returned
    assert_eq!(events.len(), 3);
}

// =========================================================================
// Phase 2 adversarial BDD tests — trace ring overflow & capacity vectors
// =========================================================================

// --- Trace ring filled to exact capacity accepts all events ---

#[test]
fn trace_ring_at_exact_capacity_accepts_all_events_without_drops() {
    // Given a trace ring with capacity 64
    let mut ring = TraceRing::new(64);
    // When pushing exactly 64 events
    for i in 0..64u64 {
        assert_eq!(
            ring.push(TraceEvent::RunSubmitted { run: RunId::new(i) }),
            true
        );
    }
    // Then no drops and all events are present
    assert_eq!(ring.dropped(), 0);
    let events = ring.drain();
    assert_eq!(events.len(), 64);
}

// --- Trace ring overflow increments dropped counter accurately ---

#[test]
fn trace_ring_overflow_counts_dropped_events_without_silent_loss() {
    // Given a ring with capacity 4
    let mut ring = TraceRing::new(4);
    // When pushing 10 events (6 overflow)
    for i in 0..10u64 {
        if i < 4 {
            assert!(ring.push(TraceEvent::RunSubmitted { run: RunId::new(i) }));
        } else {
            assert!(!ring.push(TraceEvent::RunSubmitted { run: RunId::new(i) }));
        }
    }
    // Then dropped count is exactly 6 (not silently lost)
    assert_eq!(ring.dropped(), 6);
    // And the ring still has 4 events
    let events = ring.drain();
    assert_eq!(events.len(), 4);
}

// --- Trace ring with capacity 1 accepts exactly one event ---

#[test]
fn trace_ring_capacity_one_accepts_one_rejects_second() {
    // Given a ring with capacity 1
    let mut ring = TraceRing::new(1);
    // When pushing two events
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(2) }),
        false
    );
    // Then only one event is retained
    assert_eq!(ring.dropped(), 1);
    let events = ring.drain();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events.first(),
        Some(&TraceEvent::RunSubmitted { run: RunId::new(1) })
    );
}

// --- Trace ring history evicts oldest when exceeding capacity ---

#[test]
fn trace_ring_history_only_stores_successfully_pushed_events() {
    // Given a ring with capacity 3
    let mut ring = TraceRing::new(3);
    // When pushing 5 events (3 succeed, 2 overflow)
    for i in 0..5u64 {
        if i < 3 {
            assert!(ring.push(TraceEvent::RunFinished { run: RunId::new(i) }));
        } else {
            assert!(!ring.push(TraceEvent::RunFinished { run: RunId::new(i) }));
        }
    }
    // Then history contains the 3 successfully pushed events (0, 1, 2)
    assert_eq!(ring.dropped(), 2);
    let snapshot_0 = ring.snapshot_for_run(RunId::new(0), 10);
    assert_eq!(snapshot_0.len(), 1);
    let snapshot_2 = ring.snapshot_for_run(RunId::new(2), 10);
    assert_eq!(snapshot_2.len(), 1);
    // Events that overflowed (3, 4) are not in history
    let snapshot_3 = ring.snapshot_for_run(RunId::new(3), 10);
    assert_eq!(snapshot_3.len(), 0);
}

#[test]
fn trace_ring_history_evicts_when_drained_and_refilled() {
    // Given a ring with capacity 2
    let mut ring = TraceRing::new(2);
    // When filling with events 0, 1, draining, then filling with events 2, 3
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
    // Then history has all 4 events (capacity is 2 but remember stores up to capacity)
    // Actually history grows unbounded by remember but is capped at capacity
    let snap_0 = ring.snapshot_for_run(RunId::new(0), 10);
    let snap_2 = ring.snapshot_for_run(RunId::new(2), 10);
    // After drain + refill, history evicts oldest when exceeding capacity
    assert_eq!(snap_0.len(), 0);
    assert_eq!(snap_2.len(), 1);
}

// --- Drain for run with zero limit returns empty ---

#[test]
fn trace_ring_drain_for_run_with_zero_limit_returns_empty_without_consuming() {
    // Given a ring with events
    let mut ring = TraceRing::new(8);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(2) }),
        true
    );
    // When draining with limit 0
    let events = ring.drain_for_run(RunId::new(1), 0);
    // Then result is empty but ring still has events
    assert_eq!(events.len(), 0);
    let remaining = ring.drain();
    assert_eq!(remaining.len(), 2);
}

// =======================================================================
// Adversarial BDD tests - trace ring attack vectors
// =======================================================================

#[test]
fn trace_ring_fill_drain_fill_drain_alternating_preserves_data() {
    // Given a ring with capacity 4
    let mut ring = TraceRing::new(4);
    // When filling, draining, then filling again
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
    // Then the second drain contains only the new events
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
    // Given a ring with 5 events for run 1
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
    // When snapshotting with limit 1
    let events = ring.snapshot_for_run(RunId::new(1), 1);
    // Then exactly 1 event is returned
    assert_eq!(events.len(), 1);
}

#[test]
fn trace_ring_drain_into_with_limit_exceeding_ring_returns_all() {
    // Given a ring with 3 events
    let mut ring = TraceRing::new(8);
    for i in 0..3u64 {
        assert_eq!(
            ring.push(TraceEvent::RunSubmitted { run: RunId::new(i) }),
            true
        );
    }
    // When draining with limit 100
    let mut vec = Vec::new();
    ring.drain_into(100, &mut vec);
    // Then all 3 events are returned (drain_into stops at ring exhaustion)
    assert_eq!(vec.len(), 3);
}

#[test]
fn trace_ring_history_survives_ring_drain() {
    // Given a ring with 2 events
    let mut ring = TraceRing::new(8);
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }),
        true
    );
    assert_eq!(
        ring.push(TraceEvent::RunSubmitted { run: RunId::new(2) }),
        true
    );
    // When draining the ring completely
    assert_eq!(ring.drain().len(), 2);
    // Then history still allows snapshot queries
    let snap = ring.snapshot_for_run(RunId::new(1), 10);
    assert_eq!(snap.len(), 1);
    assert_eq!(
        snap.get(0),
        Some(&TraceEvent::RunSubmitted { run: RunId::new(1) })
    );
}

#[test]
fn trace_ring_action_failed_event_carries_correct_code() {
    // Given a trace ring
    let mut ring = TraceRing::new(8);
    // When pushing an ActionFailed event
    let event = TraceEvent::ActionFailed {
        run: RunId::new(42),
        step: StepIdx::new(3),
        code: ActionFailureCode::Timeout,
    };
    assert_eq!(ring.push(event.clone()), true);
    // Then draining returns the exact event with the correct code
    let events = ring.drain();
    assert_eq!(events.len(), 1);
    assert_eq!(events.get(0), Some(&event));
    if let Some(TraceEvent::ActionFailed { code, .. }) = events.get(0) {
        assert_eq!(*code, ActionFailureCode::Timeout);
    }
}

// =========================================================================
// OBL-TRC-005: adversarial_overflow — push returns false and dropped
//               increments atomically when ring is full.
// =========================================================================

#[test]
fn adversarial_overflow() {
    // Given a ring with capacity 1 to stress the overflow path
    let mut ring = TraceRing::new(1);
    let event1 = TraceEvent::RunSubmitted { run: RunId::new(1) };
    let event2 = TraceEvent::RunSubmitted { run: RunId::new(2) };

    // When pushing first event — succeeds
    let result1 = ring.push(event1);
    assert_eq!(result1, true, "first push must succeed");
    assert_eq!(ring.dropped(), 0, "no drops yet");

    // When pushing second event — ring is full, push returns false
    let result2 = ring.push(event2);
    assert_eq!(result2, false, "push must return false when full");
    assert_eq!(
        ring.dropped(),
        1,
        "dropped counter increments atomically on overflow"
    );

    // The second event was not retained (overflow policy: drop oldest)
    let events = ring.drain();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events.first(),
        Some(&TraceEvent::RunSubmitted { run: RunId::new(1) }),
        "only the first event is retained after overflow"
    );
}

// =========================================================================
// OBL-TRC-006: fifo_ordering — events dequeued in FIFO insertion order.
// =========================================================================

#[test]
fn fifo_ordering() {
    // Given a trace ring with capacity 8
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

    // When pushing 4 events
    assert_eq!(ring.push(e1.clone()), true);
    assert_eq!(ring.push(e2.clone()), true);
    assert_eq!(ring.push(e3.clone()), true);
    assert_eq!(ring.push(e4.clone()), true);

    // Then draining returns events in exact FIFO insertion order
    let events = ring.drain();
    assert_eq!(events.len(), 4, "all 4 events are drained");
    assert_eq!(events.get(0), Some(&e1), "e1 is first (earliest)");
    assert_eq!(events.get(1), Some(&e2), "e2 is second");
    assert_eq!(events.get(2), Some(&e3), "e3 is third");
    assert_eq!(events.get(3), Some(&e4), "e4 is fourth (latest)");
}

// =========================================================================
// OBL-TRC-007: Miri UB check — belt-and-suspenders (trace.rs is
//              #![forbid(unsafe_code)] but we verify no UB via Miri).
// =========================================================================

#[cfg(miri)]
#[test]
fn miri_trace_operations_no_ub() {
    // Given a trace ring with capacity 8
    let mut ring = TraceRing::new(8);

    // Push various event types
    assert!(ring.push(TraceEvent::RunSubmitted { run: RunId::new(1) }));
    assert!(ring.push(TraceEvent::StepStarted {
        run: RunId::new(1),
        step: StepIdx::new(0)
    }));
    assert!(ring.push(TraceEvent::ActionCompleted {
        run: RunId::new(1),
        step: StepIdx::new(0)
    }));

    // Drain all events
    let events = ring.drain();
    assert_eq!(events.len(), 3, "all pushed events are drained");

    // Snapshot for a run
    let snap = ring.snapshot_for_run(RunId::new(1), 10);
    assert_eq!(snap.len(), 0, "empty after drain");

    // Push and check terminal detection
    assert!(ring.push(TraceEvent::RunFinished { run: RunId::new(2) }));
    assert!(ring.has_terminal_event_for_run(RunId::new(2)));
    assert!(!ring.has_terminal_event_for_run(RunId::new(99)));
}

// ─────────────────────────────────────────────────────────────────────────
// TRC-08 / TRC-14: has_terminal_event_for_run — RunCancelled is terminal
// ─────────────────────────────────────────────────────────────────────────

/// TRC-08: RunCancelled is a terminal event, has_terminal_event_for_run returns true.
#[test]
fn trace_ring_has_terminal_event_for_run_cancelled() {
    // Given: a trace ring with a RunCancelled event for run 3
    let mut ring = TraceRing::new(8);
    assert!(ring.push(TraceEvent::RunCancelled { run: RunId::new(3) }));

    // When: has_terminal_event_for_run is called for run 3
    let result = ring.has_terminal_event_for_run(RunId::new(3));

    // Then: returns true (RunCancelled is a terminal variant)
    assert!(
        result,
        "has_terminal_event_for_run must return true for RunCancelled"
    );
}

/// TRC-08: RunFailed is a terminal event, has_terminal_event_for_run returns true.
#[test]
fn trace_ring_has_terminal_event_for_run_failed() {
    // Given: a trace ring with a RunFailed event for run 4
    let mut ring = TraceRing::new(8);
    assert!(ring.push(TraceEvent::RunFailed { run: RunId::new(4) }));

    // When: has_terminal_event_for_run is called for run 4
    let result = ring.has_terminal_event_for_run(RunId::new(4));

    // Then: returns true (RunFailed is a terminal variant)
    assert!(
        result,
        "has_terminal_event_for_run must return true for RunFailed"
    );
}

/// TRC-08: has_terminal_event_for_run returns false when only non-terminal events present.
#[test]
fn trace_ring_has_terminal_event_returns_false_when_only_non_terminal_events() {
    // Given: a ring with only StepStarted and StepEnded events (non-terminal)
    let mut ring = TraceRing::new(8);
    assert!(ring.push(TraceEvent::StepStarted {
        run: RunId::new(5),
        step: StepIdx::new(0)
    }));
    assert!(ring.push(TraceEvent::StepEnded {
        run: RunId::new(5),
        step: StepIdx::new(0)
    }));
    assert!(ring.push(TraceEvent::ActionCompleted {
        run: RunId::new(5),
        step: StepIdx::new(0)
    }));

    // When: has_terminal_event_for_run is called for run 5
    let result = ring.has_terminal_event_for_run(RunId::new(5));

    // Then: returns false (no terminal event for run 5)
    assert!(
        !result,
        "has_terminal_event_for_run must return false when only non-terminal events"
    );
}

/// TRC-14: TraceEvent::is_terminal_for_run returns true only for terminal variants.
#[test]
fn trace_event_is_terminal_for_run_run_cancelled_is_terminal() {
    let run = RunId::new(7);
    // RunCancelled must be terminal for its own run
    assert!(
        TraceEvent::RunCancelled { run }.is_terminal_for_run(run),
        "RunCancelled must be terminal for matching run"
    );
    // RunCancelled must NOT be terminal for a different run
    assert!(
        !TraceEvent::RunCancelled { run }.is_terminal_for_run(RunId::new(99)),
        "RunCancelled must not be terminal for non-matching run"
    );
}

/// TRC-14: TraceEvent::is_terminal_for_run RunFailed variant.
#[test]
fn trace_event_is_terminal_for_run_run_failed_is_terminal() {
    let run = RunId::new(8);
    assert!(
        TraceEvent::RunFailed { run }.is_terminal_for_run(run),
        "RunFailed must be terminal for matching run"
    );
    assert!(
        !TraceEvent::RunFailed { run }.is_terminal_for_run(RunId::new(99)),
        "RunFailed must not be terminal for non-matching run"
    );
}

/// TRC-14: TraceEvent::is_terminal_for_run RunFinished variant.
#[test]
fn trace_event_is_terminal_for_run_run_finished_is_terminal() {
    let run = RunId::new(9);
    assert!(
        TraceEvent::RunFinished { run }.is_terminal_for_run(run),
        "RunFinished must be terminal for matching run"
    );
    assert!(
        !TraceEvent::RunFinished { run }.is_terminal_for_run(RunId::new(99)),
        "RunFinished must not be terminal for non-matching run"
    );
}

/// TRC-14: TraceEvent::is_terminal_for_run returns false for all non-terminal variants.
#[test]
fn trace_event_is_terminal_for_run_non_terminal_variants_return_false() {
    let run = RunId::new(10);
    let step = StepIdx::new(0);
    let slot = SlotIdx::new(0);
    let code = ActionFailureCode::Timeout;

    assert!(
        !TraceEvent::StepStarted { run, step }.is_terminal_for_run(run),
        "StepStarted is not terminal"
    );
    assert!(
        !TraceEvent::StepEnded { run, step }.is_terminal_for_run(run),
        "StepEnded is not terminal"
    );
    assert!(
        !TraceEvent::SlotWritten {
            run,
            slot,
            value: vec![]
        }
        .is_terminal_for_run(run),
        "SlotWritten is not terminal"
    );
    assert!(
        !TraceEvent::ActionScheduled { run, step }.is_terminal_for_run(run),
        "ActionScheduled is not terminal"
    );
    assert!(
        !TraceEvent::ActionCompleted { run, step }.is_terminal_for_run(run),
        "ActionCompleted is not terminal"
    );
    assert!(
        !TraceEvent::ActionFailed { run, step, code }.is_terminal_for_run(run),
        "ActionFailed is not terminal"
    );
    assert!(
        !TraceEvent::AskAnswered { run, step, slot }.is_terminal_for_run(run),
        "AskAnswered is not terminal"
    );
    assert!(
        !TraceEvent::RunSubmitted { run }.is_terminal_for_run(run),
        "RunSubmitted is not terminal"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// TRC-16: drain + refill preserves data correctly across cycles
// ─────────────────────────────────────────────────────────────────────────

/// TRC-16: Fill-drain-refill cycle preserves newest events after eviction.
#[test]
fn trace_ring_fill_drain_refill_preserves_newest_events() {
    // Given: a ring with capacity 3
    let mut ring = TraceRing::new(3);
    // Fill with 3 events (0, 1, 2)
    for i in 0..3u64 {
        assert!(ring.push(TraceEvent::RunFinished { run: RunId::new(i) }));
    }
    // Drain all 3
    let first = ring.drain();
    assert_eq!(first.len(), 3, "first drain returns 3 events");

    // Refill with 3 new events (3, 4, 5)
    for i in 3..6u64 {
        assert!(ring.push(TraceEvent::RunFinished { run: RunId::new(i) }));
    }

    // Drain again
    let second = ring.drain();
    assert_eq!(
        second.len(),
        3,
        "second drain returns 3 events after refill"
    );
    // Verify these are the new events (3, 4, 5) — oldest from first batch were evicted
    assert_eq!(
        second.get(0),
        Some(&TraceEvent::RunFinished { run: RunId::new(3) }),
        "second drain first event is run 3"
    );
    assert_eq!(
        second.get(2),
        Some(&TraceEvent::RunFinished { run: RunId::new(5) }),
        "second drain last event is run 5"
    );
}
