#![forbid(unsafe_code)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::expect_used,
    clippy::get_first,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]
//! Defense-in-depth property tests for the `collect_start` journal-preservation arm.
//!
//! Bead: vb-izu26
//! Proof-seeds: PS-F4-1 (start_millis preservation), PS-F4-2 (page_size / item_count
//! ratio reaches the preservation arm), PS-F4-4 (from_journal flag preservation).
//!
//! Background: the F4 obligation requires that `collect_start` (via
//! `upsert_started_collect`) preserves a journaled `start_millis` and keeps
//! `from_journal == true` when the prior `(RunId, collector_slot)` entry had
//! `from_journal == true`. The single-value `#[test]` at
//! `crates/vb_runtime/src/primitives/collect/tests.rs:1567` (commit 23021364a)
//! closes the obligation; the proptests below are defense-in-depth layers that
//! iterate the same shape across many `(start_millis, time_limit_ms,
//! page_size, item_count)` seeds.
//!
//! These proptests are NON-FLAKY: the preservation arm of
//! `upsert_started_collect` does not call `millis_since_epoch()`, so the
//! returned `start_millis` is a pure function of the seeded value. No
//! `prop_assume!` is required; the full `u64` range is exercised.

use proptest::prelude::*;

use vb_core::EngineSignal;
use vb_core::frame::RunFrame;
use vb_core::ids::{ListId, RunId, SlotIdx, StepIdx};
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_runtime::primitives::collect::{CollectPaginationState, CollectStates, collect_start};

// =============================================================================
// Test fixture helpers
// =============================================================================

/// Construct a fresh `RunFrame` with `step_count = 4, slot_count = 4`.
///
/// The dimensions match the existing single-value F4 test:
/// - `step_count = 4` leaves headroom for the `body` (StepIdx 1) and `done`
///   (StepIdx 2) targets used by `collect_start`.
/// - `slot_count = 4` is enough for the source slot (0) and collector slot (1).
fn fresh_frame() -> RunFrame {
    RunFrame::new(RunId::new(1), StepIdx::ZERO, 4, 4)
        .unwrap_or_else(|e| panic!("fresh_frame construction failed: {e:?}"))
}

/// Insert a list of `I64` values into `slot` of `run`.
fn write_list(run: &mut RunFrame, store: &mut ValueStore, slot: SlotIdx, items: Vec<i64>) {
    let list_id = store
        .insert_list(
            items
                .into_iter()
                .map(SlotValue::I64)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
        .unwrap_or_else(|e| panic!("insert_list failed: {e:?}"));
    run.write_slot(slot, SlotValue::List(list_id))
        .unwrap_or_else(|e| panic!("write_slot list failed: {e:?}"));
}

/// Drive `collect_start` with a 4-item source and `page_size = 2` so the
/// preservation arm of `upsert_started_collect` is reached. The function
/// returns the post-state captured from the side table.
fn drive_collect_start_and_capture(
    journaled: CollectPaginationState,
    page_size: u32,
    item_count: u32,
    limit: u32,
    time_limit_ms: Option<u64>,
) -> Result<(CollectPaginationState, EngineSignal), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = CollectStates::new();

    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    // Write a source list with `item_count` elements so `plan.page_len < plan.item_count`.
    write_list(
        &mut run,
        &mut store,
        source,
        (0..i64::from(item_count)).collect(),
    );

    // Seed the side table under the same (RunId, collector_slot) key that
    // collect_start will use. Use a known-valid `current_page` and `source`
    // list_id; the values are placeholders because the test only inspects
    // start_millis, from_journal, and time_limit_ms on the post-state.
    let seeded = CollectPaginationState {
        run_id: journaled.run_id,
        collector_slot: collector,
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: usize::try_from(page_size).map_err(|e| format!("page_size cast: {e}"))?,
        item_count: usize::try_from(item_count).map_err(|e| format!("item_count cast: {e}"))?,
        limit: usize::try_from(limit).map_err(|e| format!("limit cast: {e}"))?,
        time_limit_ms: journaled.time_limit_ms,
        start_millis: journaled.start_millis,
        from_journal: true,
    };
    states
        .upsert(seeded)
        .map_err(|e| format!("seed upsert failed: {e:?}"))?;

    let signal = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        limit,
        page_size,
        body,
        done,
        Some(collector),
        time_limit_ms,
    )
    .map_err(|e| format!("collect_start failed: {e:?}"))?;

    let after = states
        .capture_state(journaled.run_id, collector)
        .ok_or("pagination state missing after collect_start".to_owned())?;

    Ok((after, signal))
}

// =============================================================================
// Strategies
// =============================================================================

/// Generate a `start_millis` candidate. The full `u64` range is exercised to
/// defend against any future change that adds an arithmetic or branch
/// dependency on the seeded value.
fn start_millis_strategy() -> impl Strategy<Value = u64> {
    prop_oneof![
        Just(0u64),
        Just(1u64),
        Just(1_700_000_000_000u64),
        Just(u64::MAX),
        Just(u64::MAX - 1),
        0u64..=u64::MAX,
    ]
}

