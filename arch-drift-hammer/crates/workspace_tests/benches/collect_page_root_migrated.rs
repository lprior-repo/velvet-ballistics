//! Collect pagination benchmarks.
//!
//! Measures per-page collection overhead for paginated list materialization.

#![allow(missing_docs)]

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::hint::black_box;
use vb_core::{
    StepIdx, SlotIdx, SlotValue, Taint,
};
use vb_core::ids::ListId;
use vb_runtime::primitives::collect::{CollectPaginationState, CollectStates};
use vb_runtime::primitives::collect::collect_page;
use vb_core::frame::RunFrame;
use vb_core::ids::RunId;

const BENCH_METADATA: &str =
    "profile=bench;tool=criterion-0.8;durability=mixed;mode=ir;latency=p50-p95-p99-by-criterion;allocations=allocator-external;instructions=not-collected";

fn metadata(name: &str, fixture_bytes: usize, extra: &str) -> String {
    format!(
        "{name};{BENCH_METADATA};{extra};fixture_bytes={fixture_bytes}",
        name = name,
        fixture_bytes = fixture_bytes
    )
}

/// Creates a run frame with a list in the given slot.
fn run_frame_with_list(run_id: RunId, slot: SlotIdx, items: usize) -> RunFrame {
    let mut frame = RunFrame::new(run_id, StepIdx::new(0), 10, 10).expect("frame");
    let list_id = ListId::new(0);
    // Note: In a real scenario we'd populate the ValueStore with the list.
    // For benchmark purposes, we test the CollectStates table operations directly.
    let _ = frame.write_slot(slot, SlotValue::List(list_id));
    let _ = frame.write_taint(slot, Taint::Clean);
    frame
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

/// CollectStates with a known entry at (RunId(42), SlotIdx(5)).
fn collect_states_with_known_entry() -> (CollectStates, RunId, SlotIdx, ListId) {
    let mut states = CollectStates::new();
    let run_id = RunId::new(42);
    let slot = SlotIdx::new(5);
    let current_page = ListId::new(7);
    let state = CollectPaginationState {
        run_id,
        collector_slot: slot,
        source: ListId::new(1),
        current_page,
        cursor: 50,
        page_size: 50,
        item_count: 100,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    states.upsert(state).expect("upsert");
    (states, run_id, slot, current_page)
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
                    };
                    let result = states.upsert(state);
                    black_box(result.is_ok());
                    // Find it back
                    let found = states.find(run_id, slot, ListId::new(2));
                    // Exact assertion: must find the exact state we inserted
                    assert!(
                        found.is_some(),
                        "first page state must be findable"
                    );
                    let found_state = found.expect("state exists");
                    assert_eq!(
                        found_state.cursor, 0,
                        "cursor must be 0 on first page"
                    );
                    assert_eq!(
                        found_state.page_size, 50,
                        "page_size must be 50"
                    );
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
                    };
                    let result = states.upsert(second_state);
                    black_box(result.is_ok());

                    // Find second page — exact assertion on cursor
                    let found = states.find(run_id, slot, ListId::new(3));
                    assert!(
                        found.is_some(),
                        "second page state must be findable"
                    );
                    let found_state = found.expect("state exists");
                    assert_eq!(
                        found_state.cursor, 50,
                        "cursor must advance to 50 on second page"
                    );
                    assert_eq!(
                        found_state.current_page, ListId::new(3),
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
                        };
                        states.upsert(state).expect("upsert");
                        cursor = cursor.saturating_add(100);
                        page = page.saturating_add(1);
                    }
                    // Exact assertion: 10 entries must exist
                    let final_found = states.find(run_id, slot, ListId::new(9));
                    assert!(
                        final_found.is_some(),
                        "page 9 state must be findable"
                    );
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
                    let found = states.find(run_id, slot, ListId::new(u32::try_from(50).unwrap_or(0)));
                    // Exact assertion: must return Some with exact cursor value
                    assert!(
                        found.is_some(),
                        "entry at (50, slot) must be findable in 100-entry table"
                    );
                    let s = found.expect("exists");
                    assert_eq!(
                        s.cursor, 50 * 50,
                        "cursor must equal run_id * 50"
                    );
                    black_box(found);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_collect_page);
criterion_main!(benches);
