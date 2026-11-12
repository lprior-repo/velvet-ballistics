# Test Plan: vb-qi37.3 runtime collect pagination durability/hydration

## Startup Doctrine and Repair Inputs
- Read `/home/lewis/.claude/skills/test-planner/SKILL.md` and `/home/lewis/.agents/skills/test-planner/SKILL.md`; no conflict observed, agents copy would win. Doctrine applied: planning artifact only, behavior-first tests, BDD names, exact values/errors, proptest/fuzz/mutation/Kani planning, and no production/test implementation.
- Read rejected review: `.beads/vb-qi37.3/test-plan-review.md` (`STATUS: REJECTED`). This repair addresses every required repair listed at lines 86-93.
- Re-read upstream artifacts as required: `codebase-map.md`, `delivery-scope.jsonl`, `contract.md`, `tla-spec.md`, `lean-contract.md`, `verification-layers.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, and `contract-verification-review.md` (`STATUS: APPROVED`).
- This file is the only artifact repaired. It prescribes tests; it does not write production code or test code.

## Repair Decision: Exact Error Taxonomy for State 5 Red Tests
State 5 must write red tests against this target public API. These variants/kinds may not exist yet; that is intentional red-phase contract. Tests must not accept `EngineError::InvalidCompiledWorkflow { reason: "collect pagination state missing" }` for ERR-004..ERR-008.

```rust
EngineError::CollectPageOrderViolation {
    kind: CollectPageOrderViolationKind,
    run_id: RunId,
    collector_slot: SlotIdx,
    expected_page: ListId,
    observed_page: ListId,
}

enum CollectPageOrderViolationKind {
    Duplicate,
    Stale,
    OutOfOrder,
}

EngineError::CollectExtraHydrationFailed {
    kind: CollectExtraHydrationFailureKind,
    run_id: RunId,
    collector_slot: SlotIdx,
    event_seq: Option<EventSeq>,
}

enum CollectExtraHydrationFailureKind {
    EmptyExtra,
    DecodeFailed,
    RunMismatch { expected: RunId, actual: RunId },
    SlotMismatch { expected: SlotIdx, actual: SlotIdx },
    CurrentPageMismatch { expected: ListId, actual: ListId },
    NonCollectExtra,
}

