#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use vb_core::ids::RunId;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::types::{Shard, ShardConfig};
use vb_runtime::trace::{TraceEvent, TraceRing};
use vb_runtime::{RuntimeError, RuntimeResult};

#[test]
fn shard_storage_overflow_is_a_typed_error() {
    let result =
        Runtime::new_for_tests_and_benchmarks_only(NonZeroUsize::MAX, ShardConfig::default());
    assert_eq!(
        result.err(),
        Some(RuntimeError::ShardAllocationFailed {
            requested: usize::MAX
        })
    );
}

#[test]
fn trace_overflow_is_rejected_by_public_constructors() {
    let config = ShardConfig {
        trace_capacity: usize::MAX,
        ..ShardConfig::default()
    };
    let expected = RuntimeError::TraceCapacityOverflow {
        requested: usize::MAX,
    };
    assert_eq!(TraceRing::try_new(usize::MAX).err(), Some(expected.clone()));
    assert_eq!(Shard::new(config).err(), Some(expected.clone()));
    assert_eq!(
        Runtime::new_for_tests_and_benchmarks_only(NonZeroUsize::MIN, config).err(),
        Some(expected)
    );
}

#[test]
fn normal_public_construction_succeeds() -> RuntimeResult<()> {
    let config = ShardConfig {
        trace_capacity: 2,
        ..ShardConfig::default()
    };
    let shard = Shard::new(config)?;
    assert_eq!(
        shard.command_queue_capacity(),
        config.command_queue_capacity
    );
    let _runtime = Runtime::new_for_tests_and_benchmarks_only(NonZeroUsize::MIN, config)?;
    Ok(())
}

#[test]
fn fallible_trace_preserves_fifo_full_and_history() -> RuntimeResult<()> {
    let mut ring = TraceRing::try_new(2)?;
    let run = RunId::new(7);
    let first = TraceEvent::RunSubmitted { run };
    let second = TraceEvent::RunSubmitted { run: RunId::new(8) };
    assert!(ring.push(first.clone()));
    assert!(ring.push(second.clone()));
    assert!(!ring.push(first.clone()));
    assert_eq!(ring.dropped(), 1);
    assert_eq!(ring.drain(), vec![first.clone(), second]);
    assert!(ring.pending_is_empty());
    assert_eq!(ring.history_len(), 2);
    assert_eq!(ring.snapshot_for_run(run, 2), vec![first]);
    Ok(())
}

#[test]
fn fallible_zero_trace_preserves_legacy_behavior() -> RuntimeResult<()> {
    let mut ring = TraceRing::try_new(0)?;
    assert_eq!(ring.capacity(), 0);
    assert!(ring.push(TraceEvent::RunSubmitted { run: RunId::new(7) }));
    assert_eq!(ring.pending_len(), 1);
    assert!(ring.history_is_empty());
    assert!(ring.drain().is_empty());
    assert_eq!(ring.pending_len(), 1);
    Ok(())
}

#[test]
fn constructor_error_identity_and_diagnostics_are_preserved() {
    let shard = RuntimeError::ShardAllocationFailed { requested: 7 };
    let trace = RuntimeError::TraceCapacityOverflow { requested: 7 };
    assert_eq!(shard, shard.clone());
    assert_eq!(trace, trace.clone());
    assert_ne!(shard, RuntimeError::ShardAllocationFailed { requested: 8 });
    assert_ne!(trace, RuntimeError::TraceCapacityOverflow { requested: 8 });
    assert_ne!(shard, trace);
    assert_eq!(shard.to_string(), "cannot reserve storage for 7 shards");
    assert_eq!(trace.to_string(), "trace capacity cannot be represented: 7");
    assert_eq!(
        shard.diagnostic_code(),
        RuntimeError::SHARD_ALLOCATION_FAILED_CODE
    );
    assert_eq!(
        trace.diagnostic_code(),
        RuntimeError::TRACE_CAPACITY_OVERFLOW_CODE
    );
}
