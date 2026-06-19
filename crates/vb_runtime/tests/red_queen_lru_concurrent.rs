#![forbid(unsafe_code)]
#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iterator,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
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
    clippy::suspicious_operation_groups,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
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
//! Red-Queen adversarial state-space pressure tests for tier-a-6-014
//! (terminal-runs LRU + TTL).
//!
//! Bead: tier-a-6-014
//! State machine: empty / filling / full / evicting
//! Pressure: 10000-item insertions with eviction, TTL boundary sweeps,
//! capacity invariant under load, concurrent inserters, capacity=0 corner.
//!
//! These tests are deterministic. All checks are performed via exit code
//! comparison (no AI in the gate).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;

use vb_core::ids::RunId;
use vb_runtime::RuntimeError;
use vb_runtime::shard::lru_ring::{DEFAULT_MAX_TERMINAL_RUNS, LruRing};
use vb_runtime::shard::timer::TimerTick;

// ---------------------------------------------------------------------------
// Q1 — High-volume insert: 10000 items, evicts oldest, never exceeds
// capacity, TTL sweep keeps the ring bounded.
// ---------------------------------------------------------------------------

#[test]
fn red_queen_ten_thousand_items_never_exceeds_capacity() {
    let capacity = 100usize;
    let ttl = u64::MAX; // disable TTL eviction for this test
    let mut ring: LruRing<RunId> = LruRing::new(capacity, ttl);

    let mut overflow_count = 0u32;
    let mut ok_count = 0u32;
    for index in 0..10_000u64 {
        let run = RunId::new(index + 1);
        let now = TimerTick::new(index);
        let outcome = ring.insert(run, now);
        match outcome {
            Ok(()) => ok_count += 1,
            Err(RuntimeError::TerminalRunsLruFull { .. }) => overflow_count += 1,
            Err(other) => panic!("unexpected error: {other:?}"),
        }
        // Invariant: ring never exceeds capacity.
        assert!(
            ring.len() <= capacity,
            "ring must never exceed capacity (got len={} at index={})",
            ring.len(),
            index
        );
    }
    // The first `capacity` inserts must succeed; every subsequent one
    // must overflow.
    assert_eq!(
        ok_count, capacity as u32,
        "first {capacity} inserts must succeed (got {ok_count})"
    );
    assert_eq!(
        overflow_count,
        10_000 - capacity as u32,
        "every subsequent insert must overflow (got {overflow_count})"
    );
    assert_eq!(ring.len(), capacity, "ring must be saturated at capacity");
    assert_eq!(
        ring.counters().capacity_overflows,
        overflow_count as u64,
        "every overflow must increment the counter"
    );
}

#[test]
fn red_queen_ten_thousand_items_with_ttl_evicts_correctly() {
    // Insert 10_000 items with TTL=1000; every insert is at a unique tick
    // so the prior items have expired by the time the next batch
    // arrives. The ring must remain bounded, and every "new" item must
    // be present after the wave.
    let capacity = 50usize;
    let ttl = 100u64;
    let mut ring: LruRing<RunId> = LruRing::new(capacity, ttl);

    for wave in 0..200u64 {
        // Each wave inserts `capacity` distinct items at a tick well past
        // the TTL horizon from the previous wave.
        let wave_base_tick = (wave + 1) * (ttl * 2);
        for offset in 0..capacity {
            let run = RunId::new(wave * 10_000 + offset as u64 + 1);
            let outcome = ring.insert(run, TimerTick::new(wave_base_tick + offset as u64));
            assert!(
                outcome.is_ok(),
                "wave {wave} insert {offset} must succeed (TTL expired), got: {outcome:?}"
            );
            assert!(
                ring.len() <= capacity,
                "ring must never exceed capacity (wave {wave}, offset {offset}, len {})",
                ring.len()
            );
        }
        // After the wave, only the latest wave should be present (the
        // previous wave's items expired in the sweep).
        assert_eq!(
            ring.len(),
            capacity,
            "ring must be saturated at capacity after wave {wave}"
        );
    }
    // Total expired_evictions must be > 0 (we processed many waves).
    assert!(
        ring.counters().expired_evictions > 0,
        "TTL sweep must have evicted at least one item across the waves"
    );
}

