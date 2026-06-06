// vb-pyg3p: behavior test for vb_runtime runtime facade
// Tests public Runtime facade importability, API surface, and typed-error paths.
//
// This is a basic behavior test - proptest properties can be added
// in vb_runtime_facade_restoration_properties.rs once the lane is fixed.

#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use vb_core::WorkflowDigest;
use vb_core::ids::RunId;
use vb_runtime::runtime::{ActiveRunSummary, Runtime};
use vb_runtime::shard::ShardConfig;
use vb_runtime::{RuntimeError, RuntimeResult};

/// Test that Runtime can be constructed with valid configuration.
#[test]
fn test_runtime_construction_succeeds() {
    let config = ShardConfig::default();
    let _runtime = Runtime::new(NonZeroUsize::new(4).expect("non-zero"), config);
}

/// Test that ActiveRunSummary fields are accessible.
#[test]
fn test_active_run_summary_fields_accessible() {
    let summary = ActiveRunSummary {
        run_id: RunId::new(42),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
        step_count: 10,
        steps_completed: 5,
    };
    assert_eq!(summary.run_id, RunId::new(42));
    assert_eq!(summary.step_count, 10);
    assert_eq!(summary.steps_completed, 5);
}

/// Test that ActiveRunSummary Clone and Eq work.
#[test]
fn test_active_run_summary_clone_and_eq() {
    let summary1 = ActiveRunSummary {
        run_id: RunId::new(123),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
        step_count: 8,
        steps_completed: 3,
    };
    let summary2 = summary1.clone();
    assert_eq!(summary1, summary2);
    assert_eq!(summary1.step_count, summary2.step_count);
}

/// Test RuntimeError Display does not panic.
#[test]
fn test_runtime_error_display_queue_full() {
    let err = RuntimeError::QueueFull;
    let display = format!("{err}");
    assert!(!display.is_empty());
}

/// Test RuntimeError Display for RunNotFound.
#[test]
fn test_runtime_error_display_run_not_found() {
    let err = RuntimeError::RunNotFound;
    let display = format!("{err}");
    assert!(!display.is_empty());
}

/// Test RuntimeError Display for UnsupportedOperation.
#[test]
fn test_runtime_error_display_unsupported_operation() {
    let err = RuntimeError::UnsupportedOperation {
        operation: "test_operation",
    };
    let display = format!("{err}");
    assert!(!display.is_empty());
}

/// Test RuntimeResult ok path.
#[test]
fn test_runtime_result_ok() {
    let config = ShardConfig::default();
    let result: RuntimeResult<Runtime> = Ok(Runtime::new(
        NonZeroUsize::new(2).expect("non-zero"),
        config,
    ));
    assert!(result.is_ok());
}

/// Test RuntimeResult err path.
#[test]
fn test_runtime_result_err() {
    let err = RuntimeError::ActiveRunCapacityExceeded { capacity: 100 };
    let result: RuntimeResult<()> = Err(err);
    assert!(result.is_err());
}