EngineError::CollectEvidenceCapacityExceeded {
    run_id: RunId,
    slot: SlotIdx,
    capacity: usize,
    len: usize,
    required: &'static str, // exactly "collect SlotWritten extra"
}
```

Clause mapping:
- ERR-004 duplicate page -> `EngineError::CollectPageOrderViolation { kind: Duplicate, run_id, collector_slot, expected_page: live.current_page, observed_page: duplicate_page }`.
- ERR-005 stale page -> `EngineError::CollectPageOrderViolation { kind: Stale, run_id, collector_slot, expected_page: live.current_page, observed_page: stale_page }`.
- ERR-006 future/out-of-order page -> `EngineError::CollectPageOrderViolation { kind: OutOfOrder, run_id, collector_slot, expected_page: live.current_page, observed_page: future_page }`.
- ERR-007 collect-extra failures -> `EngineError::CollectExtraHydrationFailed { kind: ..., run_id, collector_slot, event_seq }` with exact kind listed per scenario below.
- ERR-008 evidence capacity -> `EngineError::CollectEvidenceCapacityExceeded { run_id, slot: collector_slot, capacity, len, required: "collect SlotWritten extra" }`.

All State 5 tests must snapshot relevant `CollectPaginationState` before the operation and assert exact equality after any rejected operation. No test may assert only class-of-error success/failure.

## Summary
- Behaviors identified: 18.
- State 5 must-write tests: 31 exact tests listed in Section 3 and Section 8. No State 5 must-write item is unresolved.
- Trophy allocation: integration widest because runtime/storage/journal/recovery boundaries are the primary risk; unit/calc for bounds and classifiers; formal lanes remain waiver-backed where no proof harness exists.
- Proptest/fuzz: concrete State 5 harness plans are specified with file targets, bounds, seeds, and commands.
- Kani/Verus/TLA+/cargo-mutants: Kani/Verus/TLA are formal-verifier-owned waivers for this bead phase; mutation has a named killer map plus approved `MUT-COLLECT-001` waiver if no collect-scoped cargo-mutants lane is wired.
- Mutation threshold: >=90% for any collect-scoped mutation lane; otherwise `MUT-COLLECT-001` waiver must cite the named killer tests in this plan.

## Existing Tests Identified

### `crates/vb_runtime/src/collect_tests.rs`
- Lifecycle/empty/final/repeated: `collect_start_jumps_to_done_when_source_empty`, `collect_start_zero_items_with_nonzero_limit_goes_to_done`, `collect_start_exact_page_limit_finishes_without_active_pagination_state`, `collect_next_writes_empty_page_and_removes_state_after_last_item`, `collect_next_returns_done_when_remaining_empty`, `collect_next_cursor_at_item_count_goes_to_done`, `collect_full_lifecycle_single_item_pages`, `collect_repeated_start_next_cycles`, `collect_finish_materializes_output`, `collect_finish_removes_state_after_writing_output`, `collect_finish_propagates_list_to_output`, `collect_finish_propagates_taint`.
- Bounds/time/resource: `collect_start_returns_error_when_source_is_not_list`, `collect_start_null_source_returns_type_mismatch`, `collect_start_returns_error_when_limit_exceeded`, `collect_start_items_exceeding_limit_by_one`, `collect_start_enforces_fanout_item_limit_at_exact_boundary`, `collect_start_rejects_fanout_one_over_limit_without_collector_state`, `collect_start_returns_error_when_page_size_zero`, `collect_start_page_size_zero_returns_error_even_for_empty_list`, `collect_start_enforces_page_size_bounds_before_allocating_page`, `collect_start_rejects_page_size_above_limit`, `collect_start_page_size_u32_max_returns_error`, `collect_start_page_size_at_limit_boundary`, `collect_start_page_size_exactly_one_over_limit`, `collect_start_limit_boundary_exact_vs_one_over`, `collect_start_with_time_limit_stores_limit_in_state`, `collect_start_without_time_limit_stores_none`, `check_time_limit_returns_error_when_exceeded`, `check_time_limit_ok_when_not_exceeded`, `check_time_limit_ok_when_no_limit_set`, `collect_next_time_limit_exceeded_returns_error`, `collect_next_honors_value_store_arena_cap_without_advancing_cursor`.
- Pagination/order/state/recovery: `collect_states_independent_entries_per_run`, `collect_states_find_returns_none_for_wrong_slot`, `collect_states_find_returns_none_for_wrong_run_id`, `collect_states_find_returns_none_for_wrong_page`, `collect_states_upsert_and_find_roundtrip`, `collect_states_remove_clears_entry`, `collect_states_remove_nonexistent_is_noop`, `collect_states_upsert_replaces_existing`, `collect_next_validates_state_consistency`, `collect_first_page_preserves_non_monotonic_source_order`, `collect_second_page_preserves_non_monotonic_source_order`, `collect_third_page_preserves_non_monotonic_source_order`, `collect_next_rejects_duplicate_first_page_response_after_cursor_advanced`, `duplicate_first_page_rejection_preserves_advanced_state`, `collect_next_rejects_stale_completion_page`, `stale_completion_page_rejection_preserves_live_state`, `collect_pagination_extra_round_trips_for_recovery`, `collect_pagination_extra_rejects_corrupt_bytes`, `collect_journal_extra_rejects_corrupt_bytes`, `collect_pagination_extra_recovered_journal_rejects_corrupt_bytes`, `collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page`, `collect_pagination_extra_rejects_identity_mismatch`, `collect_journal_extra_rejects_identity_mismatch`, `collect_pagination_extra_recovered_journal_rejects_identity_mismatch`.

### `crates/vb_runtime/src/engine/drive.rs`
- `collect_pagination_extra_single_authoritative_evidence_write` asserts exactly one `EvidenceEvent::SlotWritten { extra: Some(_) }` for collect.

## 1. Behavior Inventory
1. Collect start rejects non-list source with `EngineError::TypeMismatch { expected: "list", found }` and no collect state.
2. Collect start rejects page size 0 with `EngineError::InvalidCompiledWorkflow { reason: "collect page_size must be nonzero" }` and no collect state.
3. Collect start rejects `page_size > limit` with `EngineError::CollectPageLimitExceeded` and no collect state.
4. Collect start accepts min page size 1 and max page size exactly `limit` when item count is bounded.
5. Collect start rejects item count over collect limit/resource max with `EngineError::CollectItemLimitExceeded` and no collect state.
6. Collect start writes bounded first page and exactly one durable continuation state when more items remain.
7. Empty/final/repeated collect pages route to done and remove continuation state.
8. Collect next advances cursor monotonically and preserves source order for valid current page.
9. Duplicate page completion returns `CollectPageOrderViolation { kind: Duplicate, ... }` and preserves state.
10. Stale page completion returns `CollectPageOrderViolation { kind: Stale, ... }` and preserves state.
11. Future/out-of-order page completion returns `CollectPageOrderViolation { kind: OutOfOrder, ... }` and preserves state.
12. Collect finish materializes output, preserves taint, and removes state.
13. Collect state is isolated by exact `(RunId, SlotIdx)` across runs and nodes.
14. Collect continuation survives wait/ask suspension/resume and preserves source identity/item count.
15. Journaled collect `SlotWrittenEvent.extra` round-trips all continuation fields before recovery/resume.
16. Recovery hydrates `RunFrame` and `CollectStates` from the same event prefix and resumes without skip/repeat.
17. Empty/corrupt/mismatched/non-collect extras fail closed or skip according to exact schema behavior; no unrelated state is poisoned.
18. Evidence capacity preserves required collect extra or fails with `CollectEvidenceCapacityExceeded`; never silent success with missing collect extra.

## 2. Trophy Allocation
| Behavior | Primary layer | Required evidence |
|---|---|---|
| 1-8, 12 | Unit/calc + primitive integration | `crates/vb_runtime/src/collect_tests.rs` exact tests and proptest harness `collect_pagination_prop.rs`. |
| 9-11 | Unit/API integration | Exact page-order red tests asserting `CollectPageOrderViolation` variants/kinds/fields. |
| 13 | Unit + engine integration | `CollectStates` tests plus `drive_deterministic_full` two-run/two-slot workflow tests. |
| 14 | Shard integration | `Shard::new_with_journal`, `ShardCommand::{SubmitWithInputs, TimerFired, AskAnswered}`, `snapshot_run`, volatile journal inspection. |
| 15-17 | Runtime/storage integration + fuzz | `JournalEvent::SlotWrittenEvent`, `hydrate_run_frame_from_events`, `hydrate_collect_states_from_recovered_journal`, codec fuzz. |
| 18 | Engine integration + static/formal | `EvidenceCollector::with_capacity`, `drive_deterministic_full`, exact capacity error/preserve tests; `STATIC-COLLECT-001`. |
| All | E2E gate | `env VERIFY_BEAD_ID=vb-qi37.3 ALLOW_BEAD_LOCKBUD_WAIVER=1 bash scripts/rust-verification-gauntlet.sh all`. |

## 3. State 5 Must-Write / Repair Tests

### A. Boundary tests in `crates/vb_runtime/src/collect_tests.rs`

1. `collect_start_page_size_one_writes_single_item_page_and_state_cursor_one`
- Public entry point: `collect_start`.
- Given: `RunFrame` with source slot 0 list `[I64(10), I64(20)]`, collector slot 1, `limit=2`, `page_size=1`, `body=StepIdx(1)`, `done=StepIdx(2)`, empty `CollectStates`.
- Then: signal equals jump/continue to body, collector list equals `[I64(10)]`, `CollectStates::capture_state(run_id, SlotIdx(1))` equals `Some` with `cursor=1`, `page_size=1`, `item_count=2`, `limit=2`, `source=<source_list>`, `current_page=<collector_page>`, and no other key exists.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_start_page_size_one_writes_single_item_page_and_state_cursor_one`.

2. `collect_start_page_size_zero_returns_invalid_workflow_and_no_state`
- Public entry point: `collect_start`.
- Given: nonempty source, `limit=2`, `page_size=0`.
- Then: exact `Err(EngineError::InvalidCompiledWorkflow { reason: "collect page_size must be nonzero" })`; collector slot remains unwritten or exactly previous sentinel value; `capture_state(run_id, collector)=None`.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_start_page_size_zero_returns_invalid_workflow_and_no_state`.

3. `collect_start_page_size_at_limit_accepts_exact_limit_and_finishes_without_state`
- Public entry point: `collect_start`.
- Given: source length 3, `limit=3`, `page_size=3`.
- Then: routes done, collector list equals all 3 items in order, `capture_state(run_id, collector)=None`.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_start_page_size_at_limit_accepts_exact_limit_and_finishes_without_state`.