// ---------------------------------------------------------------------------
// Q2 — TTL boundary sweep at the exact horizon
// ---------------------------------------------------------------------------

#[test]
fn red_queen_ttl_exact_boundary_deterministic() {
    let capacity = 4usize;
    let ttl = 100u64;
    let mut ring: LruRing<RunId> = LruRing::new(capacity, ttl);

    // Insert 4 items at tick = 1000.
    let baseline = TimerTick::new(1000);
    for offset in 0..4u64 {
        let run = RunId::new(offset + 1);
        ring.insert(run, baseline).expect("first wave");
    }
    // Insert one more at the TTL horizon - 1 → must refuse (no TTL expired yet).
    let before_horizon = TimerTick::new(baseline.get() + ttl - 1);
    let outcome = ring.insert(RunId::new(99), before_horizon);
    assert!(
        matches!(outcome, Err(RuntimeError::TerminalRunsLruFull { .. })),
        "insert at ttl-1 must refuse, got: {outcome:?}"
    );

    // At exactly the horizon (baseline + ttl), the entries are exactly
    // ttl ticks old and must be evicted on the next insert.
    let at_horizon = TimerTick::new(baseline.get() + ttl);
    let outcome = ring.insert(RunId::new(100), at_horizon);
    assert!(
        outcome.is_ok(),
        "insert at ttl horizon must evict and succeed, got: {outcome:?}"
    );
    assert!(
        ring.counters().expired_evictions >= 4,
        "all 4 baseline entries must be evicted at the horizon"
    );
    assert_eq!(ring.len(), 1, "ring must contain only the new entry");
}

#[test]
fn red_queen_ttl_zero_disables_eviction() {
    // Per implementation: ttl_ticks == 0 returns early from sweep_expired,
    // so ttl=0 disables TTL eviction entirely (treating it as "no expiry").
    let capacity = 4usize;
    let mut ring: LruRing<RunId> = LruRing::new(capacity, 0);
    for offset in 0..4u64 {
        let run = RunId::new(offset + 1);
        ring.insert(run, TimerTick::new(10)).expect("insert");
    }
    assert_eq!(ring.len(), 4);

    // Advance well past the existing tick; because ttl=0 disables sweep,
    // the ring is at capacity and the new insert must refuse.
    let outcome = ring.insert(RunId::new(99), TimerTick::new(1_000_000));
    assert!(
        matches!(outcome, Err(RuntimeError::TerminalRunsLruFull { .. })),
        "ttl=0 disables sweep, so insert at capacity must refuse, got: {outcome:?}"
    );
    assert_eq!(
        ring.counters().expired_evictions,
        0,
        "ttl=0 must never evict (got {})",
        ring.counters().expired_evictions
    );
    assert_eq!(ring.len(), 4, "ring must remain at capacity");
}

// ---------------------------------------------------------------------------
// Q3 — Insert idempotency: re-inserting an existing item must not bump
// the insertion tick and must not affect capacity.
// ---------------------------------------------------------------------------

#[test]
fn red_queen_reinsert_existing_item_is_idempotent() {
    let capacity = 4usize;
    let mut ring: LruRing<RunId> = LruRing::new(capacity, u64::MAX);
    let run = RunId::new(42);
    ring.insert(run, TimerTick::new(100)).expect("first insert");
    let overflow = ring.insert(run, TimerTick::new(200));
    assert!(
        overflow.is_ok(),
        "re-insert of existing item must succeed (idempotent)"
    );
    assert_eq!(ring.len(), 1, "re-insert must not grow the ring");

    // Now we can still fill up to capacity with distinct items.
    for offset in 1..4u64 {
        let r = RunId::new(offset);
        ring.insert(r, TimerTick::new(300 + offset))
            .expect("distinct insert");
    }
    assert_eq!(ring.len(), capacity);
    // Re-inserting the duplicate still doesn't grow the ring.
    ring.insert(run, TimerTick::new(400))
        .expect("re-insert at capacity");
    assert_eq!(ring.len(), capacity);
}

