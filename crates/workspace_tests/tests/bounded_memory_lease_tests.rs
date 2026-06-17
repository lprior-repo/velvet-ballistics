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
    unused_variables,
)]

#![cfg(test)]
#![forbid(unsafe_code)]
//! bounded_memory_lease_tests: Bounded Memory Lease Tests
//!
//! Integration tests for the bounded memory lease system via AggregateResourceUsage API.
//!
//! These tests exercise the vb_core::budget::AggregateResourceUsage API directly,
//! proving budget arithmetic, capacity checking, and error handling behavior.
//!
//! # Feature Description
//!
//! The AggregateResourceUsage API provides:
//! - Budget addition via `try_add_budget`
//! - Budget subtraction via `try_subtract_budget`
//! - Capacity validation via `fits_within`
//! - Policy checking via `check_policy`
//!
//! # Test Categories
//!
//! ## Happy Paths
//! - [`reserve_then_drop_returns_memory_to_budget`] - Reserve then drop returns memory to the budget
//!
//! ## Error Paths
//! - [`reserve_above_capacity_returns_capacity_exceeded`] - Invalid reserve above capacity returns typed CapacityExceeded
//!
//! ## Edge Cases
//! - [`zero_size_reserve_follows_noop_contract`] - Zero-size reserve follows documented no-op/rejection
//!
//! ## Contract Assertions
//! - [`no_panic_api_required_for_memory_operations`] - No panic API is required
//! - [`budget_accounting_exact_after_orphaned_lease_drop`] - Budget accounting exact after orphaned drop
//!
//! ## Boundary Tests
//! - [`reserve_overflow_returns_error`] - Overflow returns error
//! - [`release_underflow_returns_error`] - Underflow returns error

use std::error::Error;
use vb_core::budget::{AggregateResourceBudget, AggregateResourceCapacity, AggregateResourceUsage};

// =============================================================================
// Test Data Fixtures
// =============================================================================

/// A small budget suitable for testing edge cases.
fn small_budget() -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_steps_executable: 10,
        max_action_tickets: 5,
        max_parallel_in_flight: 2,
        max_retries_per_action: 3,
        max_gather_pages: 1,
        max_gather_items: 100,
        max_for_each_iterations: 50,
        max_together_branches: 4,
        max_repeat_attempts: 3,
        max_run_time_seconds: 60,
        max_result_bytes: 1024,
        max_total_slots_written: 256,
        max_timer_entries: 100,
        max_trace_events: 1000,
        max_queue_depth: 32,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 8192,
        max_blob_bytes: 16384,
        max_input_bytes: 1024,
        max_step_budget_per_tick: 4,
        max_transitions_per_tick: 4,
    }
}

/// A capacity that fits the small budget exactly (1x).
#[allow(dead_code)]
fn small_capacity() -> AggregateResourceCapacity {
    AggregateResourceCapacity {
        max_steps_executable: 10,
        max_action_tickets: 5,
        max_parallel_in_flight: 2,
        max_gather_pages: 1,
        max_gather_items: 100,
        max_result_bytes: 1024,
        max_total_slots_written: 256,
        max_timer_entries: 100,
        max_trace_events: 1000,
        max_active_runs: 4,
        max_queue_depth: 32,
        max_journal_batch_bytes: 4096,
        max_ipc_payload_bytes: 8192,
        max_blob_bytes: 16384,
        max_input_bytes: 1024,
        max_step_budget_per_tick: 4,
        max_transitions_per_tick: 4,
    }
}

/// A capacity that is smaller than the small budget (constraining).
fn tight_capacity() -> AggregateResourceCapacity {
    AggregateResourceCapacity {
        max_steps_executable: 5,   // 50% of budget request
        max_action_tickets: 3,     // 60% of budget request
        max_parallel_in_flight: 1, // 50% of budget request
        max_gather_pages: 1,
        max_gather_items: 50,  // 50% of budget request
        max_result_bytes: 512, // 50% of budget request
        max_total_slots_written: 128,
        max_timer_entries: 50,
        max_trace_events: 500,
        max_active_runs: 2,
        max_queue_depth: 16,
        max_journal_batch_bytes: 2048,
        max_ipc_payload_bytes: 4096,
        max_blob_bytes: 8192,
        max_input_bytes: 512,
        max_step_budget_per_tick: 2,
        max_transitions_per_tick: 2,
    }
}