4. `collect_start_page_size_one_over_limit_returns_page_limit_and_no_state`
- Public entry point: `collect_start`.
- Given: source length 2, `limit=2`, `page_size=3`.
- Then: exact `Err(EngineError::CollectPageLimitExceeded)`, collector slot sentinel unchanged, `capture_state(run_id, collector)=None`.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_start_page_size_one_over_limit_returns_page_limit_and_no_state`.

5. `collect_start_u32_max_page_size_returns_page_limit_and_no_state`
- Public entry point: `collect_start`.
- Given: source length 1, `limit=1`, `page_size=u32::MAX`.
- Then: exact `Err(EngineError::CollectPageLimitExceeded)`, collector sentinel unchanged, `capture_state(run_id, collector)=None`.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_start_u32_max_page_size_returns_page_limit_and_no_state`.

6. `collect_start_empty_list_writes_empty_done_and_no_state`
- Public entry point: `collect_start`.
- Given: empty source list, `limit=3`, `page_size=1`.
- Then: routes done, collector list equals `[]`, `capture_state(run_id, collector)=None`.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_start_empty_list_writes_empty_done_and_no_state`.

7. `collect_start_exact_max_items_accepts_and_keeps_state_only_when_more_pages_remain`
- Public entry point: `collect_start`.
- Given: source length equals test resource max/limit 4, `page_size=2`.
- Then: first page equals first two values, state cursor=2, item_count=4, limit=4.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_start_exact_max_items_accepts_and_keeps_state_only_when_more_pages_remain`.

8. `collect_start_one_over_max_items_returns_item_limit_and_no_state`
- Public entry point: `collect_start`.
- Given: source length 5, `limit=4`, `page_size=2`.
- Then: exact `Err(EngineError::CollectItemLimitExceeded)`, collector sentinel unchanged, `capture_state(run_id, collector)=None`.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_start_one_over_max_items_returns_item_limit_and_no_state`.

### B. Page-order taxonomy tests in `crates/vb_runtime/src/collect_tests.rs`

9. `collect_next_duplicate_page_returns_order_violation_duplicate_and_preserves_state`
- Public entry point: `collect_next`.
- Given: source `[1,2,3,4]`, limit 4, page_size 2. Run `collect_start`, snapshot page1 id. Run `collect_next`, snapshot live state expecting page2 with cursor 4. Write old page1 id back into collector slot.
- Then: exact `Err(EngineError::CollectPageOrderViolation { kind: CollectPageOrderViolationKind::Duplicate, run_id, collector_slot, expected_page: page2, observed_page: page1 })`; `capture_state(run_id, collector)` equals pre-error state; collector slot remains `SlotValue::List(page1)` so the invalid observation is visible but durable state did not advance.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_next_duplicate_page_returns_order_violation_duplicate_and_preserves_state`.

10. `collect_next_stale_page_returns_order_violation_stale_and_preserves_state`
- Public entry point: `collect_next`.
- Given: source `[7,8,9]`, limit 3, page_size 1. Advance twice so live state expects page3/cursor3; write page1 id into collector.
- Then: exact `Err(EngineError::CollectPageOrderViolation { kind: CollectPageOrderViolationKind::Stale, run_id, collector_slot, expected_page: page3, observed_page: page1 })`; state equals pre-error state.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_next_stale_page_returns_order_violation_stale_and_preserves_state`.

11. `collect_next_future_page_returns_order_violation_out_of_order_and_preserves_state`
- Public entry point: `collect_next`.
- Given: source `[1,2,3]`, limit 3, page_size 1; live state expects page1 with cursor1. Insert unrelated future page list `[I64(99)]` into `ValueStore`, write it to collector slot.
- Then: exact `Err(EngineError::CollectPageOrderViolation { kind: CollectPageOrderViolationKind::OutOfOrder, run_id, collector_slot, expected_page: page1, observed_page: future_page })`; state equals pre-error state; source list remains `[1,2,3]`.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_next_future_page_returns_order_violation_out_of_order_and_preserves_state`.

### C. Isolation tests

12. `drive_collect_states_are_isolated_for_two_runs_same_collector_slot`
- File/module target: `crates/vb_runtime/src/engine/drive.rs` tests module.
- Public entry point: `drive_deterministic_full` using two independent `RunFrame`s, two `ValueStore`s, two `CollectStates`, same `SlotIdx(1)` collector, same `CompiledWorkflow` with `CollectStart -> CollectNext -> CollectFinish`.
- Given: run A source `[1,2,3,4]`, run B source `[10,20,30,40]`, both page_size 2.
- Then: after one drive step each, A state has `run_id=A`, source/page ids from A store, cursor=2; B state has `run_id=B`, source/page ids from B store, cursor=2; querying A table with B run id returns `None` and B table with A run id returns `None`.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime drive_collect_states_are_isolated_for_two_runs_same_collector_slot`.

13. `drive_collect_states_are_isolated_for_one_run_two_collector_slots`
- File/module target: `crates/vb_runtime/src/engine/drive.rs` tests module.
- Public entry point: `drive_deterministic_full` with workflow containing two independent collect chains using collector slots 1 and 2 and sources 0 and 3.
- Given: same `RunId`, two source lists `[1,2,3]` and `[9,8,7]`, page_size 1 for each.
- Then: after each start executes, `capture_state(run_id, SlotIdx(1))` has source0/cursor1 and `capture_state(run_id, SlotIdx(2))` has source3/cursor1; removing/finishing slot1 does not remove slot2.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime drive_collect_states_are_isolated_for_one_run_two_collector_slots`.

### D. Wait/ask continuation tests in `crates/vb_runtime/src/shard/tests.rs`

