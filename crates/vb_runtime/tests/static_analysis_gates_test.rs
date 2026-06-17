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
//! Static Analysis Gates — compile-time and type-system verification.
//!
//! Validates that timer-related types uphold compile-time invariants
//! without needing runtime execution. Uses `static_assertions`-style checks
//! and type-level assertions.

use std::mem::size_of;
use vb_runtime::shard::types::{
    MAX_COMMAND_QUEUE_CAPACITY, PendingTimer, PendingTimerKind, ShardCommand, ShardConfig,
    TimerDeadline, TimerDuration, TimerKind, TimerTick, is_valid_command_queue_capacity,
};

// ---------- Gate 1: PendingTimerKind is non_exhaustive ----------

/// `PendingTimerKind` is `#[non_exhaustive]`, preventing external crates
/// from exhaustively matching. This test just verifies we can construct
/// both known variants through the public API.

#[test]
fn pending_timer_kind_has_two_known_variants() {
    let wait = PendingTimerKind::Wait;
    let ask = PendingTimerKind::Ask;
    // If this compiles, the type exists. Verify discriminant inequality
    // by checking that they are not equal (different variants)
    assert_ne!(wait, ask);
}

// ---------- Gate 2: PendingTimer is Copy + Eq ----------

#[test]
fn pending_timer_implements_copy_and_eq() {
    let t1 = PendingTimer {
        step: vb_core::ids::StepIdx::ZERO,
        kind: PendingTimerKind::Wait,
        generation: 1,
        deadline: std::time::Instant::now(),
    };
    let t2 = t1;
    assert_eq!(t1, t2); // Eq + Copy work
}

// ---------- Gate 3: PendingTimerKind is its own discriminant ----------

#[test]
fn pending_timer_kind_discriminants_are_distinct() {
    assert_ne!(PendingTimerKind::Wait, PendingTimerKind::Ask);
    assert_eq!(PendingTimerKind::Wait, PendingTimerKind::Wait);
    assert_eq!(PendingTimerKind::Ask, PendingTimerKind::Ask);
}

// ---------- Gate 4: Timer-related ShardCommand variants exist ----------

#[test]
fn shard_command_timer_fired_variant_is_constructable() {
    let cmd = ShardCommand::TimerFired {
        run: vb_core::ids::RunId::new(1),
        generation: 1,
        deadline: std::time::Instant::now(),
        kind: PendingTimerKind::Wait,
    };
    // Verify the variant can be pattern-matched
    match cmd {
        ShardCommand::TimerFired {
            run,
            generation,
            kind,
            ..
        } => {
            assert_eq!(run, vb_core::ids::RunId::new(1));
            assert_eq!(generation, 1);
            assert_eq!(kind, PendingTimerKind::Wait);
        }
        _ => panic!("expected TimerFired variant"),
    }
}

// ---------- Gate 5: Capacity validation is const-evaluable ----------

#[test]
fn is_valid_command_queue_capacity_accepts_positive_values() {
    assert!(is_valid_command_queue_capacity(1));
    assert!(is_valid_command_queue_capacity(1024));
    assert!(is_valid_command_queue_capacity(MAX_COMMAND_QUEUE_CAPACITY));
}

#[test]
fn is_valid_command_queue_capacity_rejects_zero() {
    assert!(!is_valid_command_queue_capacity(0));
}

#[test]
fn is_valid_command_queue_capacity_rejects_exceeding_max() {
    assert!(!is_valid_command_queue_capacity(
        MAX_COMMAND_QUEUE_CAPACITY.saturating_add(1)
    ));
}

// ---------- Gate 6: ShardConfig default is valid ----------

#[test]
fn shard_config_default_has_valid_command_queue_capacity() {
    let config = ShardConfig::default();
    assert!(is_valid_command_queue_capacity(
        config.command_queue_capacity
    ));
    assert!(config.command_queue_capacity > 0);
    assert!(config.command_queue_capacity <= MAX_COMMAND_QUEUE_CAPACITY);
}