/// Generate an `Option<u64>` for `time_limit_ms` (None or Some). Allocations
/// are bounded; this generator never produces values that would overflow
/// during seed insertion.
fn time_limit_ms_strategy() -> impl Strategy<Value = Option<u64>> {
    prop_oneof![
        Just(None),
        Just(Some(0u64)),
        Just(Some(60_000u64)),
        Just(Some(u64::MAX)),
        any::<u64>().prop_map(Some),
    ]
}

// =============================================================================
// Property tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// PS-F4-1 + PS-F4-4: `collect_start` preserves a journaled
    /// `start_millis` and keeps `from_journal == true` across the full
    /// `u64` range of `start_millis` and across `Option<u64>` values of
    /// `time_limit_ms`.
    ///
    /// This is a defense-in-depth layer over the single-value `#[test]` at
    /// `crates/vb_runtime/src/primitives/collect/tests.rs:1567`. The test is
    /// non-flaky: the preservation arm of `upsert_started_collect` does not
    /// call `millis_since_epoch()`, so the returned `start_millis` is a pure
    /// function of the seeded value.
    #[test]
    fn proptest_collect_start_preserves_journaled_start_millis(
        start_millis in start_millis_strategy(),
        time_limit_ms in time_limit_ms_strategy(),
    ) {
        let run_id = RunId::new(1);
        let journaled = CollectPaginationState {
            run_id,
            collector_slot: SlotIdx::new(1),
            source: ListId::new(0),
            current_page: ListId::new(0),
            cursor: 0,
            page_size: 2,
            item_count: 4,
            limit: 100,
            time_limit_ms,
            start_millis,
            from_journal: true,
        };

        let (after, signal) = drive_collect_start_and_capture(
            journaled,
            /* page_size = */ 2,
            /* item_count = */ 4,
            /* limit = */ 100,
            time_limit_ms,
        )
        .map_err(|e| proptest::test_runner::TestCaseError::Fail(
            format!("drive_collect_start_and_capture failed: {e}").into(),
        ))?;

        prop_assert_eq!(
            signal,
            EngineSignal::Continue,
            "collect_start must signal Continue when page_len < item_count"
        );
        prop_assert_eq!(
            after.start_millis, start_millis,
            "start_millis must be preserved from journal (seeded={}, got={})",
            start_millis, after.start_millis
        );
        prop_assert!(
            after.from_journal,
            "from_journal flag must remain true after collect_start re-upsert"
        );
        prop_assert_eq!(
            after.time_limit_ms, time_limit_ms,
            "time_limit_ms must be carried through unchanged"
        );
    }

    /// PS-F4-2: Iterate over `(page_size, item_count)` pairs with
    /// `page_size < item_count` to defend against future changes to
    /// `finish_collect_start_page`'s short-circuit logic at
    /// `crates/vb_runtime/src/primitives/collect/mod.rs:147-150`. The
    /// preservation arm must be reached for every (page_size, item_count)
    /// pair where `page_size < item_count`, otherwise the F4 obligation
    /// is silently broken.
    #[test]
    fn proptest_collect_start_reaches_preservation_arm_for_paginated_sources(
        page_size_offset in 1u32..=16u32,
        item_count_extra in 1u32..=32u32,
    ) {
        let page_size = page_size_offset;
        let item_count = page_size.saturating_add(item_count_extra);
        // Force the preservation arm by requiring page_size < item_count.
        prop_assume!(page_size < item_count);
        // Bounded allocation: cap item_count so the test does not allocate
        // unbounded lists.
        let item_count = item_count.min(64);
        let page_size = page_size.min(item_count.saturating_sub(1).max(1));
        prop_assume!(page_size < item_count);

        let run_id = RunId::new(2);
        let journaled_start_millis: u64 = 0xDEAD_BEEF_CAFE_F00Du64;
        let journaled = CollectPaginationState {
            run_id,
            collector_slot: SlotIdx::new(1),
            source: ListId::new(0),
            current_page: ListId::new(0),
            cursor: 0,
            page_size: usize::try_from(page_size).unwrap_or(1),
            item_count: usize::try_from(item_count).unwrap_or(2),
            limit: 100,
            time_limit_ms: Some(60_000),
            start_millis: journaled_start_millis,
            from_journal: true,
        };

        let (after, _signal) = drive_collect_start_and_capture(
            journaled,
            page_size,
            item_count,
            /* limit = */ 100,
            /* time_limit_ms = */ Some(60_000),
        )
        .map_err(|e| proptest::test_runner::TestCaseError::Fail(
            format!("drive_collect_start_and_capture failed: {e}").into(),
        ))?;

        prop_assert_eq!(
            after.start_millis, journaled_start_millis,
            "start_millis must be preserved from journal (page_size={}, item_count={}, expected={}, got={})",
            page_size, item_count, journaled_start_millis, after.start_millis
        );
        prop_assert!(
            after.from_journal,
            "from_journal flag must remain true after collect_start re-upsert (page_size={}, item_count={})",
            page_size, item_count
        );
    }
}
