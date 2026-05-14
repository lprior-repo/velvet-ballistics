# Codebase Map: vb-qi37.3.1

Bead: `vb-qi37.3.1`  
Title: `runtime: Verify collect state isolation`  
State: State 2 MAP  
Workspace: `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25`

## Relevant crates/modules/files

- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_runtime/src/primitives/collect.rs`
  - Primary collect pagination implementation.
  - `CollectPaginationState` carries `run_id`, `collector_slot`, `source`, `current_page`, cursor/page metadata, limits, and timing.
  - `CollectStates` is a side table keyed by `(RunId, SlotIdx)`, replacing a global mutex pattern.
  - Important functions: `upsert`, `find`, `remove`, `capture_extra`, `capture_state`, `hydrate_extra`, `hydrate_journal_events`, `hydrate_collect_states_from_recovered_journal`, `collect_start`, `collect_next`, `collect_finish`.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_runtime/src/shard/types.rs`
  - `RunState` owns `collect_states: CollectStates`, so collect state should be isolated per active runtime run state.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_runtime/src/shard/lifecycle.rs`
  - `handle_submit` initializes `collect_states: CollectStates::new()` per submitted run.
  - `drive_state` passes `&mut state.collect_states` into `drive_deterministic_full`.
  - `drive_run` removes a `RunState` from the shard, drives it, then either keeps, finishes, awaits, or fails that same state.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_runtime/src/engine/drive.rs`
  - `drive_deterministic_full` accepts `&mut CollectStates` and passes it into node execution.
  - `collect_written_slot` identifies collect writes for evidence capture.
  - Evidence path calls `collect_states.capture_state(run.run_id(), slot)` before emitting collect slot write extras.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_runtime/src/engine/execute.rs`
  - Dispatch layer for `CompiledNodeKind::{CollectStart, CollectPage, CollectNext, CollectFinish}`.
  - All collect node handlers receive the caller-provided `collect_states`, not a global/static table.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_runtime/src/collect_tests.rs`
  - Main test module for collect primitive behavior, recovery extras, identity mismatch rejection, and existing isolation tests.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_runtime/src/engine/tests.rs`
  - Broader engine-level tests using explicit `CollectStates::new()` with `drive_deterministic_full`.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_runtime/src/shard/tests.rs`
  - Shard tests build `RunState` values with `CollectStates::new()`; likely location for runtime-level multi-run state isolation verification if this bead targets shard behavior rather than primitive behavior.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_storage/tests/recovery_integration.rs`
  - Existing recovery isolation tests; inspect later only if the contract expands to durable cross-run collect state recovery.

## Current patterns to reuse

- Construct fresh collect state via helper `fresh_states() -> CollectStates` in `collect_tests.rs`.
- Use `CollectScenario` helper in `collect_tests.rs` for collect start/next flows, including current-page lookup and stale/duplicate page rejection.
- Reuse existing error assertion helper `assert_invalid_workflow_reason` for identity/decode errors.
- Reuse `slot_written_extra(run, slot, extra)` for durable collect pagination extra tests.
- Existing durable-state pattern:
  1. `collect_start` creates page and inserts `CollectPaginationState`.
  2. `capture_extra(run_id, collector)` serializes only the matching `(run, collector)` entry.
  3. `hydrate_extra` validates embedded identity against journal/event identity.
  4. `collect_next` resumes only when current collector page matches the stored `current_page`.
- Existing runtime/shard pattern: state is owned by `RunState`, and `Shard::drive_state` passes `&mut state.collect_states` into `drive_deterministic_full`; no global collect storage should be introduced.
- Existing evidence pattern in `drive.rs`: collect slot writes carry one authoritative `extra: Some(_)` event for the active collector slot.

## Suspected touchpoints

- Most likely test touchpoint: `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_runtime/src/collect_tests.rs`.
  - Existing `collect_states_independent_entries_per_run` verifies two manual entries with same collector slot but different run IDs do not collide.
  - Bead likely needs stronger verification that runtime collect flows cannot read/advance another run's pagination state, especially when current page IDs or collector slots overlap.
- If runtime-level verification is required, touchpoint shifts to `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_runtime/src/shard/tests.rs` or `engine/tests.rs`.
  - Verify two runs driven through collect pagination keep separate `RunState.collect_states` even on the same shard.
- Recovery identity checks are already present in `collect_tests.rs` around lines 2100-2297.
  - Future contract should decide whether existing identity mismatch tests satisfy recovery isolation or whether an additional cross-run recovered-journal case is needed.
- `CollectStates::entries` is private but tests inside `collect_tests.rs` are in the same module context and already inspect `states.entries.is_empty()`; adding state-table assertions there should be easiest.

## Test locations to inspect later

- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_runtime/src/collect_tests.rs`
  - Around lines 785-812: missing-state rejection.
  - Around lines 2100-2297: durable extra round-trip, corrupt extra, identity mismatch.
  - Around lines 2495-2539: current `collect_states_independent_entries_per_run` baseline.
  - Around lines 2615 onward: edge cases for cursor and validation.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_runtime/src/engine/drive.rs`
  - Around lines 630-684: `collect_pagination_extra_single_authoritative_evidence_write`.
  - Engine tests in same module can verify drive-level evidence/extra isolation if primitive tests are insufficient.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_runtime/src/shard/lifecycle.rs`
  - Around lines 121-129 and 394-431: per-run state ownership and drive plumbing.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_runtime/src/shard/tests.rs`
  - Inspect if State 3 contract requires true runtime/shard integration rather than primitive-only verification.
- `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_storage/src/journal.rs` and `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/vb_storage/tests/recovery_integration.rs`
  - Inspect only for durable run-ID isolation dependencies.

## Risks/dependencies

- The current strongest isolation mechanism is `(RunId, SlotIdx)` keying plus hydrated identity validation. Tests must avoid proving only manual table independence if the bead specifically says `runtime`; State 3 should clarify primitive vs engine vs shard runtime scope.
- `ListId` values come from `ValueStore`; if test setups use independent stores, equal `ListId` values across runs may be easy to create. That is useful for adversarial tests but must be stated intentionally.
- `collect_next` additionally checks `current_page`; tests should verify that a run cannot advance with another run's state even when collector slot and/or page ID are adversarially similar.
- Durable recovery uses `JournalEvent::SlotWrittenEvent { run, slot, extra }`; tests must ensure mismatched embedded state identity fails instead of silently hydrating under the event identity.
- Runtime core rules apply: no `unwrap`, `expect`, `panic`, `todo`, `dbg`, unsafe, unchecked indexing, or ad-hoc global mutable state in future implementation/tests.
- `moon ci` is canonical later, but this State 2 did not run gates or alter production/tests.

## Next-state notes for rust-contract

- Define the contract around observable isolation:
  - Given two collect pagination states for the same collector slot under different `RunId`s, lookup/capture/remove for one run must not affect the other.
  - Given a collect flow for run A, `collect_next` must not use run B's state even if collector slot and page IDs collide.
  - Given durable collect extra whose embedded `run_id` or `collector_slot` differs from the journal/event identity, hydration must fail with `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state identity mismatch" }`.
  - Given runtime shard execution of two runs, each `RunState` must own independent `CollectStates` and retain it across budgeted drive resumes.
- Recommended red tests for State 5:
  - Strengthen primitive table isolation beyond existing `collect_states_independent_entries_per_run` by proving `remove(run_a, slot)` leaves run B intact and `capture_extra(run_a, slot)` cannot capture run B.
  - Add adversarial collect flow where two `RunFrame`s use the same `SlotIdx` and matching `ListId` values from independent stores; assert run A cannot find/run B state because `RunId` participates in the key and hydrated identity.
  - If runtime scope is required, add a shard/engine integration test with two active runs that both enter collect pagination and resume independently.

STATUS: COMPLETE