// ---------- Gate 7: Size assertions (bounded types) ----------

#[test]
fn pending_timer_size_is_reasonable() {
    // PendingTimer holds: StepIdx(2), PendingTimerKind(1), u64(8), Instant(platform)
    // Should be under 128 bytes on 64-bit platforms
    let size = size_of::<PendingTimer>();
    assert!(size <= 128, "PendingTimer size {size} exceeds budget");
}

#[test]
fn pending_timer_kind_is_small_copy_type() {
    let size = size_of::<PendingTimerKind>();
    assert!(size <= 8, "PendingTimerKind size {size} exceeds budget");
}

// ---------- Gate 8: ShardCommand size is bounded ----------

#[test]
fn shard_command_size_is_reasonable() {
    let size = size_of::<ShardCommand>();
    // ShardCommand is sent through bounded queues, must be reasonably sized
    assert!(size <= 1024, "ShardCommand size {size} exceeds budget");
}

// ---------- Gate 9: MAX_COMMAND_QUEUE_CAPACITY is positive ----------

const _MAX_CAPACITY_CHECK: () = {
    assert!(MAX_COMMAND_QUEUE_CAPACITY > 0);
};

#[test]
fn max_command_queue_capacity_is_positive() {
    assert!(MAX_COMMAND_QUEUE_CAPACITY > 0);
}

#[test]
fn max_command_queue_capacity_is_reasonable() {
    // 65536 is the documented bound
    assert_eq!(MAX_COMMAND_QUEUE_CAPACITY, 65536);
}

// ---------- Gate 10: Numeric timer types are small Copy types ----------

#[test]
fn timer_tick_is_small_copy_type() {
    let size = size_of::<TimerTick>();
    // TimerTick wraps a u64, should be exactly 8 bytes
    assert_eq!(size, 8, "TimerTick size {size} should be 8 (u64)");
}

#[test]
fn timer_duration_is_small_copy_type() {
    let size = size_of::<TimerDuration>();
    assert_eq!(size, 8, "TimerDuration size {size} should be 8 (u64)");
}

#[test]
fn timer_deadline_is_small_copy_type() {
    let size = size_of::<TimerDeadline>();
    assert_eq!(size, 8, "TimerDeadline size {size} should be 8 (u64)");
}

#[test]
fn timer_kind_is_small_copy_type() {
    let size = size_of::<TimerKind>();
    // TimerKind has 2 variants (Retry, DelayedAction(ActionId))
    // ActionId is u64, so tag + u64 = 16 bytes max
    assert!(size <= 16, "TimerKind size {size} exceeds budget");
}

// ---------- Gate 11: TimerKind is non_exhaustive ----------

#[test]
fn timer_kind_has_two_known_variants() {
    let retry = TimerKind::Retry;
    let delayed = TimerKind::DelayedAction(vb_core::ids::ActionId::new(1));
    assert_ne!(retry, delayed);
}

// ---------- Gate 12: Numeric timer types implement expected traits ----------

#[test]
fn timer_tick_implements_eq_and_copy() {
    let t1 = TimerTick::new(42);
    let t2 = t1;
    assert_eq!(t1, t2);
}

#[test]
fn timer_duration_implements_eq_and_copy() {
    let d1 = TimerDuration::new(10);
    let d2 = d1;
    assert_eq!(d1, d2);
}

#[test]
fn timer_deadline_implements_eq_and_copy() {
    let dl1 = TimerDeadline::new(30);
    let dl2 = dl1;
    assert_eq!(dl1, dl2);
}

#[test]
fn timer_tick_implements_ord() {
    assert!(TimerTick::new(1) < TimerTick::new(2));
    assert!(TimerTick::new(10) > TimerTick::new(5));
}

#[test]
fn timer_duration_implements_ord() {
    assert!(TimerDuration::new(1) < TimerDuration::new(2));
}

#[test]
fn timer_deadline_implements_ord() {
    assert!(TimerDeadline::new(1) < TimerDeadline::new(2));
}
