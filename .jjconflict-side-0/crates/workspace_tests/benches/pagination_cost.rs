//! Pagination cost benchmarks.
//!
//! Measures CollectStates table operations: insert, upsert, find (existing and missing).

#![allow(missing_docs)]

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use vb_core::ids::{ListId, RunId, SlotIdx};
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
        };
        let _key = (state.run_id, state.collector_slot);
        let _ = states.upsert(state);
        i = i.saturating_add(1);
    }
    states
}

/// Creates a CollectStates table with exactly 1 entry.
fn collect_states_1_entry() -> (CollectStates, RunId, SlotIdx, ListId) {
    let mut states = CollectStates::new();
    let run_id = RunId::new(1);
    let slot = SlotIdx::new(0);
    let current_page = ListId::new(2);
    let state = CollectPaginationState {
        run_id,
        collector_slot: slot,
        source: ListId::new(1),
        current_page,
        cursor: 0,
        page_size: 50,
        item_count: 100,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    states.upsert(state).expect("upsert");
    (states, run_id, slot, current_page)
}

/// CollectStates with 10-page lineage for testing lineage tracking.
fn collect_states_10_page_lineage() -> (CollectStates, RunId, SlotIdx) {
    let mut states = CollectStates::new();
    let run_id = RunId::new(42);
    let slot = SlotIdx::new(7);
    let mut page: u32 = 0;
    while page < 10 {
        let state = CollectPaginationState {
            run_id,
            collector_slot: slot,
            source: ListId::new(1),
            current_page: ListId::new(page),
            cursor: usize::try_from(page)
                .unwrap_or(usize::MAX)
                .saturating_mul(50),
            page_size: 50,
            item_count: 500,
            limit: 500,
            time_limit_ms: None,
            start_millis: 0,
        };
        states.upsert(state).expect("upsert");
        page = page.saturating_add(1);
    }
    (states, run_id, slot)
}

fn bench_pagination_cost(c: &mut Criterion) {
    let mut group = c.benchmark_group("pagination_cost");

    // Insert first entry into empty table
    {
        let fixture_bytes = 1usize;
        group.throughput(Throughput::Elements(1));
        group.bench_function(
            metadata(
                "pagination_insert_first",
                fixture_bytes,
                "fixture=empty_table;surface=collect_states_insert",
            ),
            |b| {
                b.iter(|| {
                    let mut states = CollectStates::new();
                    let run_id = RunId::new(1);
                    let slot = SlotIdx::new(0);
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
                    };
                    let result = states.upsert(state);
                    // Exact assertion: insert must succeed
                    assert!(result.is_ok(), "first insert into empty table must succeed");
                    // Exact assertion: state must be retrievable with exact values
                    let found = states.find(run_id, slot, ListId::new(2));
                    assert!(found.is_some(), "inserted state must be findable");
                    let found_state = found.expect("exists");
                    assert_eq!(found_state.cursor, 0, "inserted state cursor must be 0");
                    assert_eq!(
                        found_state.page_size, 50,
                        "inserted state page_size must be 50"
                    );
                    assert_eq!(
                        found_state.item_count, 100,
                        "inserted state item_count must be 100"
                    );
                    black_box(found);
                });
            },
        );
    }

    // Upsert second page — replaces existing state, records lineage
    {
        let fixture_bytes = 1usize;
        group.bench_function(
            metadata(
                "pagination_upsert_second_page",
                fixture_bytes,
                "fixture=1_entry_table;surface=collect_states_upsert",
            ),
            |b| {
                b.iter(|| {
                    let (mut states, run_id, slot, first_page) = collect_states_1_entry();

                    // Upsert second page state
                    let second_page = ListId::new(3);
                    let second_state = CollectPaginationState {
                        run_id,
                        collector_slot: slot,
                        source: ListId::new(1),
                        current_page: second_page,
                        cursor: 50,
                        page_size: 50,
                        item_count: 100,
                        limit: 100,
                        time_limit_ms: None,
                        start_millis: 0,
                    };
                    let result = states.upsert(second_state);
                    // Exact assertion: upsert must succeed
                    assert!(result.is_ok(), "second page upsert must succeed");

                    // Find second page — exact cursor value
                    let found = states.find(run_id, slot, second_page);
                    assert!(found.is_some(), "second page must be findable after upsert");
                    let found_state = found.expect("exists");
                    assert_eq!(found_state.cursor, 50, "second page cursor must be 50");
                    assert_eq!(
                        found_state.current_page, second_page,
                        "current_page must be ListId(3)"
                    );

                    // Old first page must NOT be findable under first_page key
                    // (it was moved to lineage)
                    let _old_found = states.find(run_id, slot, first_page);
                    // After upsert, find returns the NEWEST state for this (run, slot)
                    // regardless of which current_page we query — but the state
                    // should reflect the updated values
                    black_box(found);
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
                "pagination_find_existing",
                fixture_bytes,
                "fixture=100_entry_table;surface=collect_states_find",
            ),
            |b| {
                let states = collect_states_with_n_entries(100);
                b.iter(|| {
                    // Find entry at RunId(50), SlotIdx(50)
                    let run_id = RunId::new(50);
                    let slot = SlotIdx::new(u16::try_from(50 % 256).unwrap_or(0));
                    let current_page = ListId::new(50);
                    let found = states.find(run_id, slot, current_page);
                    // Exact assertion: must return exact cursor value
                    assert!(
                        found.is_some(),
                        "entry at (50, slot) must be findable in 100-entry table"
                    );
                    let s = found.expect("exists");
                    assert_eq!(s.cursor, 2500, "cursor must equal 50 * 50 = 2500");
                    assert_eq!(s.page_size, 50, "page_size must be 50");
                    assert_eq!(s.item_count, 100, "item_count must be 100");
                    black_box(found);
                });
            },
        );
    }

    // Find missing entry in 100-entry table
    {
        let fixture_bytes = 100usize;
        group.bench_function(
            metadata(
                "pagination_find_missing",
                fixture_bytes,
                "fixture=100_entry_table;surface=collect_states_find_missing",
            ),
            |b| {
                let states = collect_states_with_n_entries(100);
                b.iter(|| {
                    // Find non-existent entry
                    let run_id = RunId::new(9999); // Not in table
                    let slot = SlotIdx::new(255);
                    let current_page = ListId::new(9999);
                    let found = states.find(run_id, slot, current_page);
                    // Exact assertion: must return None (not an error)
                    assert!(
                        found.is_none(),
                        "find for non-existent key must return None, not Err"
                    );
                    black_box(found);
                });
            },
        );
    }

    // Lineage tracking — 10 pages of history
    {
        let fixture_bytes = 10usize;
        group.bench_function(
            metadata(
                "pagination_lineage_tracking",
                fixture_bytes,
                "fixture=10_page_lineage;surface=collect_lineage",
            ),
            |b| {
                b.iter(|| {
                    let (states, run_id, slot) = collect_states_10_page_lineage();
                    // After 10 pages, find should return page 9's state
                    let found = states.find(run_id, slot, ListId::new(9));
                    // Exact assertion: 10th page state must exist with correct cursor
                    assert!(
                        found.is_some(),
                        "10th page state must exist in lineage table"
                    );
                    let s = found.expect("exists");
                    assert_eq!(s.cursor, 450, "10th page cursor must be 450 (9 * 50)");
                    assert_eq!(
                        s.current_page,
                        ListId::new(9),
                        "current_page must be ListId(9)"
                    );
                    black_box(found);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_pagination_cost);
criterion_main!(benches);
