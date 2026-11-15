#![forbid(unsafe_code)]
use vb_core::RunId;
use vb_runtime::trace::{TraceEvent, TraceRing};

#[test]
fn zero_trace_capacity_normalizes_to_one_for_push_snapshot_and_drain() {
    let first = TraceEvent::RunSubmitted {
        run: RunId::new(11),
    };
    let second = TraceEvent::RunFinished {
        run: RunId::new(13),
    };
    let mut ring = TraceRing::new(0);
    assert_eq!(ring.capacity(), 1);
    assert!(ring.push(first.clone()));
    assert!(!ring.push(second.clone()));
    assert_eq!(ring.dropped(), 1);
    assert_eq!(ring.pending_len(), 1);
    assert_eq!(ring.history_len(), 1);
    assert_eq!(ring.snapshot_for_run(RunId::new(11), 8), [first.clone()]);
    assert_eq!(ring.drain_for_run(RunId::new(13), 8), []);
    assert_eq!(ring.pending_len(), 1);
    assert_eq!(ring.drain_for_run(RunId::new(11), 8), [first]);
    assert!(ring.pending_is_empty());
    assert!(ring.push(second.clone()));
    assert_eq!(ring.snapshot_for_run(RunId::new(11), 8), []);
    assert_eq!(ring.snapshot_for_run(RunId::new(13), 8), [second.clone()]);
    assert_eq!(ring.drain(), [second]);
    assert!(ring.pending_is_empty());
    assert_eq!(ring.history_len(), 1);
    assert_eq!(ring.dropped(), 1);
}