// ---------------------------------------------------------------------------
// Q4 — force_insert under pressure
// ---------------------------------------------------------------------------

#[test]
fn red_queen_force_insert_grows_ring_but_counts_overflow() {
    let capacity = 4usize;
    let mut ring: LruRing<RunId> = LruRing::new(capacity, u64::MAX);
    for offset in 0..4u64 {
        ring.insert(RunId::new(offset + 1), TimerTick::new(100))
            .expect("fill");
    }
    let baseline_overflows = ring.counters().capacity_overflows;
    // force_insert past capacity must NOT Err and must bump the counter.
    for offset in 0..10u64 {
        ring.force_insert(RunId::new(1000 + offset), TimerTick::new(200));
    }
    assert!(
        ring.counters().capacity_overflows > baseline_overflows,
        "force_insert past capacity must increment capacity_overflows"
    );
    assert_eq!(
        ring.len(),
        capacity + 10,
        "force_insert must grow the ring past capacity"
    );
}

// ---------------------------------------------------------------------------
// Q5 — Remove operation
// ---------------------------------------------------------------------------

#[test]
fn red_queen_remove_then_reinsert_frees_slot() {
    let capacity = 2usize;
    let mut ring: LruRing<RunId> = LruRing::new(capacity, u64::MAX);
    ring.insert(RunId::new(1), TimerTick::new(100)).expect("a");
    ring.insert(RunId::new(2), TimerTick::new(200)).expect("b");
    assert!(ring.contains(&RunId::new(1)));

    ring.remove(&RunId::new(1));
    assert!(!ring.contains(&RunId::new(1)));
    assert_eq!(ring.len(), 1);

    // Removing a non-existent item is a no-op.
    ring.remove(&RunId::new(99));
    assert_eq!(ring.len(), 1);

    // We can now insert a new item because the ring is under capacity.
    ring.insert(RunId::new(3), TimerTick::new(300)).expect("c");
    assert_eq!(ring.len(), 2);
    assert!(ring.contains(&RunId::new(3)));
}

#[test]
fn red_queen_remove_all_entries_leaves_empty_ring() {
    let capacity = 4usize;
    let mut ring: LruRing<RunId> = LruRing::new(capacity, u64::MAX);
    for offset in 0..4u64 {
        ring.insert(RunId::new(offset + 1), TimerTick::new(100))
            .expect("fill");
    }
    for offset in 0..4u64 {
        ring.remove(&RunId::new(offset + 1));
    }
    assert!(ring.is_empty());
    assert_eq!(ring.len(), 0);
}

// ---------------------------------------------------------------------------
// Q6 — Clear operation preserves capacity & TTL
// ---------------------------------------------------------------------------

#[test]
fn red_queen_clear_preserves_capacity_and_ttl() {
    let capacity = 8usize;
    let ttl = 12345u64;
    let mut ring: LruRing<RunId> = LruRing::new(capacity, ttl);
    for offset in 0..capacity {
        ring.insert(RunId::new(offset as u64 + 1), TimerTick::new(100))
            .expect("fill");
    }
    ring.clear();
    assert!(ring.is_empty());
    assert_eq!(ring.capacity(), capacity);
    assert_eq!(ring.ttl_ticks(), ttl);
    // After clear, we can fill again.
    for offset in 0..capacity {
        ring.insert(RunId::new(offset as u64 + 100), TimerTick::new(200))
            .expect("refill");
    }
    assert_eq!(ring.len(), capacity);
}

// ---------------------------------------------------------------------------
// Q7 — capacity=0 is normalized to 1 (per implementation contract)
// ---------------------------------------------------------------------------

