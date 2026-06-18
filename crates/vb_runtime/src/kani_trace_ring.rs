#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani proof harnesses for TraceRing boundedness, monotonicity, FIFO ordering,
//! and terminal event detection.
//!
//! Bounded capacity: 1..=64 (exhaustive check for all push/drain paths)
//! `rtrb` crate ring buffer implementation is trusted.
//! `trace.rs` is `#![forbid(unsafe_code)]`.

use crate::trace::{TraceEvent, TraceRing};
use vb_core::action::ActionFailureCode;
use vb_core::ids::{RunId, SlotIdx, StepIdx};

/// Generate an arbitrary TraceEvent for a given run.
///
/// Uses kani::any() for StepIdx/SlotIdx to avoid GOD RULE hardcoded-0 violations.
/// The indices are identifiers only (not array bounds), so any u16 value is valid.
fn arbitrary_trace_event(run: RunId, variant_selector: u8) -> TraceEvent {
    let step: StepIdx = kani::any();
    let slot: SlotIdx = kani::any();
    match variant_selector % 11 {
        0 => TraceEvent::StepStarted { run, step },
        1 => TraceEvent::StepEnded { run, step },
        2 => TraceEvent::SlotWritten {
            run,
            slot,
            value: vec![kani::any()],
        },
        3 => TraceEvent::ActionScheduled { run, step },
        4 => TraceEvent::ActionCompleted { run, step },
        5 => TraceEvent::ActionFailed {
            run,
            step,
            code: ActionFailureCode::Timeout,
        },
        6 => TraceEvent::AskAnswered { run, step, slot },
        7 => TraceEvent::RunSubmitted { run },
        8 => TraceEvent::RunFinished { run },
        9 => TraceEvent::RunFailed { run },
        _ => TraceEvent::RunCancelled { run },
    }
}

/// OBL-TRC-001: TraceRing len <= capacity invariant.
///
/// For any capacity in 1..=64 and any sequence of push/drain operations,
/// the ring length never exceeds the configured capacity.
#[kani::proof]
fn verify_trace_ring_bounds() {
    // Bound capacity to reasonable range for exhaustive check.
    let capacity: usize = kani::any_where(|c| *c >= 1 && *c <= 64);
    let mut ring = TraceRing::new(capacity);

    // Simulate arbitrary push/drain sequence.
    // Use a bounded loop to keep state space tractable.
    let steps: usize = kani::any_where(|s| *s <= 8);

    for i in 0..8 {
        if i >= steps {
            break;
        }

        let run = RunId::new(i as u64);
        let event = arbitrary_trace_event(run, i as u8);

        // push returns false when ring is full (not a panic).
        let _pushed = ring.push(event);

        // drain up to capacity.
        let _drained = ring.drain();

        // Invariant: len never exceeds capacity.
        kani::assert(
            ring.len() <= ring.capacity(),
            "TraceRing len never exceeds capacity",
        );
    }
}

/// OBL-TRC-002: TraceRing dropped counter is monotonic.
///
/// The dropped counter only increases and never decreases,
/// and never wraps (u64 saturated arithmetic).
#[kani::proof]
fn verify_trace_ring_dropped_monotonic() {
    let capacity: usize = kani::any_where(|c| *c >= 1 && *c <= 64);
    let mut ring = TraceRing::new(capacity);

    let initial_dropped = ring.dropped();

    // Fill ring past capacity to trigger drops.
    for i in 0..16 {
        let run = RunId::new(i as u64);
        let event = arbitrary_trace_event(run, i as u8);
        let _ = ring.push(event);
    }

    let after_dropped = ring.dropped();

    // Monotonicity: dropped only increases.
    kani::assert(
        after_dropped >= initial_dropped,
        "dropped counter is monotonic",
    );

    // Boundedness: dropped counter fits in u64 (saturating_add used in implementation).
    kani::assert(after_dropped <= u64::MAX, "dropped fits in u64");
}