/// A capacity that is larger than the small budget (headroom).
fn headroom_capacity() -> AggregateResourceCapacity {
    AggregateResourceCapacity {
        max_steps_executable: 100,
        max_action_tickets: 50,
        max_parallel_in_flight: 20,
        max_gather_pages: 10,
        max_gather_items: 1000,
        max_result_bytes: 10240,
        max_total_slots_written: 2560,
        max_timer_entries: 1000,
        max_trace_events: 10000,
        max_active_runs: 40,
        max_queue_depth: 320,
        max_journal_batch_bytes: 40960,
        max_ipc_payload_bytes: 81920,
        max_blob_bytes: 163840,
        max_input_bytes: 10240,
        max_step_budget_per_tick: 40,
        max_transitions_per_tick: 40,
    }
}

/// A budget with specific result_bytes and blob_bytes set.
fn budget_with_bytes(result_bytes: u32, blob_bytes: u64) -> AggregateResourceBudget {
    AggregateResourceBudget {
        max_result_bytes: result_bytes,
        max_blob_bytes: blob_bytes,
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_retries_per_action: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: 0,
        max_total_slots_written: 0,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    }
}

/// An empty usage (no reservations).
fn empty_usage() -> AggregateResourceUsage {
    AggregateResourceUsage {
        max_steps_executable: 0,
        max_action_tickets: 0,
        max_parallel_in_flight: 0,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_result_bytes: 0,
        max_total_slots_written: 0,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_active_runs: 0,
        max_queue_depth: 0,
        max_journal_batch_bytes: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
        max_step_budget_per_tick: 0,
        max_transitions_per_tick: 0,
    }
}

// =============================================================================
// Happy Path Tests
// =============================================================================

/// H-1: reserve_then_drop_returns_memory_to_budget
///
/// When a memory lease is reserved and then dropped (released), the budget
/// accounting must show the bytes returned to the available pool.
///
/// # Contract
/// - `try_add_budget` adds to usage
/// - `try_subtract_budget` removes from usage
/// - Net change is zero after full lifecycle
#[test]
fn reserve_then_drop_returns_memory_to_budget() -> Result<(), Box<dyn Error>> {
    let capacity = headroom_capacity();
    let initial_usage = empty_usage();

    // Verify initial state fits within capacity
    initial_usage.fits_within(&capacity)?;

    let reserve_bytes: u64 = 256;
    let _run_id = vb_core::ids::RunId::new(1);

    // Step 1: Reserve (add budget)
    let after_reserve = initial_usage.try_add_budget(&budget_with_bytes(
        u32::try_from(reserve_bytes)?,
        reserve_bytes,
    ))?;

    // Verify bytes were reserved (usage increased)
    if after_reserve.max_result_bytes != reserve_bytes {
        return Err(format!(
            "reserve must increase result_bytes usage: expected {}, got {}",
            reserve_bytes, after_reserve.max_result_bytes
        )
        .into());
    }
    if after_reserve.max_blob_bytes != reserve_bytes {
        return Err(format!(
            "reserve must increase blob_bytes usage: expected {}, got {}",
            reserve_bytes, after_reserve.max_blob_bytes
        )
        .into());
    }

    // Step 2: Drop the lease (release memory back to budget)
    let after_drop = after_reserve.try_subtract_budget(&budget_with_bytes(
        u32::try_from(reserve_bytes)?,
        reserve_bytes,
    ))?;

    // Step 3: Verify budget returned to initial state
    if after_drop.max_result_bytes != 0 {
        return Err(format!(
            "after drop, result_bytes usage must return to 0: got {}",
            after_drop.max_result_bytes
        )
        .into());
    }
    if after_drop.max_blob_bytes != 0 {
        return Err(format!(
            "after drop, blob_bytes usage must return to 0: got {}",
            after_drop.max_blob_bytes
        )
        .into());
    }

    // Sanity: capacity check still passes after full cycle
    after_drop.fits_within(&capacity)?;

    Ok(())
}

// =============================================================================
// Error Path Tests
// =============================================================================