14. `collect_state_survives_wait_timer_resume_and_next_page_uses_same_source`
- Public integration surface: `Shard::new_with_journal`, `ShardCommand::SubmitWithInputs`, `ShardCommand::TimerFired`, `Shard::tick`, `Shard::snapshot_run`, `VolatileRuntimeJournal::snapshot`.
- Workflow shape: `CollectStart(source=slot0, output=slot1, body=step1, done=step4, limit=4, page_size=2) -> WaitUntil(deadline_slot=slot2) -> CollectNext(collector_slot=slot1, body=step1, done=step3) -> CollectFinish(collector_slot=slot1, output=slot3) -> Finish(result=slot3)`. Initial inputs: slot0 list `[1,2,3,4]`, slot2 deadline already eligible according to existing timer test convention.
- Given: submit run `RunId(3701)` with volatile journal. First `tick` drives to `RuntimeSignal::AwaitingWait` and leaves one pending timer. Capture journal events and the shard-owned collect state via test-module access to `shard.runs.get(&run).collect_states.capture_state(run, SlotIdx(1))`.
- When: enqueue `TimerFired { run }`, tick until run either awaits next wait or completes next page.
- Then: state before timer has `source=<slot0 list id>`, `cursor=2`, `item_count=4`, `limit=4`, `page_size=2`. After timer/resume, next emitted collector page equals `[I64(3), I64(4)]`; source id and item_count in any remaining/final state are unchanged; journal contains `RuntimeJournalEvent::WaitScheduled { run, step: StepIdx(1) }`, `RuntimeJournalEvent::WaitResolved { run, step: StepIdx(1) }`, and collect `SlotWritten` extras for both page writes.
- Cleanup: volatile journal is in-memory and dropped at test end; no filesystem side effects.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_state_survives_wait_timer_resume_and_next_page_uses_same_source`.

15. `collect_state_survives_ask_answer_resume_and_next_page_uses_same_source`
- Public integration surface: `Shard::new_with_journal`, `ShardCommand::SubmitWithInputs`, `ShardCommand::AskAnswered`, `AskTicket`, `AskAnswer`, `Shard::tick`, `VolatileRuntimeJournal::snapshot`.
- Workflow shape: `CollectStart(source=slot0, output=slot1, body=step1, done=step5, limit=4, page_size=2) -> Ask(prompt=slot4, timeout_slot=None) -> AskResume(answer=slot5) -> CollectNext(collector_slot=slot1, body=step1, done=step4) -> CollectFinish(output=slot3) -> Finish(result=slot3)`. Inputs: slot0 list `[1,2,3,4]`, slot4 prompt symbol/bool value.
- Given: first tick reaches ask; snapshot collect state `source`, `cursor=2`, `item_count=4`, `current_page=page1`.
- When: enqueue `ShardCommand::AskAnswered { answer: AskAnswer { ticket: AskTicket { run, ask_step: StepIdx(1), resume_step: StepIdx(2) }, answer_slot: SlotIdx(5), value: SlotValue::Bool(true), taint: Taint::Clean } }` and tick.
- Then: next collect page equals `[I64(3), I64(4)]`; ask answer slot equals `Bool(true)`; collect state source/item_count unchanged until final removal; journal contains `AskScheduled`, `AskAnswered`, and collect extra-bearing `SlotWritten` events.
- Cleanup: volatile journal drop only.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_state_survives_ask_answer_resume_and_next_page_uses_same_source`.

### E. Cross-crate recovery/hydration tests

16. `recovered_frame_and_collect_state_from_same_prefix_resume_without_skip_or_repeat`
- File/module target: add integration test under `crates/vb_runtime/tests/collect_recovery_integration.rs` or runtime crate test module with `vb_storage` dev-dependency already in scope.
- Public entry points: `vb_storage::recovery::hydrate_run_frame_from_events`, `vb_runtime::primitives::collect::hydrate_collect_states_from_recovered_journal`, `collect_next`.
- Journal prefix construction: build ordered `Vec<JournalEvent>` for `RunId(3801)` with `StepStarted(step0)`, `SlotWrittenEvent(seq2, slot0, value=postcard(SlotValue::List(source_id)), extra=None)`, `SlotWrittenEvent(seq3, slot1, value=postcard(SlotValue::List(page1)), extra=postcard(CollectPaginationState { run_id, collector_slot: SlotIdx(1), source, current_page: page1, cursor: 2, page_size: 2, item_count: 4, limit: 4, time_limit_ms: None, start_millis: 10 }))`, `StepSucceeded(step0, output=slot1)`. Build matching `ValueStore` with `source=[1,2,3,4]`, `page1=[1,2]`.
- Given: same prefix supplied to both hydration functions.
- When: run `collect_next(&mut recovered_frame, &mut store, &mut recovered_states, SlotIdx(1), StepIdx(1), StepIdx(2))`.
- Then: collector page equals `[I64(3), I64(4)]`, no item from `[1,2]` repeats, no item beyond `[3,4]` appears, hydrated state before next equals exact fields above, post-next state is removed or cursor=4 according to final-page semantics, and frame pc/signal equals expected done/body transition.
- Side effects/cleanup: no filesystem; all journal events in memory.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime recovered_frame_and_collect_state_from_same_prefix_resume_without_skip_or_repeat`.

17. `recovered_collect_state_rejects_run_mismatch_and_inserts_no_state`
- Entry point: `hydrate_collect_states_from_recovered_journal`.
- Given: `JournalEvent::SlotWrittenEvent { run: RunId(3801), seq: EventSeq(3), slot: SlotIdx(1), extra: postcard(CollectPaginationState { run_id: RunId(9999), collector_slot: SlotIdx(1), ... }) }`.
- Then: exact `Err(EngineError::CollectExtraHydrationFailed { kind: RunMismatch { expected: RunId(3801), actual: RunId(9999) }, run_id: RunId(3801), collector_slot: SlotIdx(1), event_seq: Some(EventSeq(3)) })`; returned/temporary states contain no entry for either run.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime recovered_collect_state_rejects_run_mismatch_and_inserts_no_state`.

18. `recovered_collect_state_rejects_slot_mismatch_and_inserts_no_state`
- Entry point: `hydrate_collect_states_from_recovered_journal`.
- Given: event slot `SlotIdx(1)` but encoded state `collector_slot=SlotIdx(2)`.
- Then: exact `Err(EngineError::CollectExtraHydrationFailed { kind: SlotMismatch { expected: SlotIdx(1), actual: SlotIdx(2) }, run_id, collector_slot: SlotIdx(1), event_seq: Some(seq) })`; no state inserted.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime recovered_collect_state_rejects_slot_mismatch_and_inserts_no_state`.

19. `recovered_frame_hydration_rejects_mixed_run_prefix_before_collect_hydration`
- Entry points: `hydrate_run_frame_from_events`, `hydrate_collect_states_from_recovered_journal`.
- Given: prefix with at least one event for `RunId(3801)` and one event for `RunId(3802)`.
- Then: `hydrate_run_frame_from_events(&events, RunId(3801))` returns `Err(RecoveryError::ReplayDivergence { step: StepIdx::ZERO, detail })` and the exact public detail predicate is `detail.contains("multiple runs")`; collect hydration must not be called in the test after frame hydration fails; no collect state object is produced.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime recovered_frame_hydration_rejects_mixed_run_prefix_before_collect_hydration`.

