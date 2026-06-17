//! Collect pagination benchmarks.
//!
//! Measures per-page collection overhead for paginated list materialization.

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
    clippy::enum_variant_names,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::identity_op,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::if_same_then_else,
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
    clippy::manual_contains,
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
    clippy::multiple_bound_locations,
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
    missing_docs,
    unused_imports,
    unused_variables,
)]

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use vb_core::SlotIdx;
use vb_core::ids::ListId;
use vb_core::ids::RunId;
use vb_runtime::primitives::collect::{CollectPaginationState, CollectStates};

const BENCH_METADATA: &str = "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

/// Creates a CollectStates table with `count` entries.
fn collect_states_with_n_entries(count: usize) -> CollectStates {
    let mut states = CollectStates::new();
    let mut i = 0usize;
    while i < count {
        let state = CollectPaginationState {
            run_id: RunId::new(u64::try_from(i).unwrap_or(u64::MAX)),
            collector_slot: SlotIdx::new(u16::try_from(i % 256).unwrap_or(0)),
            source: ListId::new(u32::try_from(i).unwrap_or(u32::MAX)),
            current_page: ListId::new(u32::try_from(i).unwrap_or(u32::MAX)),
            cursor: i * 50,
            page_size: 50,
            item_count: 100,
            limit: 100,
            time_limit_ms: None,
            start_millis: 0,
            from_journal: false,
        };
        let _key = (state.run_id, state.collector_slot);
        let _ = states.upsert(state);
        i = i.saturating_add(1);
    }
    states
}