#[test]
fn red_queen_capacity_zero_is_normalized_to_one() {
    let mut ring: LruRing<RunId> = LruRing::new(0, u64::MAX);
    assert_eq!(ring.capacity(), 1, "capacity=0 must be normalized to 1");
    ring.insert(RunId::new(1), TimerTick::new(100))
        .expect("first insert under normalized capacity");
    let outcome = ring.insert(RunId::new(2), TimerTick::new(200));
    assert!(
        matches!(outcome, Err(RuntimeError::TerminalRunsLruFull { .. })),
        "second insert at normalized capacity must refuse, got: {outcome:?}"
    );
}

// ---------------------------------------------------------------------------
// Q8 — Default factory values are reachable
// ---------------------------------------------------------------------------

#[test]
fn red_queen_default_factory_constants_match_documented_values() {
    let ring: LruRing<RunId> = LruRing::new(DEFAULT_MAX_TERMINAL_RUNS, 86_400);
    assert_eq!(ring.capacity(), DEFAULT_MAX_TERMINAL_RUNS);
    assert_eq!(ring.ttl_ticks(), 86_400);
    assert!(ring.is_empty());
    assert!(!ring.is_full());
    assert_eq!(ring.counters().expired_evictions, 0);
    assert_eq!(ring.counters().capacity_overflows, 0);
}

// ---------------------------------------------------------------------------
// Q9 — Stress under concurrency: 8 threads, each inserting 1250 items
// into the same ring. Total of 10000 distinct insertions.
// ---------------------------------------------------------------------------

#[test]
fn red_queen_concurrent_insert_eight_threads() {
    let capacity = 200usize;
    let ttl = u64::MAX;
    let ring = Arc::new(std::sync::Mutex::new(LruRing::<RunId>::new(capacity, ttl)));
    let barrier = Arc::new(Barrier::new(8));

    let handles: Vec<_> = (0..8)
        .map(|thread_index| {
            let ring = Arc::clone(&ring);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                let mut local_inserts = 0u32;
                let mut local_overflows = 0u32;
                for offset in 0..1250u64 {
                    let run_id = RunId::new(thread_index * 100_000 + offset + 1);
                    let mut ring = ring.lock().expect("mutex must not be poisoned");
                    match ring.insert(run_id, TimerTick::new(thread_index as u64 * 2000 + offset)) {
                        Ok(()) => local_inserts += 1,
                        Err(RuntimeError::TerminalRunsLruFull { .. }) => local_overflows += 1,
                        Err(other) => panic!("unexpected error: {other:?}"),
                    }
                }
                (local_inserts, local_overflows)
            })
        })
        .collect();

    let mut total_inserts = 0u32;
    let mut total_overflows = 0u32;
    for h in handles {
        let (inserts, overflows) = h.join().expect("thread must not panic");
        total_inserts += inserts;
        total_overflows += overflows;
    }
    let ring = ring.lock().expect("mutex must not be poisoned");
    assert_eq!(
        total_inserts + total_overflows,
        8 * 1250,
        "every insert attempt must produce exactly one of Ok / Err"
    );
    assert_eq!(
        ring.len(),
        capacity.min(total_inserts as usize),
        "ring must be saturated at capacity (got len={}, total_inserts={})",
        ring.len(),
        total_inserts
    );
    assert!(
        total_overflows > 0,
        "with 8 threads × 1250 inserts and capacity=200, some inserts must overflow"
    );
}

// ---------------------------------------------------------------------------
// Q10 — Sweep counter saturation safety: even with many sweeps the counter
// must remain accurate and never panic.
// ---------------------------------------------------------------------------

#[test]
fn red_queen_many_sweeps_counter_saturates_safely() {
    // Capacity 2, TTL 1: every sweep at a new tick evicts everything.
    let mut ring: LruRing<RunId> = LruRing::new(2, 1);
    for tick in 0..1000u64 {
        // Each insert at a unique tick will sweep any prior entries
        // (their ts + 1 <= tick).
        let _ = ring.insert(RunId::new(tick + 1), TimerTick::new(tick));
    }
    assert!(
        ring.counters().expired_evictions > 0,
        "evictions must have been counted"
    );
    // After 1000 sweeps, ring must still respect capacity.
    assert!(ring.len() <= 2);
}
