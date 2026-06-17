#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]
#![forbid(unsafe_code)]
//! Error chain integration tests.
//!
//! Verifies that error types implement Display, Error, and support proper
//! error chain propagation through From conversions across crate boundaries.

use std::error::Error;
use std::sync::Arc;

fn source_as<'a, T>(error: &'a (dyn Error + 'static)) -> &'a T
where
    T: Error + 'static,
{
    let source = error.source().and_then(<dyn Error>::downcast_ref::<T>);
    assert!(
        source.is_some(),
        "expected source type {}",
        std::any::type_name::<T>()
    );
    match source {
        Some(source) => source,
        None => std::process::abort(),
    }
}

/// Compile error propagates to CLI with proper Display and source chain.
#[test]
fn compile_error_propagates_to_cli() {
    let source = b"version: velvet-ballistics/v1\nname: test\nwhen:\n  manual: {}\nsteps: []\n";
    let compiled = vb_compile::compile_workflow(source);
    assert!(
        matches!(&compiled, Err(errors) if errors.iter().any(|e| matches!(e, vb_compile::CompileError::EmptySteps))),
        "expected EmptySteps compile error for empty steps, got: {:?}",
        compiled
    );
    let errors = match compiled {
        Ok(_) => std::process::abort(),
        Err(errors) => errors,
    };

    // CompileErrors implements Display
    let display = format!("{errors}");
    assert_eq!(
        display, "[0] workflow steps must not be empty",
        "CompileErrors Display should render the exact empty-steps diagnostic"
    );
    assert_eq!(errors.len(), 1, "expected exactly one compile error");

    // The first error implements Display
    let first_error = errors
        .first()
        .expect("errors must have at least one element");
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
        compile_error.diagnostic_code().as_str(),
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
            source_as::<vb_core::workflow::WorkflowError>(&compile_error),
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
            source_as::<vb_core::errors::CoreError>(&runtime_error),
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
            source_as::<vb_storage::JournalError>(&runtime_error),
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
            source_as::<vb_core::errors::CoreError>(&runtime_error),
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
            source_as::<vb_core::workflow::WorkflowError>(&compile_error),
            vb_core::workflow::WorkflowError::Expression(
                vb_core::errors::CoreError::ExpressionStackOverflow { max: 64 }
            )
        ),
        "CompileError should expose exact WorkflowError source"
    );

    let workflow_src = source_as::<vb_core::workflow::WorkflowError>(&compile_error);
    assert!(
        matches!(
            source_as::<vb_core::errors::CoreError>(workflow_src),
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
            source_as::<vb_compile::CompileError>(&errors),
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
            source_as::<vb_storage::JournalError>(&recovery_error),
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

#[test]
fn expr_error_propagates_through_workflow_error_to_compile_error() {
    let core_error = vb_core::errors::CoreError::DivisionByZero;
    assert_eq!(
        format!("{core_error}"),
        "division by zero",
        "CoreError::DivisionByZero Display should render"
    );

    let workflow_error = vb_core::workflow::WorkflowError::from(core_error);
    let compile_error: vb_compile::CompileError = workflow_error.into();
    assert!(
        compile_error
            .to_string()
            .contains("compiled workflow IR failed validation"),
        "compile error should wrap workflow error: {compile_error}"
    );
}

#[test]
fn journal_queue_full_maps_to_runtime_storage_append_error() {
    let journal_error = vb_storage::JournalError::QueueFull;
    let runtime_error: vb_runtime::RuntimeError = journal_error.into();
    assert_eq!(
        format!("{runtime_error}"),
        "storage journal append failed: journal writer queue is full",
        "QueueFull should map to StorageJournalAppend with exact display"
    );
    assert!(
        matches!(
            source_as::<vb_storage::JournalError>(&runtime_error),
            vb_storage::JournalError::QueueFull
        ),
        "RuntimeError should expose exact JournalError::QueueFull source"
    );
}

#[test]
fn all_compile_error_variants_have_non_empty_display() {
    let errors: &[vb_compile::CompileError] = &[
        vb_compile::CompileError::EmptySource,
        vb_compile::CompileError::EmptySteps,
        vb_compile::CompileError::Validation(vb_validate::ValidationError::DuplicateKey),
    ];
    for error in errors {
        let display = format!("{error}");
        assert!(
            !display.is_empty(),
            "compile error variant must have non-empty Display: {error:?}"
        );
        let code = error.diagnostic_code();
        assert!(
            !code.as_str().is_empty(),
            "compile error variant must have non-empty diagnostic code: {error:?}"
        );
    }
}

#[test]
fn runtime_error_admission_durability_code_is_stable() {
    let code = vb_runtime::RuntimeError::ADMISSION_HEADER_PERSISTENCE_FAILED_CODE;
    assert_eq!(
        code.to_string(),
        "E2015",
        "admission durability code must be stable"
    );
    let error = vb_runtime::RuntimeError::AdmissionHeaderPersistenceFailed {
        source: Arc::new(vb_storage::JournalError::QueueFull),
    };
    assert_eq!(
        error.runtime_code(),
        Some("ADMISSION_DURABILITY_ERROR"),
        "runtime_code must be stable for admission durability errors"
    );
}

#[test]
fn core_error_queue_full_implements_display_and_error_traits() {
    let error = vb_core::errors::CoreError::QueueFull;
    let runtime_error: vb_runtime::RuntimeError = error.into();

    let display = format!("{runtime_error}");
    assert_eq!(
        display, "runtime core error: queue full",
        "runtime error from QueueFull should display exact chain"
    );
    assert!(
        matches!(&runtime_error, vb_runtime::RuntimeError::Core { .. }),
        "QueueFull maps to RuntimeError::Core"
    );
    assert!(
        matches!(
            source_as::<vb_core::errors::CoreError>(&runtime_error),
            vb_core::errors::CoreError::QueueFull
        ),
        "RuntimeError should expose exact CoreError::QueueFull source"
    );
}
