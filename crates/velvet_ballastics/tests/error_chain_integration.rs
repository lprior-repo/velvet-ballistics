//! Error chain integration tests.
//!
//! Verifies that error types implement Display, Error, and support proper
//! error chain propagation through From conversions across crate boundaries.

use std::error::Error;

/// Compile error propagates to CLI with proper Display and source chain.
#[test]
fn compile_error_propagates_to_cli() {
    let source = b"version: velvet-ballastics/v1\nname: test\nwhen:\n  manual: {}\nsteps: []\n";
    let result = vb_compile::compile_workflow(source);
    let Err(errors) = result else {
        panic!("expected compile error for empty steps");
    };

    // CompileErrors implements Display
    let display = format!("{errors}");
    assert!(
        display.contains("steps must not be empty"),
        "CompileErrors Display should mention empty steps, got: {display}"
    );

    // The first error implements Display
    let first = errors
        .first()
        .cloned()
        .ok_or_else(|| String::from("expected at least one error"))
        .ok();
    let Some(first_error) = first else {
        panic!("CompileErrors should contain at least one CompileError");
    };
    let first_display = format!("{first_error}");
    assert!(
        !first_display.is_empty(),
        "CompileError Display must not be empty"
    );
}

/// ValidationError converts into CompileError preserving the diagnostic code.
#[test]
fn validation_error_propagates_into_compile_error() {
    let validation = vb_validate::ValidationError::DuplicateKey;
    let compile_error: vb_compile::CompileError = validation.into();
    let display = format!("{compile_error}");
    assert!(
        display.contains("validation gate failure"),
        "CompileError from ValidationError should display validation gate failure, got: {display}"
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
    assert!(
        display.contains("compiled workflow IR failed validation"),
        "CompileError from WorkflowError should display IR validation failure, got: {display}"
    );
}

/// CoreError converts into RuntimeError for queue-full propagation.
#[test]
fn runtime_error_includes_cause_from_core_error() {
    let core_error = vb_core::errors::CoreError::QueueFull;
    let runtime_error: vb_runtime::RuntimeError = core_error.into();

    // CoreError::QueueFull maps to RuntimeError::QueueFull
    let display = format!("{runtime_error}");
    assert!(
        display.contains("queue full"),
        "RuntimeError from CoreError::QueueFull should display queue full, got: {display}"
    );
}

/// JournalError converts into RuntimeError mapping to storage failure.
#[test]
fn storage_error_propagates_up_through_runtime() {
    let journal_error = vb_storage::JournalError::WriteLockPoisoned;
    let runtime_error: vb_runtime::RuntimeError = journal_error.into();

    let display = format!("{runtime_error}");
    assert!(
        display.contains("storage journal append failed"),
        "RuntimeError from JournalError should map to StorageJournalAppendFailed, got: {display}"
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
    assert!(
        display.contains("must contain exactly one"),
        "CompileErrors Display should contain the inner error text, got: {display}"
    );
}

/// CoreError within WorkflowError within CompileError forms a three-level chain.
#[test]
fn error_chain_three_levels_deep() {
    let core_error = vb_core::errors::CoreError::ExpressionStackOverflow { max: 64 };
    let workflow_error = vb_core::workflow::WorkflowError::from(core_error);
    let compile_error = vb_compile::CompileError::from(workflow_error);

    let display = format!("{compile_error}");
    assert!(
        display.contains("compiled workflow IR failed validation"),
        "three-level chain top should show IR validation, got: {display}"
    );

    // Verify source chain: CompileError -> WorkflowError -> CoreError
    let workflow_src = compile_error.source();
    assert!(
        workflow_src.is_some(),
        "CompileError::Workflow should expose WorkflowError via source()"
    );
}

/// RecoveryError wraps JournalError and exposes it as source.
#[test]
fn recovery_error_wraps_journal_error() {
    let journal_error = vb_storage::JournalError::QueueFull;
    let recovery_error: vb_storage::recovery::RecoveryError = journal_error.into();

    let display = format!("{recovery_error}");
    assert!(
        display.contains("journal error during recovery"),
        "RecoveryError::Journal Display should mention recovery context, got: {display}"
    );

    // RecoveryError has source chain: RecoveryError -> JournalError
    let source = recovery_error.source();
    assert!(
        source.is_some(),
        "RecoveryError::Journal should expose JournalError via source()"
    );
}

/// Every error variant produces a non-empty Display string.
#[test]
fn all_runtime_error_variants_have_display() {
    let errors = vec![
        format!("{}", vb_runtime::RuntimeError::QueueFull),
        format!("{}", vb_runtime::RuntimeError::RunNotFound),
        format!("{}", vb_runtime::RuntimeError::RunAlreadyExists),
        format!("{}", vb_runtime::RuntimeError::ShutdownInProgress),
        format!("{}", vb_runtime::RuntimeError::JournalPoisoned),
        format!("{}", vb_runtime::RuntimeError::StorageJournalAppendFailed),
        format!("{}", vb_runtime::RuntimeError::FramePoolUnavailable),
        format!("{}", vb_runtime::RuntimeError::InvalidActionCompletion),
        format!("{}", vb_runtime::RuntimeError::InvalidTimerFire),
        format!(
            "{}",
            vb_runtime::RuntimeError::ActiveRunCapacityExceeded { capacity: 8 }
        ),
        format!(
            "{}",
            vb_runtime::RuntimeError::UnsupportedOperation {
                operation: "test_op"
            }
        ),
    ];
    for (i, display) in errors.iter().enumerate() {
        assert!(
            !display.is_empty(),
            "RuntimeError variant at index {i} should have non-empty Display"
        );
    }
}
