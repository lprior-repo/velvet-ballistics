bead_id: vb-qi37.3
bead_title: runtime: Prove collect pagination durability and hydration
phase: State 2 - Codebase exploration and scope

## Master-doc clauses

- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/velvet-ballistics-MASTER.md:95` forbids unbounded pagination.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/velvet-ballistics-MASTER.md:459` defines `ResourceContract.max_collect_items`.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/velvet-ballistics-MASTER.md:635-638` requires final IR nodes `CollectStart`, `CollectPage`, `CollectNext`, and `CollectFinish`.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/velvet-ballistics-MASTER.md:1478` makes collect Phase 26 responsible for page/item/time limits, pagination state, and finish materialization.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/velvet-ballistics-MASTER.md:1508-1509` says storage/recovery and runtime still need executable evidence for full live-frame hydration and collect pagination recovery.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/velvet-ballistics-MASTER.md:2307-2356` defines per-node collect semantics, required slot writes, journal evidence, transitions, and errors.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/velvet-ballistics-MASTER.md:3414` requires replacing collect global `Mutex<Vec>` with per-run pagination state.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/velvet-ballistics-MASTER.md:3697` Phase 56 requires per-run pagination state, time limit, `RunId`-keyed state, concurrent-run tests, time-limit tests, and crash-recovery tests.

## Runtime/collect files

- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_runtime/src/primitives/collect.rs`
  - `CollectPaginationState`: durable per-run/per-collector pagination state with `run_id`, `collector_slot`, `source`, `current_page`, `cursor`, `page_size`, `item_count`, `limit`, `time_limit_ms`, and `start_millis`.
  - `CollectStates`: side table keyed by `(RunId, SlotIdx)`, currently backed by `HashMap<(RunId, SlotIdx), CollectPaginationState>`.
  - `CollectStates::upsert`, `find`, `remove`, `capture_extra`, `capture_state`, `hydrate_extra`, `hydrate_journal_events`.
  - `hydrate_collect_states_from_recovered_journal` builds collect side state from recovered `JournalEvent` values.
  - `collect_start`, `collect_page`, `collect_next`, `collect_finish` implement primitive behavior.
  - Current stale/duplicate/out-of-order rejection path is `CollectStates::find(run, slot, current_page).ok_or(EngineError::InvalidCompiledWorkflow { reason: "collect pagination state missing" })`; this is typed as `EngineError` but not a specific stale/duplicate/out-of-order variant.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_runtime/src/engine/execute.rs`
  - `execute_node_full` dispatches `CompiledNodeKind::CollectStart`, `CollectPage`, `CollectNext`, and `CollectFinish` into collect primitive functions.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_runtime/src/engine/drive.rs`
  - `drive_deterministic_full` passes mutable `CollectStates` through execution.
  - `collect_written_slot` identifies collect output/collector slots.
  - Slot evidence captures `collect_states.capture_state(run.run_id(), slot)` and pushes `EvidenceEvent::SlotWritten { extra }`.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_runtime/src/engine/types.rs`
  - `EvidenceEvent::SlotWritten { extra: Option<CollectPaginationState> }` and `EvidenceCollector::push_slot_written_with_extra` carry collect durable extras.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_runtime/src/shard/types.rs`
  - `RunState.collect_states: CollectStates` stores per-run active collect state inside shard-owned run state.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_runtime/src/shard/impl_.rs`
  - `flush_slot_written` postcard-encodes `CollectPaginationState` extra and appends `RuntimeJournalEvent::SlotWritten`.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_core/src/workflow/mod.rs`
  - `CompiledNodeKind::{CollectStart, CollectPage, CollectNext, CollectFinish}` public IR variants.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_core/src/errors.rs`
  - Existing collect errors: `CollectPageLimitExceeded`, `CollectItemLimitExceeded`, `CollectTimeLimitExceeded`. No specific `DuplicateCollectPage`, `StaleCollectPage`, or `OutOfOrderCollectPage` variant was found.

## Storage/journal/recovery files

- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_storage/src/events.rs`
  - `JournalEvent::SlotWrittenEvent { run, seq, slot, value, extra }` persists slot value plus optional extra bytes.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_runtime/src/journal.rs`
  - `RuntimeJournalEvent::SlotWritten { run, slot, value, taint, extra }` includes extra bytes.
  - `StorageRuntimeJournal::boundary_storage_event` maps runtime slot writes to storage `SlotWrittenEvent` and calls `encoded_slot_taint_extra`.
  - `encoded_slot_taint_extra` stores `extra` if present, otherwise postcard-encoded taint. Risk: collect hydration treats every `SlotWrittenEvent.extra` as collect state; non-collect taint extras can decode-fail unless filtered by caller/context.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_storage/src/journal.rs`
  - `FjallJournal::append_strict`, `append_strict_batch`, `append_unpersisted`, `append_queued_unpersisted`, `events_for_run`, `events_for_run_from` provide append/replay surfaces.
  - Duplicate `(run, seq)` events return `JournalError::DuplicateEvent`; replay sequence gaps return `JournalError::SequenceGap` via `validate_replayed_event`.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_storage/src/recovery/recover.rs`
  - `recover_runtime_summary`, `recover_runtime_frame_seed`, `recover_all_incomplete_runs`, `verify_digests`.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_storage/src/recovery/hydrate.rs`
  - `hydrate_run_frame`, `hydrate_run_frame_from_events` reconstruct `RunFrame` from snapshots/events but do not hydrate `CollectStates` directly.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_storage/src/recovery/hydrate_support.rs`
  - `apply_tail_events` applies `SlotWrittenEvent.value` to frame but ignores `SlotWrittenEvent.extra` except as carried bytes for separate collect hydration.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_storage/src/recovery/replay/core.rs`
  - `recover_full_journal` is the recovered-event source used by collect tests.

## Test surfaces

- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_runtime/src/collect_tests.rs`
  - Empty/final page: `collect_start_jumps_to_done_when_source_empty`, `collect_start_exact_page_limit_finishes_without_active_pagination_state`, `collect_next_writes_empty_page_and_removes_state_after_last_item`.
  - Normal lifecycle/order: `collect_full_lifecycle_single_item_pages`, `collect_first_page_preserves_non_monotonic_source_order`, `collect_second_page_preserves_non_monotonic_source_order`, `collect_third_page_preserves_non_monotonic_source_order`.
  - Duplicate/stale page: `collect_next_rejects_duplicate_first_page_response_after_cursor_advanced`, `duplicate_first_page_rejection_preserves_advanced_state`, `collect_next_rejects_stale_completion_page`, `stale_completion_page_rejection_preserves_live_state`.
  - Durable extra/recovery: `collect_pagination_extra_round_trips_for_recovery`, `collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page`, corrupt/identity mismatch rejection tests.
  - Isolation/bounds: `collect_states_independent_entries_per_run`, `collect_next_honors_value_store_arena_cap_without_advancing_cursor`, `collect_start_enforces_page_size_bounds_before_allocating_page`.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_runtime/src/engine/drive.rs` tests around lines 624-678 assert collect slot evidence extras are captured.
- `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_storage/src/tests.rs` and `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/crates/vb_storage/src/vb_2bok_durability_gate_tests.rs` cover `DuplicateEvent`, `SequenceGap`, snapshots, and replay ordering generally, not collect-specific pagination semantics.

## Existing evidence/worktrees

- Requested worktree `/home/lewis/src/Velvet-ballistics-core-engine-12` was not present under `/home/lewis/src`; no evidence could be inspected there.
- Current bead artifacts exist at `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/.beads/vb-qi37.3/baseline-report.md` and `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/.beads/vb-qi37.3/STATE.md`; State 1 notes parent has completed child evidence but parent integration remains blocked/needs revalidation.
- Adjacent worktree `/home/lewis/src/vb-qi37.3.2-ws` contains `.beads/vb-qi37.3.1/codebase-map.md`; it may be useful context for child evidence, but it is not the requested `/home/lewis/src/Velvet-ballistics-core-engine-12` evidence path.
- Current workspace also has nested `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/vb-99n6-ws/crates/vb_runtime/src/primitives/collect.rs` and `/home/lewis/src/Velvet-ballistics-vb-qi37-3-go/vb-99n6-ws/crates/vb_runtime/src/collect_tests.rs`; likely stale/nested worktree copies. Do not edit unless module graph proves they are active.

## Risks/unknowns

- Typed stale/duplicate/out-of-order page errors are ambiguous: implementation rejects by missing current page state with generic `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state missing" }`, not a dedicated typed variant.
- `RuntimeJournalEvent::SlotWritten.extra` conflates collect pagination extra and fallback taint extra. `CollectStates::hydrate_journal_events` will attempt to decode any `SlotWrittenEvent.extra` as collect state; integration must filter collect slots/events or change encoding/discriminant.
- `vb_storage::recovery::hydrate_run_frame*` reconstructs `RunFrame` but not `CollectStates`; collect state hydration is a separate helper in `vb_runtime` and needs an end-to-end integration proof with recovered frame + recovered collect side table.
- `EvidenceCollector` drops events when at capacity; if a collect `SlotWritten` extra is dropped, recovery cannot hydrate pagination. This may require a typed failure/evidence-capacity verifier or bounded-capacity proof.
- `CollectStates` uses `HashMap`; master forbids hot runtime maps/string maps, but this map is numeric-keyed. Verify accepted under current hot-path resource rules or replace with bounded numeric table if required by reviewer.
- No direct test found for out-of-order page completion as a distinct typed error; duplicate/stale tests cover older page IDs only.
- No direct integration test found proving collect continuation survives wait/ask suspension and resume through shard/runtime API.
- Requested evidence worktree path is missing; this is a State 2 blocker only if the parent requires reusing that exact closed-child evidence.

