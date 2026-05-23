# vb-jpq7.3 Behavior Test Review

STATUS: APPROVED

## Findings

1. **Resolved P0: full-journal corrupt prefixed slot-taint metadata is covered fail-closed.**
   - `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:261-295` exercises the public `hydrate_run_frame_from_events(&events, run)` seam with a full-journal `SlotWrittenEvent` whose `value` is valid but whose versioned `extra` prefix (`SLOT_WRITTEN_EXTRA_PREFIX`) is followed by corrupt payload bytes.
   - The test asserts the exact public error variant and field: `Err(RecoveryError::CorruptSlotTaint { slot })` with `observed == slot`.
   - Mutation thought experiment: deleting the prefixed-envelope decode failure branch or falling back to legacy/default Clean on corrupt prefixed bytes is caught by this public contract test and by the focused storage unit test.

2. **Legacy/current extra schema parity is now behavior-tested.**
   - Current schema: `crates/vb_storage/src/slot_extra.rs:63-68` only treats bytes with `VBSE\x01` as the current envelope; corrupt prefixed payload returns `DecodeFailed` and recovery maps that to `RecoveryError::CorruptSlotTaint` in `crates/vb_storage/src/recovery/replay/summary.rs:447-461`.
   - Legacy schema: `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:297-334` passes real `CollectPaginationState` postcard bytes as legacy frame extra and asserts slot value plus `Taint::Clean`, proving those bytes are not misclassified as corrupt taint.
   - Collect hydration parity: `crates/vb_runtime/src/primitives/collect.rs:254-266` hydrates both `Envelope { frame_extra: Some(_) }` and `LegacyFrameExtra(_)`; the targeted runtime collect test passes.

3. **Runtime write path preserves the current envelope contract.**
   - `crates/vb_runtime/src/journal/chunk_002.rs:181-193` encodes every runtime `SlotWritten` event with `encode_slot_written_extra(taint, extra)` before storage append.
   - `crates/vb_runtime/src/journal/tests/chunk_002.rs:72-144` asserts the exact persisted `JournalEvent::SlotWrittenEvent` including `extra: vb_storage::encode_slot_written_extra(Taint::Clean, None).ok()`, killing mutations that write raw legacy extra or drop the sidecar.

4. **Workspace contract suite is deterministic and public-API oriented.**
   - The inspected workspace contract file now contains 11 deterministic `#[test]` scenarios and no ignored tests.
   - Public behavior scenarios cover replay limits, zero replay bound rejection, first-tail sequence gap, explicit close durability observability, snapshot lookup non-erasure, taint read fail-closed, corrupt current envelope fail-closed, legacy collect/frame-extra compatibility, bounded post-snapshot scan, and API/source scanner guardrails.
   - Source-string scanner assertions remain acceptable here because the dangerous paths also have direct behavior tests and exact public error assertions; they are not the sole proof for taint or replay behavior.

## Commands run by reviewer

- `rustup run nightly-2026-04-28 cargo test -p velvet-ballastics-workspace-tests --test vb_jpq7_3_fail_closed_storage_recovery_contract` => `11 passed; 0 failed; 0 ignored`.
- `rustup run nightly-2026-04-28 cargo test -p vb_storage hydrate_run_frame_from_events` => `5 passed; 0 failed; 0 ignored` for the filtered storage hydration tests.
- `rustup run nightly-2026-04-28 cargo test -p vb_runtime storage_runtime_journal_maps_action_wait_and_ask_events` => focused runtime journal mapping test passed.
- `rustup run nightly-2026-04-28 cargo test -p vb_runtime collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page` => focused collect extra compatibility test passed.

## Evidence inspected

- Latest Moon log `/home/lewis/.local/share/opencode/tool-output/tool_e54cfc867001em3UkY7dnDZZ7z` ends with `Tasks: 25 completed (3 cached)` and is cited by the ledger as `12169 tests run: 12169 passed (5 slow), 0 skipped`.
- `.beads/vb-jpq7.3/verification-ledger.jsonl:vl-003` records the workspace contract test as `11 passed; 0 failed`.
- `.beads/vb-jpq7.3/verification-ledger.jsonl:vl-030` records `vb_storage hydrate_run_frame_from_events` as `5 passed; 0 failed`.
- `.beads/vb-jpq7.3/verification-ledger.jsonl:vl-033` records the runtime collect hydration and journal mapping targeted tests as passed.
- `.beads/vb-jpq7.3/verification-ledger.jsonl:vl-035` records the current canonical `moon ci` pass after the versioned slot-write extra envelope repair.
- `.beads/vb-jpq7.3/traceability-matrix.jsonl` maps taint fail-closed, no-silent-discard, bounded replay, strict tail replay, explicit close, and global readiness requirements to the repaired tests and current evidence.

## Remaining blockers

None in the inspected vb-jpq7.3 behavior-test scope.

## Files written

- `.beads/vb-jpq7.3/test-review.md`