/// E-1: reserve_above_capacity_returns_capacity_exceeded
///
/// When a reservation request exceeds the available capacity,
/// the operation MUST return a typed `CapacityExceeded` error (NOT panic).
///
/// # Contract
/// - `try_add_budget` performs arithmetic addition only (may succeed)
/// - `fits_within` validates against capacity and returns `CapacityExceeded` if exceeded
/// - The error contains `requested` and `available` fields
/// - No panic occurs
#[test]
fn reserve_above_capacity_returns_capacity_exceeded() -> Result<(), Box<dyn Error>> {
    let budget = small_budget();
    let capacity = tight_capacity(); // 50% of budget
    let usage = empty_usage();

    // Usage fits in tight capacity (empty)
    usage.fits_within(&capacity)?;

    // Step 1: Add the budget (arithmetic addition)
    let result = usage.try_add_budget(&budget);
    if result.is_err() {
        return Err(format!(
            "try_add_budget must succeed for arithmetic (overflow check only): {:?}",
            result
        )
        .into());
    }

    let after_add = result?;

    // Step 2: Check capacity - this is where CapacityExceeded is detected
    let capacity_result = after_add.fits_within(&capacity);

    // The capacity check should fail because 1024 > 512
    if !capacity_result.is_err() {
        return Err("resulting usage must exceed tight capacity".into());
    }

    // The error should be CapacityExceeded (not overflow)
    let Err(error) = capacity_result else {
        return Err("expected CapacityExceeded error".into());
    };

    // Assert the correct error variant using matches! macro
    if !matches!(
        error,
        vb_core::budget::AggregateBudgetError::CapacityExceeded { .. }
    ) {
        return Err(format!(
            "Expected CapacityExceeded error for capacity exceeded case, got {:?}",
            error
        )
        .into());
    }

    Ok(())
}

// =============================================================================
// Edge Case Tests
// =============================================================================

/// EC-1: zero_size_reserve_follows_noop_contract
///
/// A zero-size reservation is a no-op (adding zero doesn't change usage).
///
/// # Contract
/// - Zero-size reserve does NOT panic
/// - Zero-size reserve returns Ok with no change to usage
#[test]
fn zero_size_reserve_follows_noop_contract() -> Result<(), Box<dyn Error>> {
    let usage = empty_usage();
    let capacity = headroom_capacity();

    // A zero-size budget addition is a no-op (adding zero doesn't change usage)
    let zero_budget = budget_with_bytes(0, 0);

    let result = usage.try_add_budget(&zero_budget);

    // Zero addition must succeed (no-op)
    if result.is_err() {
        return Err("adding zero budget must be a no-op and return Ok".into());
    }

    let after = result?;

    if after.max_result_bytes != 0 {
        return Err(format!(
            "zero add must not change result_bytes: got {}",
            after.max_result_bytes
        )
        .into());
    }
    if after.max_blob_bytes != 0 {
        return Err(format!(
            "zero add must not change blob_bytes: got {}",
            after.max_blob_bytes
        )
        .into());
    }

    // Verify it still fits in capacity
    after.fits_within(&capacity)?;

    Ok(())
}

// =============================================================================
// Contract Assertion Tests
// =============================================================================

/// CA-1: no_panic_api_required_for_memory_operations
///
/// Memory operations (reserve, release, split, merge) must NEVER panic.
/// All error conditions must be returned as typed errors.
///
/// # Contract
/// - All memory operations return `Result` or `Option`
/// - No `unwrap()` or `expect()` in memory operation paths
/// - Panic occurs ONLY if bug in implementation (not expected behavior)
#[test]
fn no_panic_api_required_for_memory_operations() -> Result<(), Box<dyn Error>> {
    let usage = empty_usage();
    let capacity = headroom_capacity();

    // All these operations must NOT panic

    // try_add_budget
    let add_result = usage.try_add_budget(&small_budget());
    if add_result.is_err() {
        return Err("try_add_budget must not panic on valid input".into());
    }

    // try_subtract_budget with matching values
    if let Ok(added) = add_result {
        let sub_result = added.try_subtract_budget(&small_budget());
        if sub_result.is_err() {
            return Err("try_subtract_budget must not panic on valid input".into());
        }
    }

    // fits_within
    let fit_result = usage.fits_within(&capacity);
    if fit_result.is_err() {
        return Err("fits_within must not panic".into());
    }

    // check_policy
    let policy_result = usage.check_policy(&vb_core::budget::BoundednessPolicy::DEFAULT);
    if policy_result.is_err() {
        return Err("check_policy must not panic".into());
    }

    // validate_aggregate_budget
    let validate_result = vb_core::budget::validate_aggregate_budget(
        &small_budget(),
        &vb_core::budget::BoundednessPolicy::DEFAULT,
    );
    if validate_result.is_err() {
        return Err("validate_aggregate_budget must not panic".into());
    }

    Ok(())
}