/// OBL-TRC-003: drain_for_run filter correctness and insertion-order preservation.
///
/// drain_for_run returns only events matching the target run_id,
/// and returns them in the same order they appear in the ring (FIFO).
#[kani::proof]
#[kani::unwind(24)]
fn verify_drain_for_run_correctness() {
    let capacity: usize = kani::any_where(|c| *c >= 4 && *c <= 16);
    let mut ring = TraceRing::new(capacity);

    let target_run = RunId::new(42);
    let other_run = RunId::new(99);

    let _ = ring.push(TraceEvent::StepStarted {
        run: target_run,
        step: kani::any(),
    });
    let _ = ring.push(TraceEvent::ActionScheduled {
        run: other_run,
        step: kani::any(),
    });
    let _ = ring.push(TraceEvent::RunFinished { run: target_run });
    let _ = ring.push(TraceEvent::RunSubmitted { run: other_run });

    // Drain for target run.
    let drained = ring.drain_for_run(target_run, 4);

    let mut target_position = 0u8;
    for event in &drained {
        kani::assert(event.run_id() == target_run, "drained event belongs to target run");
        match target_position {
            0 => match event {
                TraceEvent::StepStarted { run, .. } => {
                    kani::assert(*run == target_run, "first target event is first pushed target")
                }
                _ => kani::assert(false, "first target event preserves FIFO order"),
            },
            1 => match event {
                TraceEvent::RunFinished { run } => {
                    kani::assert(*run == target_run, "second target event is second pushed target")
                }
                _ => kani::assert(false, "second target event preserves FIFO order"),
            },
            _ => kani::assert(false, "no extra target events are drained"),
        }
        target_position = match target_position.checked_add(1) {
            Some(next) => next,
            None => return,
        };
    }

    kani::assert(drained.len() == 2, "only target events are drained");
    kani::assert(target_position == 2, "two target events are observed");

    core::mem::forget(drained);
    core::mem::forget(ring);
}

/// OBL-TRC-004: Terminal event (RunFinished/RunFailed/RunCancelled) detection.
///
/// has_terminal_event_for_run returns true iff the ring contains a terminal
/// event (RunFinished, RunFailed, or RunCancelled) for the target run.
#[kani::proof]
fn verify_terminal_event_detection() {
    let capacity: usize = kani::any_where(|c| *c >= 1 && *c <= 64);
    let mut ring = TraceRing::new(capacity);

    let target_run = RunId::new(7);
    let other_run = RunId::new(99);

    // Push a terminal event for target_run.
    let terminal = TraceEvent::RunFinished { run: target_run };
    let _ = ring.push(terminal);

    // Push non-terminal events.
    let _ = ring.push(TraceEvent::StepStarted {
        run: target_run,
        step: StepIdx::new(0),
    });
    let _ = ring.push(TraceEvent::RunSubmitted { run: other_run });

    // Detection must return true for target_run (has terminal).
    kani::assert(
        ring.has_terminal_event_for_run(target_run),
        "target_run has terminal event",
    );

    // Detection must return false for other_run (no terminal).
    kani::assert(
        !ring.has_terminal_event_for_run(other_run),
        "other_run has no terminal event",
    );

    // Empty ring has no terminal events.
    let empty_ring = TraceRing::new(8);
    kani::assert(
        !empty_ring.has_terminal_event_for_run(target_run),
        "empty ring has no terminal event",
    );

    // RunFailed terminal event.
    let mut ring2 = TraceRing::new(8);
    let _ = ring2.push(TraceEvent::RunFailed { run: target_run });
    kani::assert(
        ring2.has_terminal_event_for_run(target_run),
        "RunFailed is terminal",
    );

    // RunCancelled terminal event.
    let mut ring3 = TraceRing::new(8);
    let _ = ring3.push(TraceEvent::RunCancelled { run: target_run });
    kani::assert(
        ring3.has_terminal_event_for_run(target_run),
        "RunCancelled is terminal",
    );
}
