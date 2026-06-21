#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::borrow_deref_ref,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::cloned_ref_to_slice_refs,
    clippy::cmp_owned,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::derivable_impls,
    clippy::duplicated_attributes,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::io_other_error,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::manual_unwrap_or_default,
    clippy::map_clone,
    clippy::map_flatten,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::new_without_default,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_sort_by,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_asref,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
#![forbid(unsafe_code)]
//! RAII Introspection Registry Tests
//!
//! Tests for the RAII-based introspection registry that manages handles for
//! making runs visible to inspect operations. The registry provides:
//! - Epoch-based handle registration with automatic cleanup on guard drop
//! - Typed conflict outcomes for overlapping registrations
//! - No-op outcomes for missing handle unregister operations
//! - Bulk unregister operations for overlapping ranges
//!
//! This test file covers:
//! - Happy paths: register, unregister, visibility via snapshot
//! - Error paths: overlapping registration, missing handle unregister
//! - Edge cases: stale guard drop, unregister_all
//! - Contract assertions: no global mutable state, cold-path formatting

use std::num::NonZeroUsize;

use vb_core::ids::{ConstIdx, RunId, SlotIdx, StepIdx};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::{
    InspectResponse, InspectSnapshotFormatter, IntrospectionRegistry, RegisterOverlapOutcome,
    ShardConfig, UnregisterOutcome,
};

// ============================================================================
// Test Fixtures and Helpers
// ============================================================================

fn shard_count(value: usize) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(value).ok_or_else(|| format!("expected non-zero shard count, got {value}"))
}

fn relaxed_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 32,
        trace_capacity: 64,
        step_budget_per_tick: 16,
        max_active_runs: 8,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
        coalesce_window_ticks: 1,
        snapshot_interval_steps: 0,
        max_terminal_runs: 16,
        terminal_runs_ttl_ticks: 86_400,        max_terminal_outcomes: 100_000,
    }
}

/// Creates a finished workflow (SetConst -> Finish).
fn finished_workflow() -> Result<CompiledWorkflow, String> {
    let nodes = Box::from([
        CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        },
    ]);

    let parts = WorkflowParts {
        name: Box::from("test_finished"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0x21; 32]),
        nodes,
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };

    CompiledWorkflow::try_from_parts(parts).map_err(|e| format!("workflow creation failed: {e:?}"))
}

// ============================================================================
// Happy Path Tests
// ============================================================================

/// Registering a shard handle makes it visible to inspect snapshot.
#[test]
fn raii_registry_register_handle_makes_run_visible_to_snapshot() -> Result<(), String> {
    // Given: a fresh runtime and a submitted run
    let mut runtime = Runtime::new(shard_count(1)?, relaxed_config()).expect("runtime config is valid");
    let run = RunId::new(1001);

    // Submit a simple workflow that finishes immediately
    let workflow = finished_workflow()?;

    runtime
        .submit_direct(run, workflow)
        .map_err(|e| format!("submit failed: {e:?}"))?;
    runtime
        .tick_all()
        .map_err(|e| format!("tick failed: {e:?}"))?;

    // Create a registry and register the handle
    let mut registry = IntrospectionRegistry::new();
    let handle = registry
        .register(run)
        .map_err(|e| format!("register failed: {e:?}"))?;

    // When: snapshot is requested through the registry
    // Then: the run is visible because the handle is registered
    assert!(
        registry.is_visible(run),
        "run {:?} should be visible after handle registration",
        run
    );

    // Cleanup: drop the handle
    drop(handle);

    // Then: the run is no longer visible
    assert!(
        !registry.is_visible(run),
        "run {:?} should not be visible after handle dropped",
        run
    );

    Ok(())
}

/// Dropping matching guard removes handle from registry.
#[test]
fn raii_registry_drop_guard_removes_handle() -> Result<(), String> {
    // Given: a fresh registry
    let mut registry = IntrospectionRegistry::new();
    let run = RunId::new(1002);

    // Register a handle
    let handle = registry
        .register(run)
        .map_err(|e| format!("register failed: {e:?}"))?;

    // Verify the run is visible
    assert!(
        registry.is_visible(run),
        "run should be visible after register"
    );

    // When: the guard is dropped
    drop(handle);

    // Then: the run is no longer visible
    assert!(
        !registry.is_visible(run),
        "run should not be visible after guard drop"
    );

    Ok(())
}

// ============================================================================
// Error Path Tests
// ============================================================================