20. `recovered_collect_state_empty_event_list_returns_empty_states`
- Entry point: `hydrate_collect_states_from_recovered_journal`.
- Given: `events=[]`.
- Then: exact `Ok(CollectStates::new())`; `capture_state(any_run, any_slot)=None`.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime recovered_collect_state_empty_event_list_returns_empty_states`.

21. `recovered_frame_empty_event_list_returns_no_recovery_data`
- Entry point: `hydrate_run_frame_from_events`.
- Given: `events=[]`, run `RunId(3801)`.
- Then: exact `Err(RecoveryError::NoRecoveryData { run: RunId(3801) })`.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime recovered_frame_empty_event_list_returns_no_recovery_data`.

22. `collect_recovery_prefix_with_duplicate_sequence_fails_before_resume`
- Entry points: use `vb_storage::journal::FjallJournal::append_strict` when testing durable storage path; if in-memory event recovery is used, call the storage validator path that reports duplicate `(run, seq)` per existing storage tests.
- Given: temp directory journal, append first collect `SlotWrittenEvent` with `(run, seq=3)`, append duplicate event with same `(run, seq=3)`.
- Then: exact `Err(JournalError::DuplicateEvent)` from append; no call to collect hydration; temp directory removed by test tempdir drop.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_storage collect_recovery_prefix_with_duplicate_sequence_fails_before_resume`.

23. `collect_recovery_prefix_with_sequence_gap_fails_before_resume`
- Entry point: `FjallJournal::events_for_run` or replay validator used by existing `SequenceGap` tests.
- Given: temp journal events seq 1 and seq 3 for the same run, missing seq 2, including collect slot write at seq 3.
- Then: exact `Err(JournalError::SequenceGap { expected: EventSeq::new(2), actual: EventSeq::new(3) })`; collect hydration is not called; tempdir cleanup by drop.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_storage collect_recovery_prefix_with_sequence_gap_fails_before_resume`.

### F. Extra schema tests in `crates/vb_runtime/src/collect_tests.rs`

24. `collect_hydration_empty_extra_returns_empty_extra_error_and_no_state`
- Entry point: `CollectStates::hydrate_extra` and/or journal wrapper.
- Given: `extra=[]`, run `RunId(3901)`, slot `SlotIdx(1)`.
- Then: exact `Err(EngineError::CollectExtraHydrationFailed { kind: EmptyExtra, run_id: RunId(3901), collector_slot: SlotIdx(1), event_seq: None })`; `capture_state(run, slot)=None`.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_hydration_empty_extra_returns_empty_extra_error_and_no_state`.

25. `collect_hydration_corrupt_extra_returns_decode_failed_and_no_state`
- Entry point: `CollectStates::hydrate_extra`.
- Given: bytes `[0xFF,0x00,0x13]`.
- Then: exact `Err(EngineError::CollectExtraHydrationFailed { kind: DecodeFailed, run_id, collector_slot, event_seq: None })`; no state inserted.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_hydration_corrupt_extra_returns_decode_failed_and_no_state`.

26. `collect_hydration_current_page_mismatch_returns_page_mismatch_and_no_state`
- Entry point: journal hydration with event value page id and extra state current page id compared.
- Given: event `value=SlotValue::List(page1)`, encoded state `current_page=page2`, same run/slot.
- Then: exact `Err(EngineError::CollectExtraHydrationFailed { kind: CurrentPageMismatch { expected: page1, actual: page2 }, run_id, collector_slot, event_seq: Some(seq) })`; no state inserted.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_hydration_current_page_mismatch_returns_page_mismatch_and_no_state`.

27. `collect_hydration_non_collect_taint_extra_is_skipped_without_decode_or_state`
- Entry point: `hydrate_collect_states_from_recovered_journal`.
- Given: `JournalEvent::SlotWrittenEvent { run, seq, slot: non_collect_slot, value=postcard(SlotValue::I64(7)), extra=Some(postcard(Taint::Tainted)) }` or the repository's taint fallback byte shape.
- Then: exact `Ok(states)` with `states.capture_state(run, non_collect_slot)=None`; non-collect taint bytes are skipped and are never decoded as collect state.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_hydration_non_collect_taint_extra_is_skipped_without_decode_or_state`.

28. `collect_hydration_non_collect_extra_does_not_block_later_valid_collect_extra`
- Entry point: `hydrate_collect_states_from_recovered_journal`.
- Given: ordered events: first non-collect taint extra for slot 0, second valid collect extra for slot 1.
- Then: exact `Ok(states)`, no state for slot0, exact valid `CollectPaginationState` for slot1.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_hydration_non_collect_extra_does_not_block_later_valid_collect_extra`.

### G. Evidence capacity tests in `crates/vb_runtime/src/engine/drive.rs`

29. `collect_slot_extra_capacity_zero_returns_capacity_error_before_success`
- Public entry point: `drive_deterministic_full` with `EvidenceCollector::with_capacity(0)`.
- Given: workflow `CollectStart` writes first page and active continuation; evidence capacity 0.
- Then: exact `Err(RuntimeEngineError::Core(EngineError::CollectEvidenceCapacityExceeded { run_id, slot: collector, capacity: 0, len: 0, required: "collect SlotWritten extra" }))`; run must not be reported resumable success; `evidence.dropped()` may be 0 or unchanged because operation fails before silent drop.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_slot_extra_capacity_zero_returns_capacity_error_before_success`.