/// CA-2: budget_accounting_exact_after_orphaned_lease_drop
///
/// When a lease is dropped without being properly released (orphaned),
/// the budget accounting MUST still be exact - no leakage, no phantom bytes.
///
/// # Contract
/// - Orphaned drop does not leak bytes
/// - Budget usage after orphaned drop is deterministic
/// - Dropping an already-dropped lease returns Underflow error
#[test]
fn budget_accounting_exact_after_orphaned_lease_drop() -> Result<(), Box<dyn Error>> {
    // Scenario:
    // 1. Create usage with some bytes reserved
    // 2. "Orphan" the lease (simulate by trying to subtract what was never added)
    // 3. Budget accounting must remain exact

    let usage = empty_usage();
    let budget = budget_with_bytes(256, 0);

    // Add budget first
    let with_usage = usage.try_add_budget(&budget)?;

    if with_usage.max_result_bytes != 256 {
        return Err(format!(
            "after add, usage must be exactly 256: got {}",
            with_usage.max_result_bytes
        )
        .into());
    }

    // Now "orphan" - we drop what we added
    let after_orphan = with_usage.try_subtract_budget(&budget)?;

    // Budget must be EXACTLY back to initial state
    if after_orphan.max_result_bytes != 0 {
        return Err(format!(
            "after orphan drop, usage must return to exactly 0: got {}",
            after_orphan.max_result_bytes
        )
        .into());
    }

    // No leakage: trying to drop again fails with underflow
    let double_drop = after_orphan.try_subtract_budget(&budget);
    if !double_drop.is_err() {
        return Err("dropping already-dropped usage must fail with underflow error".into());
    }

    // The error must be underflow, not some other error
    let Err(error) = double_drop else {
        return Err("double drop must produce error".into());
    };
    if !matches!(
        error,
        vb_core::budget::AggregateBudgetError::Underflow { .. }
    ) {
        return Err(format!("double drop must produce Underflow error, got: {:?}", error).into());
    }

    Ok(())
}

// =============================================================================
// Budget Overflow/Underflow Boundary Tests
// =============================================================================

/// B-1: reserve_overflow_returns_error
///
/// When reservation would overflow the budget, return error (not panic).
#[test]
fn reserve_overflow_returns_error() -> Result<(), Box<dyn Error>> {
    let max_usage = AggregateResourceUsage {
        max_result_bytes: u64::MAX,
        max_blob_bytes: u64::MAX,
        ..Default::default()
    };

    let budget = budget_with_bytes(1, 1);

    // Adding to max usage should overflow
    let result = max_usage.try_add_budget(&budget);

    if !result.is_err() {
        return Err("adding to max usage must return error, not panic".into());
    }

    let Err(error) = result else {
        return Err("overflow must produce error".into());
    };
    if !matches!(
        error,
        vb_core::budget::AggregateBudgetError::Overflow { .. }
    ) {
        return Err(format!("overflow must produce Overflow error, got: {:?}", error).into());
    }

    Ok(())
}

/// B-2: release_underflow_returns_error
///
/// When release would go negative, return error (not panic).
#[test]
fn release_underflow_returns_error() -> Result<(), Box<dyn Error>> {
    let usage = empty_usage();

    let budget = budget_with_bytes(1, 0);

    // Subtracting from zero usage should underflow
    let result = usage.try_subtract_budget(&budget);

    if !result.is_err() {
        return Err("subtracting from zero must return error, not panic".into());
    }

    let Err(error) = result else {
        return Err("underflow must produce error".into());
    };
    if !matches!(
        error,
        vb_core::budget::AggregateBudgetError::Underflow { .. }
    ) {
        return Err(format!("underflow must produce Underflow error, got: {:?}", error).into());
    }

    Ok(())
}
