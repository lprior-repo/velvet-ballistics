# Contract Specification: vb-qi37.3 collect pagination durability/hydration

## Context
- Bead: `vb-qi37.3` - runtime: prove collect pagination durability and hydration.
- Authoritative clauses: `velvet-ballistics-MASTER.md` lines 95, 459, 635-638, 1478, 1508-1509, 2307-2356, 3414, 3697.
- Scoped crates: `vb_runtime`, `vb_storage`, `vb_core` per `delivery-scope.jsonl`.
- Existing API facts read: `CollectPaginationState`, `CollectStates`, `hydrate_collect_states_from_recovered_journal`, `collect_start`, `collect_next`, `collect_finish`, `EvidenceEvent::SlotWritten { extra }`, `JournalEvent::SlotWrittenEvent { extra }`, `hydrate_run_frame_from_events`, and `EngineError`.
- Skill startup: `/home/lewis/.claude/skills/rust-contract/SKILL.md` and `/home/lewis/.agents/skills/rust-contract/SKILL.md` both specify contract-first, TLA+ temporal default, Verus-first Rust core, valid JSONL proof obligations, and no implementation/proof/test code. No conflict observed.

## Assumptions
- `CollectPaginationState` is the durable continuation token for collect. It must be persisted only as collect-specific extra, never confused with taint bytes.
- `CollectStates` is owned per active run/shard state and keyed by `(RunId, SlotIdx)`; no global collect state is permitted.
- Exact TLA+/Verus module paths for collect pagination do not exist in this workspace. State 3 therefore records scoped temporary TLA+/Verus waivers with owner, expiry, limitation, and compensating executable evidence instead of inventing proof/model files.
- Existing Moon verification lanes are real: `moon run :verify-proof`, `moon run :verify-deep`, `moon run :verify-standard`, and `moon ci` via `.moon/tasks/all.yml`.

## Open Questions / State 4 Review Checks
- OQ-001: Should `EngineError` gain dedicated `DuplicateCollectPage`, `StaleCollectPage`, and `OutOfOrderCollectPage` variants, or one structured `CollectPageOrderViolation { kind }` variant? Current `InvalidCompiledWorkflow { reason: "collect pagination state missing" }` is not sufficiently typed.
- OQ-002: Should `JournalEvent::SlotWrittenEvent.extra` become tagged/discriminated, or should hydration be given collect-slot metadata so non-collect taint bytes are skipped before decode?
- OQ-003: Should `EvidenceCollector` return a typed error at capacity for required collect `SlotWritten` extras, or reserve capacity for required evidence so it cannot silently drop collect durability state?

## Preconditions
- PRE-001: `collect_start` input `source` must resolve to a list in the run frame and value store; otherwise return `EngineError::TypeMismatch`.
- PRE-002: `page_size` must be nonzero and `page_size <= limit`; otherwise return `EngineError::InvalidCompiledWorkflow` for zero size or `EngineError::CollectPageLimitExceeded` for size above limit.
- PRE-003: Source item count must satisfy `item_count <= limit` and `limit <= ResourceContract.max_collect_items`; otherwise return `EngineError::CollectItemLimitExceeded` or admission/budget rejection.
- PRE-004: `collect_next` input `collector_slot` must contain the exact current page list id recorded in durable state for `(run_id, collector_slot)`.
- PRE-005: Hydration input events must be ordered recovered journal events for one run, and collect-extra decode must only be attempted for events proven to be collect pagination extras.
- PRE-006: Frame hydration and collect side-table hydration must consume the same recovered event prefix/snapshot-tail boundary for the same `RunId`.
- PRE-007: Evidence collection capacity must be sufficient or fail-closed before a required collect `SlotWritten` durable extra is dropped.