30. `collect_slot_extra_capacity_one_preserves_required_slot_written_extra`
- Entry point: `drive_deterministic_full` with `EvidenceCollector::with_capacity(1)`.
- Given: same collect workflow, capacity 1.
- Then: `evidence.drain()` contains exactly one `EvidenceEvent::SlotWritten { slot: collector, value: SlotValue::List(page1), extra: Some(expected_state), .. }`; no successful result is allowed without that event. This test chooses the preservation/reservation contract for capacity 1.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_slot_extra_capacity_one_preserves_required_slot_written_extra`.

31. `collect_slot_extra_capacity_full_returns_capacity_error_not_silent_drop`
- Entry point: direct public `EvidenceCollector::push_slot_written_with_extra` on `EvidenceCollector::with_capacity(2)` prefilled with two non-required events.
- Given: capacity full before required collect write; prefill with `push_step_started(StepIdx::ZERO)` and `push_step_succeeded(StepIdx::ZERO, None)`, then attempt `push_slot_written_with_extra(collector, SlotValue::List(page1), Taint::Clean, Some(expected_state))`.
- Then: target red-phase API returns exact `Err(EngineError::CollectEvidenceCapacityExceeded { run_id, slot: collector, capacity: 2, len: 2, required: "collect SlotWritten extra" })`; `evidence.drain()` contains exactly the two prefilled non-required events and contains no successful collect `SlotWritten` without `extra`; no silent `dropped()>0` success is allowed.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_slot_extra_capacity_full_returns_capacity_error_not_silent_drop`.

## 4. Concrete Proptest Harness Plan (State 5 executable)

File target: `crates/vb_runtime/src/collect_property_tests.rs` included from `primitives/collect.rs` test cfg or as a runtime crate test module. State 5 must add the harness unless the formal verifier explicitly re-approves `PROP-COLLECT-001` as not State-5 work.

Global settings: `ProptestConfig { cases: 256, max_shrink_iters: 1024, failure_persistence: Some(Box::new(FileFailurePersistence::SourceParallel("proptest-regressions"))), .. }`. Strategies are bounded to keep tests deterministic and fast.

1. `proptest_collect_pages_concatenate_to_original_source`
- Strategy: `Vec<i64>` length 0..=8, values -10..=10; `page_size` 1..=8; `limit=item_count.max(page_size)` capped at 8.
- Assertion: if item_count <= limit, concatenating emitted pages from `collect_start`/`collect_next*` equals original values exactly; final state is absent.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime proptest_collect_pages_concatenate_to_original_source`.

2. `proptest_collect_bounds_preserved_across_valid_transitions`
- Strategy: source len 1..=8, page_size 1..=len, limit len..=8.
- Assertion after every transition: `cursor <= item_count`, `item_count <= limit`, `page_size <= limit`, cursor monotonically increases by emitted page length, never by more than page_size.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime proptest_collect_bounds_preserved_across_valid_transitions`.