fn bench_collect_page(c: &mut Criterion) {
    let mut group = c.benchmark_group("collect_page");

    // First page collection — 100 items, page_size=50
    {
        let fixture_bytes = 100usize;
        group.throughput(Throughput::Bytes(fixture_bytes as u64));
        group.bench_function(
            metadata(
                "collect_page_first_page_small",
                fixture_bytes,
                "fixture=list_100_page_50;surface=collect_first_page",
            ),
            |b| {
                b.iter(|| {
                    let mut states = CollectStates::new();
                    let run_id = RunId::new(1);
                    let slot = SlotIdx::new(0);
                    // Insert first page state
                    let state = CollectPaginationState {
                        run_id,
                        collector_slot: slot,
                        source: ListId::new(1),
                        current_page: ListId::new(2),
                        cursor: 0,
                        page_size: 50,
                        item_count: 100,
                        limit: 100,
                        time_limit_ms: None,
                        start_millis: 0,
                        from_journal: false,
                    };
                    let result = states.upsert(state);
                    black_box(result.is_ok());
                    // Find it back
                    let found = states.find(run_id, slot, ListId::new(2));
                    // Exact assertion: must find the exact state we inserted
                    assert!(found.is_some(), "first page state must be findable");
                    let found_state = found.expect("state exists");
                    assert_eq!(found_state.cursor, 0, "cursor must be 0 on first page");
                    assert_eq!(found_state.page_size, 50, "page_size must be 50");
                    black_box(found);
                });
            },
        );
    }

    // Second page collection — cursor=50
    {
        let fixture_bytes = 100usize;
        group.bench_function(
            metadata(
                "collect_page_second_page",
                fixture_bytes,
                "fixture=list_100_page_50;surface=collect_second_page",
            ),
            |b| {
                b.iter(|| {
                    let mut states = CollectStates::new();
                    let run_id = RunId::new(1);
                    let slot = SlotIdx::new(0);
                    // Insert first page state
                    let first_state = CollectPaginationState {
                        run_id,
                        collector_slot: slot,
                        source: ListId::new(1),
                        current_page: ListId::new(2),
                        cursor: 0,
                        page_size: 50,
                        item_count: 100,
                        limit: 100,
                        time_limit_ms: None,
                        start_millis: 0,
                        from_journal: false,
                    };
                    states.upsert(first_state).expect("first upsert");

                    // Second page upsert
                    let second_state = CollectPaginationState {
                        run_id,
                        collector_slot: slot,
                        source: ListId::new(1),
                        current_page: ListId::new(3),
                        cursor: 50,
                        page_size: 50,
                        item_count: 100,
                        limit: 100,
                        time_limit_ms: None,
                        start_millis: 0,
                        from_journal: false,
                    };
                    let result = states.upsert(second_state);
                    black_box(result.is_ok());

                    // Find second page — exact assertion on cursor
                    let found = states.find(run_id, slot, ListId::new(3));
                    assert!(found.is_some(), "second page state must be findable");
                    let found_state = found.expect("state exists");
                    assert_eq!(
                        found_state.cursor, 50,
                        "cursor must advance to 50 on second page"
                    );
                    assert_eq!(
                        found_state.current_page,
                        ListId::new(3),
                        "current_page must be ListId(3)"
                    );
                    black_box(found);
                });
            },
        );
    }

    // Page exhausted — cursor == item_count
    {
        let fixture_bytes = 100usize;
        group.bench_function(
            metadata(
                "collect_page_exhausted",
                fixture_bytes,
                "fixture=list_100_page_50;surface=collect_exhausted",
            ),
            |b| {
                b.iter(|| {
                    let mut states = CollectStates::new();
                    let run_id = RunId::new(1);
                    let slot = SlotIdx::new(0);
                    // Insert exhausted state (cursor == item_count)
                    let state = CollectPaginationState {
                        run_id,
                        collector_slot: slot,
                        source: ListId::new(1),
                        current_page: ListId::new(2),
                        cursor: 100, // exhausted
                        page_size: 50,
                        item_count: 100,
                        limit: 100,
                        time_limit_ms: None,
                        start_millis: 0,
                        from_journal: false,
                    };
                    states.upsert(state).expect("upsert");

                    // Remove on exhaustion
                    states.remove(run_id, slot);

                    // Exact assertion: entry must be gone
                    let found = states.find(run_id, slot, ListId::new(2));
                    assert!(
                        found.is_none(),
                        "exhausted state must be removed from table"
                    );
                    black_box(found);
                });
            },
        );
    }

    // Large: 10 pages (1000 items, page_size=100)
    {
        let fixture_bytes = 1000usize;
        group.throughput(Throughput::Bytes(fixture_bytes as u64));
        group.bench_function(
            metadata(
                "collect_page_large_10_pages",
                fixture_bytes,
                "fixture=list_1000_page_100;surface=collect_10_pages",
            ),
            |b| {
                b.iter(|| {
                    let mut states = CollectStates::new();
                    let run_id = RunId::new(1);
                    let slot = SlotIdx::new(0);
                    // Simulate 10 pages
                    let mut page = 0u64;
                    let mut cursor = 0usize;
                    while page < 10 {
                        let state = CollectPaginationState {
                            run_id,
                            collector_slot: slot,
                            source: ListId::new(1),
                            current_page: ListId::new(u32::try_from(page).unwrap_or(u32::MAX)),
                            cursor,
                            page_size: 100,
                            item_count: 1000,
                            limit: 1000,
                            time_limit_ms: None,
                            start_millis: 0,
                            from_journal: false,
                        };
                        states.upsert(state).expect("upsert");
                        cursor = cursor.saturating_add(100);
                        page = page.saturating_add(1);
                    }
                    // Exact assertion: 10 entries must exist
                    let final_found = states.find(run_id, slot, ListId::new(9));
                    assert!(final_found.is_some(), "page 9 state must be findable");
                    let final_state = final_found.expect("state");
                    assert_eq!(
                        final_state.cursor, 900,
                        "final cursor must be 900 (10th page start)"
                    );
                    black_box(final_found);
                });
            },
        );
    }

    // Find existing entry in 100-entry table
    {
        let fixture_bytes = 100usize;
        group.throughput(Throughput::Elements(100));
        group.bench_function(
            metadata(
                "collect_page_find_existing",
                fixture_bytes,
                "fixture=100_entry_table;surface=collect_find_state",
            ),
            |b| {
                let states = collect_states_with_n_entries(100);
                b.iter(|| {
                    // Find entry at RunId(50), SlotIdx(50 % 256)
                    let run_id = RunId::new(50);
                    let slot = SlotIdx::new(u16::try_from(50 % 256).unwrap_or(0));
                    let found =
                        states.find(run_id, slot, ListId::new(u32::try_from(50).unwrap_or(0)));
                    // Exact assertion: must return Some with exact cursor value
                    assert!(
                        found.is_some(),
                        "entry at (50, slot) must be findable in 100-entry table"
                    );
                    let s = found.expect("exists");
                    assert_eq!(s.cursor, 50 * 50, "cursor must equal run_id * 50");
                    black_box(found);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_collect_page);
criterion_main!(benches);