## Postconditions
- POST-001: Nonempty `collect_start` writes the first bounded page to the collector slot and upserts exactly one continuation state keyed by `(run_id, collector_slot)` when more items remain.
- POST-002: Empty or single-page `collect_start` writes an empty/final page, routes to `done`, and removes any active continuation state for `(run_id, collector_slot)`.
- POST-003: Successful `collect_next` advances the cursor monotonically by the emitted page length, writes the next page, and updates the durable state to the new page id when more items remain.
- POST-004: Final `collect_next` writes the final/empty terminal page, routes to `done`, and removes continuation state.
- POST-005: `collect_finish` writes the collector value to output, preserves taint, and removes continuation state for `(run_id, collector_slot)`.
- POST-006: Journaled `SlotWrittenEvent` for an active collect page contains a durable, decodeable collect extra that reconstructs the same `(run_id, collector_slot, current_page, cursor, source, page_size, item_count, limit, time_limit_ms, start_millis)`.
- POST-007: Recovery/hydration reconstructs both `RunFrame` slots/pc/step states and `CollectStates` so resume continues at the exact next page without repeating or skipping items.
- POST-008: Stale, duplicate, and out-of-order page completions return typed `EngineError` variants and leave live collect state unchanged.

## Invariants
- INV-001: Collect state isolation: no state entry may be read, overwritten, or removed unless its key exactly matches `(RunId, collector_slot)` for the active run and node.
- INV-002: Current-page identity: a `collect_next` is valid only when the page id in `collector_slot` equals `CollectPaginationState.current_page` for the same key.
- INV-003: Cursor boundedness: `0 <= cursor <= item_count <= limit <= ResourceContract.max_collect_items`, and `page_size > 0 && page_size <= limit` always hold for persisted collect state.
- INV-004: Cursor monotonicity: across successful page transitions for one key, cursor never decreases and increases by at most `page_size` until completion.
- INV-005: Source stability: the hydrated `source` list id and item count for an active collect state do not change across wait, ask, replay, recovery, and resume.
- INV-006: Durability-before-resume: a run that can be resumed after a collect page write must have durable journal evidence for the slot value and matching collect extra, or must fail recovery with a typed error.
- INV-007: Extra schema separation: non-collect taint bytes in `SlotWrittenEvent.extra` must never be interpreted as collect pagination state.
- INV-008: Hydration coherence: `hydrate_run_frame_from_events` and collect side-table hydration must be joined by the same event sequence and `RunId`; mismatched identity or corrupt collect extra fails closed.
- INV-009: Evidence no-silent-loss: required collect `SlotWritten` extra cannot be silently dropped by `EvidenceCollector`; capacity exhaustion must produce typed failure or preserve required collect evidence.
- INV-010: Time/resource bounds: collect checks page, item, time, value-store arena, and evidence capacity bounds before state advances or state mutation becomes externally visible.

## Error Taxonomy Contract
- ERR-001: `EngineError::CollectPageLimitExceeded` when `page_size > limit` or represented page bounds are exceeded.
- ERR-002: `EngineError::CollectItemLimitExceeded` when source item count exceeds collect limit or `ResourceContract.max_collect_items`.
- ERR-003: `EngineError::CollectTimeLimitExceeded` when elapsed collect time exceeds `time_limit_ms` before `collect_next` advances.
- ERR-004: Typed duplicate page error required: duplicate completion of a page already advanced must be rejected and state unchanged.
- ERR-005: Typed stale page error required: completion of an older page id/cursor must be rejected and state unchanged.
- ERR-006: Typed out-of-order page error required: completion of a future/unknown page id for the same key must be rejected and state unchanged.
- ERR-007: Typed collect-extra decode/identity error required: corrupt, mismatched, or non-collect extra must fail closed without poisoning unrelated state.
- ERR-008: Typed evidence-capacity error required: collect durability evidence capacity exhaustion must not be represented as silent success.

