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
//! E2E Tests: Full timer lifecycle scenarios.
//!
//! Tests the complete timer lifecycle across creation, fire, cancel,
//! replay-determinism, and cross-shard behavior via the `TimerWheel` API
//! and publicly-constructable `PendingTimer` value objects.

use std::time::{Duration, Instant};
use vb_core::ids::{RunId, StepIdx};
use vb_runtime::shard::timer_wheel::TimerWheel;
use vb_runtime::shard::types::{PendingTimer, PendingTimerKind};

fn run(id: u64) -> RunId {
    RunId::new(id)
}

// ---------- E2E 1: Full timer lifecycle ----------

#[test]
fn full_timer_lifecycle_insert_fire_cancel_reinsert() {
    let mut wheel = TimerWheel::new();

    // Phase 1: Insert timer
    let d1 = Instant::now() + Duration::from_millis(100);
    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.len(), 1);
    assert_eq!(wheel.get_entry(run(1)).expect("entry").generation, 1);

    // Phase 2: Cancel before fire
    assert!(wheel.cancel(run(1)));
    assert!(wheel.is_empty());

    // Phase 3: Re-insert with new deadline
    let d2 = Instant::now() + Duration::from_millis(200);
    assert_eq!(wheel.insert(run(1), d2, PendingTimerKind::Ask), Ok(()));
    assert_eq!(wheel.get_entry(run(1)).expect("entry2").generation, 1);

    // Phase 4: Fire
    let fired = wheel.fire_expired(d2);
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].run, run(1));
    assert_eq!(fired[0].kind, PendingTimerKind::Ask);
    assert!(wheel.is_empty());
}

// ---------- E2E 2: Timer with deadline overflow guard (generation) ----------

#[test]
fn timer_lifecycle_generation_tracks_correctly_through_cycle() {
    let mut wheel = TimerWheel::new();
    let base = Instant::now();

    // Create → fire → create → fire → create → fire (3 cycles)
    for cycle in 0..3u64 {
        let deadline = base + Duration::from_millis(cycle * 100);
        assert_eq!(
            wheel.insert(run(1), deadline, PendingTimerKind::Wait),
            Ok(())
        );

        let fired = wheel.fire_expired(deadline);
        assert_eq!(fired.len(), 1);
        assert_eq!(
            fired[0].generation, 1,
            "each new cycle starts generation at 1"
        );
    }
}

// ---------- E2E 3: Replay determinism ----------

#[test]
fn replay_determinism_identical_inserts_produce_identical_fire_results() {
    let deadline = Instant::now() - Duration::from_millis(500);

    // First run
    let mut w1 = TimerWheel::new();
    assert_eq!(w1.insert(run(1), deadline, PendingTimerKind::Wait), Ok(()));
    assert_eq!(w1.insert(run(2), deadline, PendingTimerKind::Ask), Ok(()));
    let f1 = w1.fire_expired(Instant::now());

    // Second run (replay)
    let mut w2 = TimerWheel::new();
    assert_eq!(w2.insert(run(1), deadline, PendingTimerKind::Wait), Ok(()));
    assert_eq!(w2.insert(run(2), deadline, PendingTimerKind::Ask), Ok(()));
    let f2 = w2.fire_expired(Instant::now());

    // Results must be identical
    assert_eq!(f1.len(), f2.len());
    for (a, b) in f1.iter().zip(f2.iter()) {
        assert_eq!(a.run, b.run);
        assert_eq!(a.generation, b.generation);
        assert_eq!(a.kind, b.kind);
    }
}

#[test]
fn replay_determinism_same_state_produces_consistent_len_before_and_after_fire() {
    for trial in 0..5 {
        let mut wheel = TimerWheel::new();
        let past = Instant::now() - Duration::from_millis(100);

        assert_eq!(wheel.insert(run(1), past, PendingTimerKind::Wait), Ok(()));
        assert_eq!(wheel.insert(run(2), past, PendingTimerKind::Ask), Ok(()));
        assert_eq!(wheel.insert(run(3), past, PendingTimerKind::Wait), Ok(()));

        let before_len = wheel.len();
        assert_eq!(before_len, 3, "trial {trial}");

        let fired = wheel.fire_expired(Instant::now());
        assert_eq!(fired.len(), 3, "trial {trial}");
        assert!(wheel.is_empty(), "trial {trial}");
    }
}

// ---------- E2E 4: Multiple independent runs with interleaved timers ----------

#[test]
fn multiple_runs_with_different_deadlines_independent_fire() {
    let mut wheel = TimerWheel::new();
    let base = Instant::now();

    // Run 1: fires at +10ms and +30ms
    // Run 2: fires at +20ms
    let d1 = base + Duration::from_millis(10);
    let d2 = base + Duration::from_millis(20);
    let d1b = base + Duration::from_millis(30);

    assert_eq!(wheel.insert(run(1), d1, PendingTimerKind::Wait), Ok(()));
    assert_eq!(wheel.insert(run(2), d2, PendingTimerKind::Ask), Ok(()));

    // Fire at d1 — only run 1 fires
    let f1 = wheel.fire_expired(d1);
    assert_eq!(f1.len(), 1);
    assert_eq!(f1[0].run, run(1));

    // Re-insert run 1 at d1b
    assert_eq!(wheel.insert(run(1), d1b, PendingTimerKind::Wait), Ok(()));

    // Fire at d2 — run 2 fires
    let f2 = wheel.fire_expired(d2);
    assert_eq!(f2.len(), 1);
    assert_eq!(f2[0].run, run(2));

    // Fire at d1b — run 1 fires again
    let f3 = wheel.fire_expired(d1b);
    assert_eq!(f3.len(), 1);
    assert_eq!(f3[0].run, run(1));

    assert!(wheel.is_empty());
}

// ---------- E2E 5: Authority validation in complete cycle ----------

#[test]
fn authority_validation_prevents_mismatched_timer_from_firing() {
    let timer = PendingTimer {
        step: StepIdx::new(3),
        kind: PendingTimerKind::Wait,
        generation: 7,
        deadline: Instant::now(),
    };

    // Correct authority — should match
    assert!(timer.matches_authority(7, timer.deadline, PendingTimerKind::Wait));

    // Stale generation from earlier fire
    assert!(!timer.matches_authority(6, timer.deadline, PendingTimerKind::Wait));

    // Wrong kind (Ask vs Wait)
    assert!(!timer.matches_authority(7, timer.deadline, PendingTimerKind::Ask));

    // Wrong deadline
    let other_deadline = timer.deadline + Duration::from_secs(1);
    assert!(!timer.matches_authority(7, other_deadline, PendingTimerKind::Wait));
}

// ---------- E2E: Stress — many timers, many fires, many cancels ----------

#[test]
fn stress_many_timers_interleaved_with_cancels_and_fires() {
    let mut wheel = TimerWheel::new();
    let base = Instant::now();

    // Insert 50 timers
    for i in 0..50u64 {
        let deadline = base + Duration::from_millis(i * 10);
        assert_eq!(
            wheel.insert(run(i), deadline, PendingTimerKind::Wait),
            Ok(())
        );
    }
    assert_eq!(wheel.len(), 50);

    // Cancel every 3rd timer
    for i in (0..50u64).step_by(3) {
        wheel.cancel(run(i));
    }

    // Fire all past deadlines
    let far_future = base + Duration::from_secs(10);
    let fired = wheel.fire_expired(far_future);

    let expected = 50 - ((50 + 2) / 3); // 50 total - ~17 cancelled
    assert_eq!(fired.len(), expected);
    assert!(wheel.is_empty());
}
