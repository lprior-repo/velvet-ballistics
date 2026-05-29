#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::RunId;
use vb_runtime::shard::{CompletionWatermark, CompletionWatermarkError};

#[test]
fn completing_prefix_in_order_drains_each_sequence() {
    let run = RunId::new(11);
    let mut watermark = CompletionWatermark::new(run, 4, 4);

    let first = watermark.complete(run, 1);
    assert_eq!(
        first.map(|drain| (drain.boundary, drain.drained.into_vec())),
        Ok((1, vec![1]))
    );

    let second = watermark.complete(run, 2);
    assert_eq!(
        second.map(|drain| (drain.boundary, drain.drained.into_vec())),
        Ok((2, vec![2]))
    );
}

#[test]
fn out_of_order_completion_waits_for_gap_then_drains_prefix() {
    let run = RunId::new(12);
    let mut watermark = CompletionWatermark::new(run, 4, 4);

    let gap = watermark.complete(run, 2);
    assert_eq!(
        gap.map(|drain| (drain.boundary, drain.drained.into_vec())),
        Ok((0, Vec::new()))
    );
    assert_eq!(watermark.pending_len(), 1);

    let prefix = watermark.complete(run, 1);
    assert_eq!(
        prefix.map(|drain| (drain.boundary, drain.drained.into_vec())),
        Ok((2, vec![1, 2]))
    );
    assert_eq!(watermark.pending_len(), 0);
}

#[test]
fn duplicate_completion_does_not_double_drain() {
    let run = RunId::new(13);
    let mut watermark = CompletionWatermark::new(run, 4, 4);

    assert!(watermark.complete(run, 1).is_ok());
    assert_eq!(
        watermark.complete(run, 1),
        Err(CompletionWatermarkError::Duplicate { seq: 1 })
    );
    assert_eq!(watermark.boundary(), 1);
}

#[test]
fn invalid_sequence_and_capacity_return_typed_errors() {
    let run = RunId::new(14);
    let mut watermark = CompletionWatermark::new(run, 1, 1);

    assert_eq!(
        watermark.complete(run, 0),
        Err(CompletionWatermarkError::InvalidSequence { seq: 0 })
    );
    assert_eq!(
        watermark.register_waiter(0),
        Err(CompletionWatermarkError::InvalidSequence { seq: 0 })
    );
    assert_eq!(watermark.register_waiter(2), Ok(()));
    assert_eq!(
        watermark.register_waiter(3),
        Err(CompletionWatermarkError::QueueFull { capacity: 1 })
    );
}

#[test]
fn large_boundary_sequence_does_not_overflow() {
    let run = RunId::new(15);
    let mut watermark = CompletionWatermark::from_boundary(run, u64::MAX - 1, 1, 1);
    let result = watermark.complete(run, u64::MAX);

    assert_eq!(
        result.map(|drain| (drain.boundary, drain.drained.into_vec())),
        Ok((u64::MAX, vec![u64::MAX]))
    );
    assert_eq!(
        watermark.complete(run, u64::MAX),
        Err(CompletionWatermarkError::Duplicate { seq: u64::MAX })
    );
}

proptest! {
    #[test]
    fn completion_watermark_boundary_never_decreases(seq_values in prop::collection::vec(1_u64..=8, 1..16)) {
        let run = RunId::new(16);
        let mut watermark = CompletionWatermark::new(run, 8, 8);
        let mut previous = watermark.boundary();

        for seq in seq_values {
            let _ = watermark.complete(run, seq);
            prop_assert!(watermark.boundary() >= previous);
            previous = watermark.boundary();
        }
    }
}
