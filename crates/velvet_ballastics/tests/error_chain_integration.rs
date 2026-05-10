#![forbid(unsafe_code)]
//! Error chain integration tests.
//!
//! Verifies that error types implement Display, Error, and support proper
//! error chain propagation through From conversions across crate boundaries.

use std::error::Error;

fn source_as<'a, T>(error: &'a (dyn Error + 'static)) -> Option<&'a T>
where
    T: Error + 'static,
{
    error.source().and_then(<dyn Error>::downcast_ref::<T>)
}

/// Compile error propagates to CLI with proper Display and source chain.
#[test]
fn compile_error_propagates_to_cli() {
    let source = b"version: velvet-ballastics/v1\nname: test\nwhen:\n  manual: {}\nsteps: []\n";
    let errors = vb_compile::compile_workflow(source).expect_err("expected compile error for empty steps");

    // CompileErrors implements Display
    let display = format!("{errors}");
    assert_eq!(
        display, "[0] workflow steps must not be empty",
        "CompileErrors Display should render the exact empty-steps diagnostic"
    );
    assert_eq!(errors.len(), 1, "expected exactly one compile error");

    // The first error implements Display
    let first_error = errors.first().expect("CompileErrors should contain at least one CompileError");
    let first_display = format!("{first_error}");
    assert_eq!(
        first_display, "workflow steps must not be empty",
        "CompileError Display must render the exact empty-steps diagnostic"
    );
    assert!(
        matches!(first_error, vb_compile::CompileError::EmptySteps),
        "CompileErrors should preserve the exact EmptySteps variant"
    );
}

/// ValidationError converts into CompileError preserving the diagnostic code.
#[test]
fn validation_error_propagates_into_compile_error() {
    let validation = vb_validate::ValidationError::DuplicateKey;
    let compile_error: vb_compile::CompileError = validation.into();
    let display = format!("{compile_error}");
    assert_eq!(
        display, "validation gate failure: DUPLICATE_KEY",
        "CompileError from ValidationError should display the exact validation chain"
    );
    assert!(
        matches!(
            &compile_error,
            &vb_compile::CompileError::Validation(vb_validate::ValidationError::DuplicateKey)
        ),
        "CompileError should preserve the exact ValidationError::DuplicateKey source"
    );
    assert_eq!(
        compile_error.diagnostic_code(),
        "INVALID_COMPILED_WORKFLOW",
        "diagnostic code should be preserved through From conversion"
    );
}

/// WorkflowError converts into CompileError preserving the chain.
#[test]
fn workflow_error_propagates_into_compile_error() {
    let workflow_error = vb_core::workflow::WorkflowError::EmptyNodes;
    let compile_error: vb_compile::CompileError = workflow_error.into();
    let display = format!("{compile_error}");
    assert_eq!(
        display,
        "compiled workflow IR failed validation: compiled workflow must contain at least one node",
        "CompileError from WorkflowError should display the exact IR validation chain"
    );
    assert!(
        matches!(
            source_as::<vb_core::workflow::WorkflowError>(&compile_error).expect("should have source"),
            vb_core::workflow::WorkflowError::EmptyNodes
        ),
        "CompileError should expose exact WorkflowError::EmptyNodes source"
    );
}

/// CoreError converts into RuntimeError for queue-full propagation.
#[test]
fn runtime_error_includes_cause_from_core_error() {
    let core_error = vb_core::errors::CoreError::QueueFull;
    let runtime_error: vb_runtime::RuntimeError = core_error.into();

    // CoreError::QueueFull maps to RuntimeError::QueueFull
    let display = format!("{runtime_error}");
    assert_eq!(
        display, "runtime core error: queue full",
        "RuntimeError from CoreError::QueueFull should display the exact core chain"
    );
    assert!(
        matches!(&runtime_error, &vb_runtime::RuntimeError::Core { .. }),
        "RuntimeError should preserve CoreError in RuntimeError::Core"
    );
    assert!(
        matches!(
            source_as::<vb_core::errors::CoreError>(&runtime_error).expect("should have source"),
            vb_core::errors::CoreError::QueueFull
        ),
        "RuntimeError should expose the exact CoreError::QueueFull source"
    );
}

/// JournalError converts into RuntimeError mapping to storage failure.
#[test]
fn storage_error_propagates_up_through_runtime() {
    let journal_error = vb_storage::JournalError::WriteLockPoisoned;
    let runtime_error: vb_runtime::RuntimeError = journal_error.into();

    let display = format!("{runtime_error}");
    assert_eq!(
        display, "storage journal append failed: journal write lock is poisoned",
        "RuntimeError from JournalError should display the exact storage append chain"
    );
    assert!(
        matches!(
            &runtime_error,
            &vb_runtime::RuntimeError::StorageJournalAppend { .. }
        ),
        "RuntimeError should preserve JournalError in StorageJournalAppend"
    );
    assert!(
        matches!(
            source_as::<vb_storage::JournalError>(&runtime_error).expect("should have source"),
            vb_storage::JournalError::WriteLockPoisoned
        ),
        "RuntimeError should expose the exact JournalError::WriteLockPoisoned source"
    );
}

/// RuntimeError converted from non-queue CoreError keeps CoreError in the source chain.
#[test]
fn core_error_propagates_up_through_runtime_with_source() {
    let core_error = vb_core::errors::CoreError::ExpressionStackOverflow { max: 16 };
    let runtime_error: vb_runtime::RuntimeError = core_error.into();

    let display = format!("{runtime_error}");
    assert_eq!(
        display, "runtime core error: expression stack overflow: max 16",
        "RuntimeError from CoreError should display the exact runtime core chain"
    );
    assert!(
        matches!(
            source_as::<vb_core::errors::CoreError>(&runtime_error).expect("should have source"),
            vb_core::errors::CoreError::ExpressionStackOverflow { max: 16 }
        ),
        "RuntimeError should expose exact CoreError source"
    );
}

/// CompileError converts into CompileErrors preserving the single error.
#[test]
fn single_compile_error_into_compile_errors() {
    let compile_error = vb_compile::CompileError::EmptySource;
    let errors: vb_compile::CompileErrors = vb_compile::CompileErrors(vec![compile_error]);
    assert_eq!(
        errors.len(),
        1,
        "CompileErrors from single error should have exactly one error"
    );
    let display = format!("{errors}");
    assert_eq!(
        display, "[0] YAML source must contain exactly one non-empty document",
        "CompileErrors Display should render the exact inner error text"
    );
}

/// CoreError within WorkflowError within CompileError forms a three-level chain.
#[test]
fn error_chain_three_levels_deep() {
    let core_error = vb_core::errors::CoreError::ExpressionStackOverflow { max: 64 };
    let workflow_error = vb_core::workflow::WorkflowError::from(core_error);
    let compile_error = vb_compile::CompileError::from(workflow_error);

    let display = format!("{compile_error}");
    assert_eq!(
        display,
        "compiled workflow IR failed validation: expression program is invalid: expression stack overflow: max 64",
        "three-level chain top should render the exact IR validation chain"
    );

    assert!(
        matches!(
            source_as::<vb_core::workflow::WorkflowError>(&compile_error).expect("should have source"),
            vb_core::workflow::WorkflowError::Expression(
                vb_core::errors::CoreError::ExpressionStackOverflow { max: 64 }
            )
        ),
        "CompileError should expose exact WorkflowError source"
    );

    let workflow_src = source_as::<vb_core::workflow::WorkflowError>(&compile_error).expect("should have source");
    assert!(
        matches!(
            source_as::<vb_core::errors::CoreError>(workflow_src).expect("should have source"),
            vb_core::errors::CoreError::ExpressionStackOverflow { max: 64 }
        ),
        "WorkflowError should expose exact nested CoreError source"
    );
}

/// CompileErrors exposes its first CompileError as source for aggregate chains.
#[test]
fn compile_errors_collection_exposes_first_source() {
    let errors = vb_compile::CompileErrors(vec![vb_compile::CompileError::EmptySource]);
    assert!(
        matches!(
            source_as::<vb_compile::CompileError>(&errors).expect("should have source"),
            vb_compile::CompileError::EmptySource
        ),
        "CompileErrors should expose the exact first CompileError source"
    );
}

/// RecoveryError wraps JournalError and exposes it as source.
#[test]
fn recovery_error_wraps_journal_error() {
    let journal_error = vb_storage::JournalError::QueueFull;
    let recovery_error: vb_storage::recovery::RecoveryError = journal_error.into();

    let display = format!("{recovery_error}");
    assert_eq!(
        display, "journal error during recovery: journal writer queue is full",
        "RecoveryError::Journal Display should render the exact journal chain"
    );
    assert!(
        matches!(
            &recovery_error,
            &vb_storage::recovery::RecoveryError::Journal(vb_storage::JournalError::QueueFull)
        ),
        "RecoveryError should preserve JournalError in the exact Journal variant"
    );

    assert!(
        matches!(
            source_as::<vb_storage::JournalError>(&recovery_error).expect("should have source"),
            vb_storage::JournalError::QueueFull
        ),
        "RecoveryError::Journal should expose exact JournalError source"
    );
}

/// Every error variant produces a non-empty Display string.
#[test]
fn all_runtime_error_variants_have_display() {
    assert_eq!(
        format!("{}", vb_runtime::RuntimeError::QueueFull),
        "queue full"
    );
    assert_eq!(
        format!("{}", vb_runtime::RuntimeError::RunNotFound),
        "run not found"
    );
    assert_eq!(
        format!("{}", vb_runtime::RuntimeError::RunAlreadyExists),
        "run already exists"
    );
    assert_eq!(
        format!("{}", vb_runtime::RuntimeError::ShutdownInProgress),
        "shutdown in progress"
    );
    assert_eq!(
        format!("{}", vb_runtime::RuntimeError::JournalPoisoned),
        "runtime journal lock poisoned"
    );
    assert_eq!(
        format!(
            "{}",
            vb_runtime::RuntimeError::from(vb_storage::JournalError::QueueFull)
        ),
        "storage journal append failed: journal writer queue is full"
    );
    assert_eq!(
        format!("{}", vb_runtime::RuntimeError::FramePoolUnavailable),
        "frame pool unavailable"
    );
    assert_eq!(
        format!("{}", vb_runtime::RuntimeError::InvalidActionCompletion),
        "invalid action completion"
    );
    assert_eq!(
        format!("{}", vb_runtime::RuntimeError::InvalidTimerFire),
        "invalid timer fire"
    );
    assert_eq!(
        format!(
            "{}",
            vb_runtime::RuntimeError::ActiveRunCapacityExceeded { capacity: 8 }
        ),
        "active run capacity exceeded: 8"
    );
    assert_eq!(
        format!(
            "{}",
            vb_runtime::RuntimeError::UnsupportedOperation {
                operation: "test_op"
            }
        ),
        "unsupported runtime operation: test_op"
    );
}