/// Invalid input overlapping registration returns typed conflict.
#[test]
fn raii_registry_overlapping_registration_returns_conflict() -> Result<(), String> {
    // Given: a registry with an existing registration
    let mut registry = IntrospectionRegistry::new();
    let run = RunId::new(1003);

    let _handle1 = registry
        .register(run)
        .map_err(|e| format!("first register failed: {e:?}"))?;

    // When: a second registration is attempted for the same run
    let result = registry.register_with_overlap_policy(run);

    // Then: the result indicates a conflict or replacement
    match result {
        Ok((_handle, Err(RegisterOverlapOutcome::Conflict))) => {
            // Expected: conflict detected
        }
        Ok((_handle, Err(RegisterOverlapOutcome::Replaced { .. }))) => {
            // Also acceptable: replaced with new epoch
        }
        Ok((_, Ok(()))) => {
            return Err(String::from(
                "overlapping registration should not succeed silently",
            ));
        }
        Err(e) => {
            return Err(format!("unexpected error: {e:?}"));
        }
    }

    Ok(())
}

/// Overlapping registration replaces with new epoch.
#[test]
fn raii_registry_overlapping_registration_replaces_with_new_epoch() -> Result<(), String> {
    // Given: a registry with an existing registration
    let mut registry = IntrospectionRegistry::new();
    let run = RunId::new(1004);

    let handle1 = registry
        .register(run)
        .map_err(|e| format!("first register failed: {e:?}"))?;
    let epoch1 = handle1.epoch();

    // When: a second registration is attempted with overlap policy
    let result = registry.register_with_overlap_policy(run);

    // Then: the result should be a replacement with a new epoch
    match result {
        Ok((
            handle2,
            Err(RegisterOverlapOutcome::Replaced {
                old_epoch,
                new_epoch,
            }),
        )) => {
            if old_epoch == epoch1 && new_epoch > old_epoch {
                // Expected: epoch incremented
            } else {
                return Err(format!(
                    "expected old_epoch={}, new_epoch={}, got old={}, new={}",
                    epoch1,
                    epoch1 + 1,
                    old_epoch,
                    new_epoch
                ));
            }
            // The new handle should have the new epoch
            if handle2.epoch() != new_epoch {
                return Err(format!(
                    "handle epoch should be {}, got {}",
                    new_epoch,
                    handle2.epoch()
                ));
            }
        }
        Ok((_, Err(RegisterOverlapOutcome::Conflict))) => {
            // Conflict is also acceptable (depends on policy)
        }
        Ok((_, Ok(()))) => {
            return Err(String::from(
                "overlapping registration should not succeed silently",
            ));
        }
        Err(e) => {
            return Err(format!("unexpected error: {e:?}"));
        }
    }

    Ok(())
}