## Recommended scope JSONL

{"bead_id":"vb-qi37.3","touched_crates":["vb_runtime","vb_storage","vb_core"],"touched_files":["crates/vb_runtime/src/primitives/collect.rs","crates/vb_runtime/src/engine/drive.rs","crates/vb_runtime/src/engine/types.rs","crates/vb_runtime/src/shard/impl_.rs","crates/vb_runtime/src/journal.rs","crates/vb_runtime/src/collect_tests.rs","crates/vb_storage/src/events.rs","crates/vb_storage/src/recovery/hydrate.rs","crates/vb_storage/src/recovery/hydrate_support.rs","crates/vb_storage/src/recovery/recover.rs","crates/vb_storage/src/journal.rs","crates/vb_core/src/errors.rs","crates/vb_core/src/workflow/mod.rs"],"public_apis":["vb_runtime::primitives::collect::CollectPaginationState","vb_runtime::primitives::collect::CollectStates","vb_runtime::primitives::collect::hydrate_collect_states_from_recovered_journal","vb_runtime::primitives::collect::collect_start","vb_runtime::primitives::collect::collect_next","vb_runtime::primitives::collect::collect_finish","vb_storage::JournalEvent::SlotWrittenEvent","vb_storage::recovery::hydrate_run_frame_from_events","vb_core::errors::EngineError","vb_core::workflow::CompiledNodeKind::CollectStart","vb_core::workflow::CompiledNodeKind::CollectNext","vb_core::workflow::CompiledNodeKind::CollectFinish"],"changed_dependencies":[],"contract_clauses":["MASTER:95 no unbounded pagination","MASTER:459 max_collect_items","MASTER:635-638 collect IR nodes","MASTER:1478 collect phase","MASTER:1508-1509 recovery/runtime gaps","MASTER:2307-2356 collect semantics","MASTER:3414 per-run pagination state","MASTER:3697 collect hardening"],"risk_tags":["runtime-hot-path","durability","recovery-hydration","journal-extra-schema","typed-error-contract","bounded-state","resume-replay","stale-duplicate-ordering"],"required_verifier_modes":["cargo-nextest:vb_runtime::collect_tests","cargo-nextest:vb_runtime::engine","cargo-nextest:vb_storage::recovery","cargo-nextest:cross-crate runtime/storage integration","proptest:pagination-ordering-replay","miri:vb_core-vb_runtime pure recovery helpers","moon ci"],"release_critical":true}