3. `proptest_collect_rejected_pages_preserve_state_and_exact_kind`
- Strategy: source len 3..=6, page_size 1..=2, invalid observation enum `{Duplicate, Stale, Future}`.
- Assertions: exact `CollectPageOrderViolationKind` for generated invalid observation; pre/post `CollectPaginationState` equal.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime proptest_collect_rejected_pages_preserve_state_and_exact_kind`.

4. `proptest_collect_extra_roundtrip_or_exact_reject`
- Strategy: valid `CollectPaginationState` fields in small bounds plus mismatch enum `{Run, Slot, CurrentPage}`.
- Assertions: valid encode/decode preserves every field; mismatches return exact `CollectExtraHydrationFailed` kind/fields and insert no state.
- Command: `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime proptest_collect_extra_roundtrip_or_exact_reject`.

Approved fallback if State 5 scope excludes proptest: cite `PROP-COLLECT-001` waiver from `proof-obligations.jsonl`, with `TLA-COLLECT-001`, `VERUS-COLLECT-001`, `VERUS-COLLECT-003`, and exact tests above as compensating evidence. This fallback is not a State 5 unresolved placeholder; it is formal-verifier-owned.

## 5. Fuzz Scope (formal-verifier-owned for this phase)

Fuzz is not State 5 must-write for this bead phase. Ownership is the approved formal obligation `FUZZ-CODEC-001` with compensating obligations `VERUS-COLLECT-004`, `TLA-COLLECT-005`, and `GATE-COLLECT-ALL`. State 5 must not invent or choose a fuzz path; the formal verifier records the waiver/evidence.

1. `fuzz_collect_extra_hydration_bytes`
- Input: arbitrary bytes plus fixed context `(RunId(4001), SlotIdx(1), EventSeq(1))`.
- Seeds: empty bytes; valid encoded state; truncated valid state; `[0xFF]`; postcard taint bytes; valid state wrong run; wrong slot; wrong current page.
- Assertion: never panic/OOM; output is exact `Ok` with state matching context and value page for valid collect extras, exact `CollectExtraHydrationFailed { kind: EmptyExtra|DecodeFailed|RunMismatch|SlotMismatch|CurrentPageMismatch, ... }` for invalid collect extras, or exact `Ok` with no inserted state for non-collect tagged extras.
- Command ownership: formal-verifier-owned `FUZZ-CODEC-001`; no State 5 command required.

2. `fuzz_collect_recovered_journal_event_sequence`
- Input: small generated event sequence length 0..=6, event kind enum `{SlotWrittenNoExtra, SlotWrittenCollectExtra, SlotWrittenNonCollectExtra, StepStarted, StepSucceeded}`, seq values 1..=8.
- Seeds: empty event list, same-run ordered valid prefix, mixed-run prefix, duplicate seq, sequence gap, non-collect extra before valid collect extra.
- Assertion: no panic; exact `NoRecoveryData` for empty frame hydration; exact recovery/storage errors for mixed/gap/duplicate when using storage path; collect states only inserted for valid collect extras.
- Command ownership: formal-verifier-owned `FUZZ-CODEC-001`; no State 5 command required.

Formal-verifier evidence must cite `FUZZ-CODEC-001`, `VERUS-COLLECT-004`, `TLA-COLLECT-005`, and `GATE-COLLECT-ALL`.

## 6. Formal / Kani / Verus / TLA Scope

These are not State 5 must-write test placeholders. They are formal-verifier-owned or waiver-backed by approved obligations:
- TLA model missing: `TLA-COLLECT-001`, `TLA-COLLECT-002`, `TLA-COLLECT-003`, `TLA-COLLECT-004`, `TLA-COLLECT-005` and waiver `TLA-WAIVER-COLLECT-001`.
- Verus proofs missing: `VERUS-COLLECT-001`, `VERUS-COLLECT-002`, `VERUS-COLLECT-003`, `VERUS-COLLECT-004` and waiver `VERUS-WAIVER-COLLECT-001`.
- Miri/deep lane: `MIRI-COLLECT-001`, command `bash scripts/rust-verification-gauntlet.sh deep`.
- Evidence capacity static/formal lane: `STATIC-COLLECT-001`; State 5 still writes tests 29-31, formal verifier records waiver/evidence.
- Cargo-mutants lane: `MUT-COLLECT-001`; State 5 writes named mutation-killer tests, formal verifier records waiver if no executable collect-scoped mutants command is available.

Future Kani candidates, not State 5 blockers unless formal verifier assigns them: collect bounds transition safety, page classifier totality, extra schema classifier, evidence capacity no-silent-loss. Each maps to the Verus/TLA/static obligations above.

## 7. Named Mutation-Killer Mapping
If a collect-scoped cargo-mutants lane is wired, use: `cargo mutants -p vb_runtime --file crates/vb_runtime/src/primitives/collect.rs --test-tool nextest -- --no-fail-fast`. If not wired, `MUT-COLLECT-001` waiver must cite this table.

| Mutant class | Example mutant | Exact killer test |
|---|---|---|
| Boundary min page | remove `page_size == 0` check | `collect_start_page_size_zero_returns_invalid_workflow_and_no_state` |
| Boundary max page | change `page_size > limit` to `page_size >= limit` | `collect_start_page_size_at_limit_accepts_exact_limit_and_finishes_without_state` |
| Boundary one-over | change `page_size > limit` to always false | `collect_start_page_size_one_over_limit_returns_page_limit_and_no_state` |
| Overflow candidate | accept `u32::MAX` page size | `collect_start_u32_max_page_size_returns_page_limit_and_no_state` |
| Item exact max | change `count > max` to `count >= max` | `collect_start_exact_max_items_accepts_and_keeps_state_only_when_more_pages_remain` |
| Item one-over | remove item limit branch | `collect_start_one_over_max_items_returns_item_limit_and_no_state` |
| Empty list state leak | omit `states.remove` on empty | `collect_start_empty_list_writes_empty_done_and_no_state` |
| Duplicate collapsed | return generic `InvalidCompiledWorkflow` | `collect_next_duplicate_page_returns_order_violation_duplicate_and_preserves_state` |
| Stale collapsed | return duplicate/generic for stale | `collect_next_stale_page_returns_order_violation_stale_and_preserves_state` |
| Out-of-order collapsed | accept future page or return generic | `collect_next_future_page_returns_order_violation_out_of_order_and_preserves_state` |
| Rejection mutates state | advance cursor on invalid page | all three page-order tests compare pre/post state equality |
| Wrong-run key read | ignore run id in key | `drive_collect_states_are_isolated_for_two_runs_same_collector_slot` |
| Wrong-slot key read | ignore slot in key | `drive_collect_states_are_isolated_for_one_run_two_collector_slots` |
| Wait loses state | recreate collect state after timer | `collect_state_survives_wait_timer_resume_and_next_page_uses_same_source` |
| Ask loses state | recreate collect state after answer | `collect_state_survives_ask_answer_resume_and_next_page_uses_same_source` |
| Recovery skip/repeat | hydrate cursor off by one | `recovered_frame_and_collect_state_from_same_prefix_resume_without_skip_or_repeat` |
| Decode corrupt as valid | ignore postcard decode error | `collect_hydration_corrupt_extra_returns_decode_failed_and_no_state` |
| Empty extra accepted | treat empty as default state | `collect_hydration_empty_extra_returns_empty_extra_error_and_no_state` |
| Run mismatch accepted | omit run validation | `recovered_collect_state_rejects_run_mismatch_and_inserts_no_state` |
| Slot mismatch accepted | omit slot validation | `recovered_collect_state_rejects_slot_mismatch_and_inserts_no_state` |
| Current page mismatch | omit event value/current_page comparison | `collect_hydration_current_page_mismatch_returns_page_mismatch_and_no_state` |
| Schema confusion | decode taint bytes as collect | `collect_hydration_non_collect_taint_extra_is_skipped_without_decode_or_state` |
| Schema blocks valid later event | abort whole scan on skipped non-collect | `collect_hydration_non_collect_extra_does_not_block_later_valid_collect_extra` |
| Evidence capacity drop | silent success with missing extra | `collect_slot_extra_capacity_zero_returns_capacity_error_before_success` and `collect_slot_extra_capacity_full_returns_capacity_error_not_silent_drop` |
| Required evidence not prioritized | capacity 1 holds StepStarted not SlotWritten extra | `collect_slot_extra_capacity_one_preserves_required_slot_written_extra` |

## 8. Hydration / Schema / Evidence Boundary Matrix
| Case | Test | Exact expected outcome |
|---|---|---|
| Empty event list for collect states | `recovered_collect_state_empty_event_list_returns_empty_states` | `Ok(CollectStates::new())`; no state for probed keys. |
| Empty event list for frame | `recovered_frame_empty_event_list_returns_no_recovery_data` | `Err(RecoveryError::NoRecoveryData { run })`. |
| Empty extra bytes | `collect_hydration_empty_extra_returns_empty_extra_error_and_no_state` | `CollectExtraHydrationFailed { kind: EmptyExtra, ... }`; no state. |
| Corrupt bytes | `collect_hydration_corrupt_extra_returns_decode_failed_and_no_state` | `CollectExtraHydrationFailed { kind: DecodeFailed, ... }`; no state. |
| Wrong run | `recovered_collect_state_rejects_run_mismatch_and_inserts_no_state` | `CollectExtraHydrationFailed { kind: RunMismatch { expected, actual }, ... }`; no state. |
| Wrong slot | `recovered_collect_state_rejects_slot_mismatch_and_inserts_no_state` | `CollectExtraHydrationFailed { kind: SlotMismatch { expected, actual }, ... }`; no state. |
| Wrong current page | `collect_hydration_current_page_mismatch_returns_page_mismatch_and_no_state` | `CollectExtraHydrationFailed { kind: CurrentPageMismatch { expected, actual }, ... }`; no state. |
| Non-collect taint extra | `collect_hydration_non_collect_taint_extra_is_skipped_without_decode_or_state` | `Ok` and no state for non-collect slot; never decode as collect. |
| Non-collect then valid collect | `collect_hydration_non_collect_extra_does_not_block_later_valid_collect_extra` | No state for first event; exact state for later valid event. |
| Duplicate sequence | `collect_recovery_prefix_with_duplicate_sequence_fails_before_resume` | `Err(JournalError::DuplicateEvent)`; collect hydration not called. |
| Sequence gap | `collect_recovery_prefix_with_sequence_gap_fails_before_resume` | `Err(JournalError::SequenceGap { expected: EventSeq::new(2), actual: EventSeq::new(3) })`; collect hydration not called. |
| Capacity 0 | `collect_slot_extra_capacity_zero_returns_capacity_error_before_success` | `CollectEvidenceCapacityExceeded { capacity: 0, len: 0, required: "collect SlotWritten extra" }`. |
| Capacity 1 | `collect_slot_extra_capacity_one_preserves_required_slot_written_extra` | exact `SlotWritten { extra: Some(expected_state) }` retained by reservation/preservation contract. |
| Capacity full | `collect_slot_extra_capacity_full_returns_capacity_error_not_silent_drop` | exact capacity error; no silent success with dropped extra. |

## 9. Exact Commands

Existing upstream exact commands remain valid:
```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_start_returns_error_when_source_is_not_list
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_start_enforces_page_size_bounds_before_allocating_page
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_start_items_exceeding_limit_by_one
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_full_lifecycle_single_item_pages
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_next_rejects_duplicate_first_page_response_after_cursor_advanced
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime duplicate_first_page_rejection_preserves_advanced_state
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_next_rejects_stale_completion_page
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime stale_completion_page_rejection_preserves_live_state
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_pagination_extra_round_trips_for_recovery
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_pagination_extra_recovered_journal_rejects_corrupt_bytes
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_pagination_extra_recovered_journal_rejects_identity_mismatch
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_next_time_limit_exceeded_returns_error
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_start_with_time_limit_stores_limit_in_state
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime check_time_limit_returns_error_when_exceeded
bash scripts/rust-verification-gauntlet.sh deep
env VERIFY_BEAD_ID=vb-qi37.3 ALLOW_BEAD_LOCKBUD_WAIVER=1 bash scripts/rust-verification-gauntlet.sh all
moon ci
```

State 5 new commands are the exact command lines listed beside tests 1-31 and proptest/fuzz harnesses above. After State 5 creates those tests, no command name is left for the test-writer to invent.

## 10. Formal Obligation Evidence Mapping
| Obligation | State 5 / later evidence |
|---|---|
| TLA-COLLECT-001 | Tests 9-15 plus all-mode waiver evidence. |
| TLA-COLLECT-002 | Existing recovery extra tests plus tests 16, 24-28. |
| TLA-COLLECT-003 | Tests 16, 19-23. |
| TLA-COLLECT-004 | Tests 9-11 exact page-order taxonomy. |
| TLA-COLLECT-005 | Tests 24-28 exact schema separation. |
| VERUS-COLLECT-001 | Tests 1-8, 31, proptest bounds. |
| VERUS-COLLECT-002 | Tests 6-8, 12-16. |
| VERUS-COLLECT-003 | Tests 9-11, proptest rejected pages. |
| VERUS-COLLECT-004 | Tests 17-18, 24-28, fuzz targets. |
| TEST-* | Existing exact nextest commands and new State 5 named tests. |
| FUZZ-CODEC-001 | Concrete fuzz targets in Section 5 or approved formal waiver. |
| PROP-COLLECT-001 | Concrete proptest harness in Section 4 or approved formal waiver. |
| MIRI-COLLECT-001 | `bash scripts/rust-verification-gauntlet.sh deep`. |
| STATIC-COLLECT-001 | Tests 29-31 plus formal waiver/evidence. |
| MUT-COLLECT-001 | Section 7 named mutation killer map plus waiver if no command lane. |
| API-COLLECT-001 | Exact `EngineError` API taxonomy tests 9-11, 17-18, 24-31. |
| GATE-COLLECT-ALL | all-mode gauntlet with approved waivers and no blocking failures. |

## 11. Remaining Non-State-5 Waivers
These are not unresolved State 5 test placeholders. They are approved upstream waiver/formal obligations:
- `TLA-COLLECT-001` through `TLA-COLLECT-005` / `TLA-WAIVER-COLLECT-001`.
- `VERUS-COLLECT-001` through `VERUS-COLLECT-004` / `VERUS-WAIVER-COLLECT-001`.
- `MIRI-COLLECT-001` for deep gauntlet or scoped Miri waiver.
- `STATIC-COLLECT-001` for formal/static evidence; State 5 still writes tests 29-31.
- `MUT-COLLECT-001` for cargo-mutants lane absence; State 5 still writes mutation-killer tests.
- `FUZZ-CODEC-001` and `PROP-COLLECT-001` only if formal verifier declares fuzz/proptest out of State 5 scope; otherwise Section 4/5 are concrete State 5 harness plans.

## 12. State 5 Exit Criteria
- All 31 State 5 tests exist with the names and commands specified here, unless a listed formal waiver ID explicitly owns the lane.
- ERR-004..ERR-008 tests assert the exact target error variants/kinds/fields from this plan.
- Every rejection/capacity/schema failure asserts pre/post state equality or exact no-state-inserted condition.
- Wait/ask tests assert public shard side effects: pending timers/ask answer, volatile journal events, page contents, source identity, and cleanup by in-memory drop.
- Cross-crate recovery tests construct explicit event prefixes and assert exact frame/state/page outcomes before resume.
- Proptest/fuzz commands are either implemented as Section 4/5 commands or replaced by explicit formal-verifier report citing approved waiver IDs.
- No assertion relies only on `is_ok()` or `is_err()`.

## Self-Check Against Rejected Review
- Error taxonomy resolved: exact `CollectPageOrderViolation`, `CollectExtraHydrationFailed`, and `CollectEvidenceCapacityExceeded` targets chosen.
- State 5 placeholders removed: every must-write test has file/module target, public entry point, Given/When/Then, exact assertion, and intended command.
- Hydration/schema/evidence/boundary cases split into named min/max/empty/corrupt/mismatch/capacity cases.
- Proptest/fuzz/mutation sections now give concrete executable plans, with waiver IDs only for formal-verifier-owned non-State-5 lanes.
- Mutation-killer map names the exact test for every boundary, error-collapse, state-mutation, schema-confusion, and evidence-drop mutant.
- Wait/ask and cross-crate recovery tests specify integration surface, journal prefix construction, side effects, cleanup, and exact page/state assertions.