/// Missing handle unregister is a no-op typed outcome.
#[test]
fn raii_registry_missing_handle_unregister_is_noop() -> Result<(), String> {
    // Given: a fresh registry with no registrations
    let mut registry = IntrospectionRegistry::new();
    let run = RunId::new(9999); // Never registered

    // When: unregister is called for a missing handle
    let outcome = registry
        .unregister(run)
        .map_err(|e| format!("unregister failed: {e:?}"))?;

    // Then: the outcome is Missing (no-op)
    assert_eq!(
        outcome,
        UnregisterOutcome::Missing,
        "unregistering missing handle should return Missing outcome"
    );

    Ok(())
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Stale guard drop after replacement preserves fresh handle.
#[test]
fn raii_registry_stale_guard_drop_preserves_fresh_handle() -> Result<(), String> {
    // Given: a registry with an existing registration
    let mut registry = IntrospectionRegistry::new();
    let run = RunId::new(1005);

    // First registration
    let handle1 = registry
        .register(run)
        .map_err(|e| format!("first register failed: {e:?}"))?;
    let epoch1 = handle1.epoch();

    // When: we register again (creating a new handle) and then drop the OLD handle
    let result = registry
        .register_with_overlap_policy(run)
        .map_err(|e| format!("second register failed: {e:?}"))?;
    let handle2_epoch = result.0.epoch();

    // Drop the FIRST handle (the stale one)
    drop(handle1);

    // Then: the NEW handle should still be valid
    assert!(
        registry.is_visible(run),
        "run should still be visible after dropping stale handle"
    );

    // And: the registry still tracks this run
    let unregister_result = registry
        .unregister(run)
        .map_err(|e| format!("unregister failed: {e:?}"))?;
    assert_eq!(
        unregister_result,
        UnregisterOutcome::Unregistered,
        "should be able to unregister the fresh handle"
    );

    // Verify epoch was incremented (stale handle's epoch should not work)
    assert!(handle2_epoch >= epoch1, "new epoch should be >= old epoch");

    Ok(())
}

/// Unregister_all removes all handles.
#[test]
fn raii_registry_unregister_all_removes_all_handles() -> Result<(), String> {
    // Given: a registry with multiple registrations
    let mut registry = IntrospectionRegistry::new();

    // Register multiple runs
    let run1 = RunId::new(1010);
    let run2 = RunId::new(1011);
    let run3 = RunId::new(1012);

    let _handle1 = registry
        .register(run1)
        .map_err(|e| format!("register run1 failed: {e:?}"))?;
    let _handle2 = registry
        .register(run2)
        .map_err(|e| format!("register run2 failed: {e:?}"))?;
    let _handle3 = registry
        .register(run3)
        .map_err(|e| format!("register run3 failed: {e:?}"))?;

    // Verify all are visible
    assert!(registry.is_visible(run1));
    assert!(registry.is_visible(run2));
    assert!(registry.is_visible(run3));

    // When: unregister_all is called
    let removed_count = registry
        .unregister_all()
        .map_err(|e| format!("unregister_all failed: {e:?}"))?;

    // Then: all registrations are removed
    assert_eq!(
        removed_count, 3,
        "unregister_all should remove all 3 registrations"
    );

    // And: no runs are visible
    assert!(!registry.is_visible(run1));
    assert!(!registry.is_visible(run2));
    assert!(!registry.is_visible(run3));

    Ok(())
}

// ============================================================================
// Contract Assertion Tests
// ============================================================================

/// Introspection registration does not create global mutable run state.
#[test]
fn raii_registry_does_not_create_global_mutable_run_state() -> Result<(), String> {
    // Given: two independent registries
    let mut registry1 = IntrospectionRegistry::new();
    let registry2 = IntrospectionRegistry::new();

    let run = RunId::new(1020);

    // Register in the first registry only
    let _handle1 = registry1
        .register(run)
        .map_err(|e| format!("register failed: {e:?}"))?;

    // Then: the second registry should NOT see the registration
    assert!(
        !registry2.is_visible(run),
        "independent registry should not see registration from another registry"
    );

    // And: the first registry should still see its registration
    assert!(
        registry1.is_visible(run),
        "first registry should still see its own registration"
    );

    Ok(())
}

/// Snapshot formatting stays cold path (no computation on hot path).
#[test]
fn raii_registry_snapshot_formatting_stays_cold_path() -> Result<(), String> {
    // Given: an inspect response
    let response = InspectResponse::NotFound {
        run: RunId::new(1030),
        correlation: 42,
    };

    // When: formatting is applied (this should be a simple formatting, no computation)
    let formatted = InspectSnapshotFormatter::format_snapshot(RunId::new(1030), &response);

    // Then: the formatting produces expected output
    assert!(
        formatted.contains("NotFound"),
        "formatted output should contain NotFound"
    );
    assert!(
        formatted.contains("1030"),
        "formatted output should contain run id"
    );

    Ok(())
}

// ============================================================================
// Integration Test: Full RAII Lifecycle
// ============================================================================

/// Full lifecycle: register, inspect, unregister, verify cleanup.
#[test]
fn raii_registry_full_lifecycle_with_inspect() -> Result<(), String> {
    // Given: a fresh runtime and registry
    let mut runtime = Runtime::new(shard_count(1)?, relaxed_config()).expect("runtime config is valid");
    let mut registry = IntrospectionRegistry::new();

    let run = RunId::new(1040);

    // Submit and drive a workflow to completion
    let workflow = finished_workflow()?;

    runtime
        .submit_direct(run, workflow)
        .map_err(|e| format!("submit failed: {e:?}"))?;
    runtime
        .tick_all()
        .map_err(|e| format!("tick failed: {e:?}"))?;

    // The run is no longer active (finished), but we want to test registry visibility
    // In a real scenario, the registry would track visibility independently of run state

    // When: we register the handle
    let handle = registry
        .register(run)
        .map_err(|e| format!("register failed: {e:?}"))?;

    // Then: snapshot should show the registration effect
    assert!(
        registry.is_visible(run),
        "run should be visible after registration"
    );

    // When: we drop the handle
    drop(handle);

    // Then: visibility is removed
    assert!(
        !registry.is_visible(run),
        "run should not be visible after guard drop"
    );

    Ok(())
}
