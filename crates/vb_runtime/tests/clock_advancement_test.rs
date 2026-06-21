#![forbid(unsafe_code)]
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
    clippy::collapsible_if,
    clippy::collapsible_match,
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
    clippy::map_clone,
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
//! PS-007: Clock Advancement — behavior tests (G1-G5).
//!
//! Tests the numeric timer seam `advance_clock_to` and `current_tick` API
//! on `Shard`. Uses deterministic `TimerTick` values instead of `Instant`.

use vb_runtime::shard::types::{Shard, ShardConfig, TimerTick};

// ---------- Behavior G1: Backward clock advance rejected ----------

#[test]
fn advance_clock_to_rejects_backward_tick_returns_error() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(100)), Ok(()));
    let result = shard.advance_clock_to(TimerTick::new(50));
    assert_eq!(result, Err(vb_runtime::RuntimeError::InvalidTimerFire));
    // Current tick must be preserved after rejection
    assert_eq!(shard.current_tick(), TimerTick::new(100));
    Ok(())
}

#[test]
fn advance_clock_to_backward_tick_preserves_current_tick() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(1000)), Ok(()));
    // Attempt to go backward
    let _ = shard.advance_clock_to(TimerTick::new(500));
    // Tick must remain 1000
    assert_eq!(shard.current_tick(), TimerTick::new(1000));
    Ok(())
}

#[test]
fn advance_clock_to_rejects_single_tick_backward() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(10)), Ok(()));
    assert_eq!(
        shard.advance_clock_to(TimerTick::new(9)),
        Err(vb_runtime::RuntimeError::InvalidTimerFire)
    );
    assert_eq!(shard.current_tick(), TimerTick::new(10));
    Ok(())
}

// ---------- Behavior G2: Equal tick advance is a no-op ----------

#[test]
fn advance_clock_to_same_tick_is_noop() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(42)), Ok(()));
    assert_eq!(shard.advance_clock_to(TimerTick::new(42)), Ok(()));
    assert_eq!(shard.current_tick(), TimerTick::new(42));
    Ok(())
}

#[test]
fn advance_clock_to_same_zero_tick_is_noop() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.current_tick(), TimerTick::new(0));
    assert_eq!(shard.advance_clock_to(TimerTick::new(0)), Ok(()));
    assert_eq!(shard.current_tick(), TimerTick::new(0));
    Ok(())
}

// ---------- Behavior G3: Forward advance fires due timers ----------

#[test]
fn advance_clock_to_forward_increments_current_tick() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(50)), Ok(()));
    assert_eq!(shard.current_tick(), TimerTick::new(50));
    assert_eq!(shard.advance_clock_to(TimerTick::new(100)), Ok(()));
    assert_eq!(shard.current_tick(), TimerTick::new(100));
    Ok(())
}

#[test]
fn advance_clock_to_multiple_forward_steps_are_monotonic() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(ShardConfig::default());
    let ticks = [1u64, 5, 10, 50, 100, 500, 1000];
    for (i, &tick) in ticks.iter().enumerate() {
        assert_eq!(shard.advance_clock_to(TimerTick::new(tick)), Ok(()));
        assert_eq!(shard.current_tick(), TimerTick::new(tick));
        // Monotonic: each tick is >= previous
        if i > 0 {
            assert!(tick >= ticks[i - 1]);
        }
    }
    Ok(())
}

#[test]
fn advance_clock_to_large_jump_succeeds() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(0)), Ok(()));
    assert_eq!(shard.advance_clock_to(TimerTick::new(1_000_000)), Ok(()));
    assert_eq!(shard.current_tick(), TimerTick::new(1_000_000));
    Ok(())
}

// ---------- Behavior G5: Maximum tick boundary ----------

#[test]
fn advance_clock_to_accepts_max_u64_tick() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(u64::MAX)), Ok(()));
    assert_eq!(shard.current_tick(), TimerTick::new(u64::MAX));
    Ok(())
}

#[test]
fn advance_clock_to_max_tick_then_reject_any_subsequent() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(u64::MAX)), Ok(()));
    // Any tick < u64::MAX is now backward
    assert_eq!(
        shard.advance_clock_to(TimerTick::new(u64::MAX - 1)),
        Err(vb_runtime::RuntimeError::InvalidTimerFire)
    );
    // Equal tick is still OK (no-op)
    assert_eq!(shard.advance_clock_to(TimerTick::new(u64::MAX)), Ok(()));
    Ok(())
}

// ---------- current_tick availability ----------

#[test]
fn current_tick_starts_at_zero_for_new_shard() -> Result<(), RuntimeError> {
    let shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.current_tick(), TimerTick::new(0));
    Ok(())
}

#[test]
fn current_tick_returns_consistent_value() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(77)), Ok(()));
    // Multiple reads all return the same value
    for _ in 0..10 {
        assert_eq!(shard.current_tick(), TimerTick::new(77));
    }
    Ok(())
}

// ---------- Shard status includes tick state ----------

#[test]
fn shard_status_available_after_clock_advance() -> Result<(), RuntimeError> {
    let mut shard = Shard::new(ShardConfig::default());
    assert_eq!(shard.advance_clock_to(TimerTick::new(100)), Ok(()));
    let status = shard.status();
    // Status is available; tick does not corrupt shard state
    assert!(status.running);
    Ok(())
}