## Contract Signatures (existing or required surfaces)
- Existing: `CollectStates::upsert(state: CollectPaginationState) -> Result<(), EngineError>`.
- Existing: `CollectStates::find(run_id: RunId, collector_slot: SlotIdx, current_page: ListId) -> Option<CollectPaginationState>`; contract requires downstream replacement/refinement that can distinguish missing/duplicate/stale/out-of-order causes as typed `EngineError`.
- Existing: `CollectStates::hydrate_extra(run_id: RunId, collector_slot: SlotIdx, extra: &[u8]) -> Result<(), EngineError>`; contract requires it only be called for collect-tagged/proven extras.
- Existing: `hydrate_collect_states_from_recovered_journal(events: &[JournalEvent]) -> Result<CollectStates, EngineError>`; contract requires same-run ordered recovered event stream and collect-extra filtering.
- Existing: `collect_start(...) -> Result<EngineSignal, EngineError>`.
- Existing: `collect_next(...) -> Result<EngineSignal, EngineError>`.
- Existing: `collect_finish(...) -> Result<EngineSignal, EngineError>`.
- Required integration surface: hydrate recovered `RunFrame` and `CollectStates` together from the same replay evidence before resuming a mid-collect run.

## Verus-Owned Clauses
- PRE-002, PRE-003, POST-001 through POST-005, INV-001 through INV-005, INV-010, ERR-001 through ERR-006 are Rust-local state-transition and bounds properties suitable for Verus after proof surfaces exist.
- Temporary Verus waiver boundary: no `verification/verus/*.rs` target exists in this workspace, so Verus proof is waived only for State 3 artifact consumption. Owner: State 6 implementer with State 4 reviewer approval. Expiry: before release-critical acceptance of `vb-qi37.3` unless a real Verus proof target is added earlier. Limitation: runtime tests/proptest-like scenarios do not prove all abstract states. Compensating evidence: exact `vb_runtime` nextest commands listed in `proof-obligations.jsonl`, plus direct release gauntlet `env VERIFY_BEAD_ID=vb-qi37.3 ALLOW_BEAD_LOCKBUD_WAIVER=1 bash scripts/rust-verification-gauntlet.sh all`.

## TLA+-Owned Clauses
- POST-006, POST-007, INV-006, INV-008, and wait/ask/replay/resume preservation of INV-001 through INV-005 are temporal state-over-time properties and require a collect-specific TLA+ model.
- Temporary TLA+ waiver boundary: existing TLA specs (`RecoveryReplay`, `JournalBeforeDispatch`, `BoundedAdmission`, `AttemptTracking`, `ShardOwnership`) do not model collect pagination cursor/source stability or collect-extra hydration. Owner: State 6 implementer with State 4 reviewer approval. Expiry: before release-critical acceptance of `vb-qi37.3` unless `specs/tla/CollectPagination.*` or equivalent is added earlier. Limitation: exact runtime/storage tests cover known scenarios but cannot exhaust temporal interleavings. Compensating evidence: exact recovery, stale/duplicate, source-stability, time-limit, and all-mode gauntlet commands in `proof-obligations.jsonl`.

## Theorem-Owned Clauses
- None required beyond Verus/TLA+ at this bead scope. Lean/Aeneas/Hax are waived unless State 4 identifies a tiny algebraic codec/refinement kernel that Verus cannot express.

## Planned Test Scenarios (for traceability only; no tests implemented here)
- `given_empty_source_when_collect_start_runs_then_done_and_no_pagination_state`
- `given_final_first_page_when_collect_start_runs_then_done_and_state_removed`
- `given_mid_collect_when_wait_suspends_and_resume_runs_then_next_page_uses_same_state`
- `given_recovered_mid_collect_when_hydrated_then_resume_emits_next_page_without_skip_or_repeat`
- `given_two_runs_same_collector_slot_when_pages_advance_then_states_are_isolated_by_run`
- `given_duplicate_page_completion_when_collect_next_runs_then_typed_duplicate_error_and_state_unchanged`
- `given_stale_page_completion_when_collect_next_runs_then_typed_stale_error_and_state_unchanged`
- `given_out_of_order_page_completion_when_collect_next_runs_then_typed_out_of_order_error_and_state_unchanged`
- `given_non_collect_taint_extra_when_collect_hydration_scans_journal_then_extra_is_ignored_not_decoded_as_collect`
- `given_evidence_capacity_full_when_collect_slot_extra_required_then_typed_capacity_error_or_required_extra_preserved`

## Non-goals
- No performance speedup claim is made by this contract.
- No production code, proof code, harness code, or tests are implemented in State 3.
