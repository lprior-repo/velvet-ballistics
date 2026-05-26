# Red Queen Adversarial Review: vb-jpq7.3

Date: 2026-05-23
Reviewer: red-queen
Verdict: **APPROVE — crown defended for requested current evidence/test scope**

## Commands Run In This Review

- `rtk git status --short` -> **observed pre-existing modified/untracked worktree state**; no staging/commit/push performed.
- `rtk cargo test -p velvet-ballistics-workspace-tests --test vb_jpq7_3_fail_closed_storage_recovery_contract -- --nocapture` -> **PASS**, 11 passed.
- `bash scripts/check-ignored-fallible-results.sh` -> **PASS**, embedded/split `.ok()` fixtures caught and production scan printed `NoViolationFound`.
- `rtk cargo test -p vb_runtime collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page storage_runtime_journal_maps_action_wait_and_ask_events -- --exact --nocapture` -> **operator error**, Cargo accepts one test filter; no product verdict drawn.
- `rtk cargo test -p vb_runtime collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page -- --exact --nocapture` -> **0 tests run** because the exact filter did not match the fully-qualified test name; no product verdict drawn.
- `rtk cargo test -p vb_runtime storage_runtime_journal_maps_action_wait_and_ask_events -- --exact --nocapture` -> **0 tests run** because the exact filter did not match the fully-qualified test name; no product verdict drawn.
- `rtk cargo test -p vb_runtime collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page -- --nocapture` -> **PASS**, 1 passed.
- `rtk cargo test -p vb_runtime storage_runtime_journal_maps_action_wait_and_ask_events -- --nocapture` -> **PASS**, 1 passed.

## Raw Evidence Inspected

- Latest Moon CI raw log: `/home/lewis/.local/share/opencode/tool-output/tool_e54cfc867001em3UkY7dnDZZ7z`.
  - Lines 520-521: ignored-fallible fixtures include `DISCARD-003 embedded ok lossy` and `DISCARD-003 split ok lossy`, both exit 2.
  - Line 548: ignored-fallible production scan `NoViolationFound`.
  - Lines 543-546: `velvet-ballistics:supply-chain` completed.
  - Line 558: `test integrity: PASS base=HEAD`.
  - Lines 808 and 872: `Starting 12169 tests across 171 binaries`; `12169 tests run: 12169 passed (5 slow), 0 skipped`.
  - Line 983: `Tasks: 25 completed (3 cached)`.
- Versioned slot-write extra envelope: `crates/vb_storage/src/slot_extra.rs:6-69`.
- Full-journal recovery taint path: `crates/vb_storage/src/recovery/replay/summary.rs:390-460`.
- Runtime journal encode path: `crates/vb_runtime/src/journal/chunk_002.rs:181-234`.
- Collect hydration decode path: `crates/vb_runtime/src/primitives/collect.rs:232-272`.
- Public contract tests: `crates/workspace_tests/tests/vb_jpq7_3_fail_closed_storage_recovery_contract.rs:57-76,261-333`.
- Fallible-result scanner: `scripts/check-ignored-fallible-results.sh:93-147,213-224,266-318`.
- Proof limitations: `.beads/vb-jpq7.3/proof-to-implementation.md:7-11,181-190`, `.beads/vb-jpq7.3/proof-strategy.md:34-45`, `.beads/vb-jpq7.3/verifier-lane-review.md:10-15,39,85`.

## Adversarial Scenarios Attempted

| Scenario / mutation question | Defense observed | Status |
|---|---|---|
| Corrupt prefixed `SlotWrittenEvent.extra` envelope payload could be laundered through legacy fallback and classified as clean taint. | `decode_slot_written_extra` only treats bytes without `SLOT_WRITTEN_EXTRA_PREFIX` as legacy. Prefixed bytes with undecodable payload return `SlotWrittenExtraError::DecodeFailed`; `decoded_slot_taint` maps any decode error to `RecoveryError::CorruptSlotTaint { slot }`. The 11-test workspace contract rerun passed the corrupt-prefixed full-journal hydration assertion. | **Defended** |
| Legacy non-prefixed collect/frame extra could be misclassified as corrupt taint after adding the prefix. | Non-prefixed bytes return `DecodedSlotWrittenExtra::LegacyFrameExtra`; recovery records legacy fallback taint with `unsupported: true`, and collect hydration routes legacy bytes into `hydrate_frame_extra`. Workspace test `given_legacy_collect_frame_extra_when_hydrating_full_journal_then_extra_is_not_corrupt_taint` passed in the 11-test suite. | **Defended** |
| Current runtime envelope containing both taint and optional frame extra could decode taint but drop collect frame extra during full-journal recovery/collect hydration. | Runtime writes `encode_slot_written_extra(taint, extra)` into `SlotWrittenEvent.extra`; recovery uses envelope taint; collect hydration unwraps `Envelope(frame_extra: Some(...))` and calls `hydrate_frame_extra`. Targeted runtime collect hydration test passed. | **Defended** |
| Runtime journal encode errors could be erased by `.ok()`/defaulting. | Storage conversion uses `encoded_slot_taint_extra(taint, extra)?`, and that function maps `encode_slot_written_extra` failure to `RuntimeError::EncodeFailed`; no `.ok()` is present in the production path. Runtime journal mapping test passed; scanner production pass found no ignored fallible results. | **Defended** |
| Embedded `.ok()` scanner evasion could survive inside a larger expression after a recognized fallible source. | Scanner compacts each line and records `DISCARD-003` when the same compact line contains `.ok()`/`.err()` and recognized fallible sources such as `from_bytes`/`to_allocvec`. Fixture `postcard::from_bytes::<u8>(bytes).ok().unwrap_or(0)` was caught in current local scanner run and Moon log. | **Defended** |
| Split-chain `.ok()` scanner evasion could survive when the fallible source and `.ok()` are on different lines. | Scanner tracks `pending_lossy_line_no` for unterminated fallible-source lines and records `DISCARD-003` when a following continuation line begins with `.` and contains `.ok()`/`.err()`. Split fixture was caught in current local scanner run and Moon log. | **Defended** |
| Latest global evidence could be stale after the versioned slot-write extra envelope repair. | Closure evidence now points at `tool_e54cfc867001em3UkY7dnDZZ7z`, not the older `12167`-test log. Raw log shows supply-chain completed, test-integrity PASS, scanner `NoViolationFound`, and `12169/12169` tests passed. | **Defended** |
| Formal/proof package could overclaim live Fjall/`RunFrame`/codec proof. | Current bridge/review artifacts explicitly state Verus is auxiliary/spec-seam only, TLA+ is bounded abstract evidence with `MaxSeq = 3`, Kani is scoped to allocation-free seams, and live Fjall/`RunFrame`/codec/replay/hydration behavior is closed by behavior tests, source scans, and trusted-base declarations. | **Defended with limitation preserved** |

## Surviving Mutants / Blockers

No surviving requested-scope mutants identified.

Blockers: **none**.

## Non-Blocking Notes

- The first attempted multi-filter Cargo invocation and the two `--exact` runtime invocations were operator/filter mistakes, not product defects; corrected filtered runtime test commands passed.
- The formal package remains approval-worthy only with the stated limitations. Do not cite Verus/TLA+/Kani artifacts as full formal proof of live Fjall replay, codec behavior, or full `RunFrame` hydration.
- Worktree already contained many modified/untracked files before this review. This review intentionally wrote only `.beads/vb-jpq7.3/red-queen-report.md` and did not stage, commit, push, close beads, or edit production code.

## Final Verdict

Approve for bead `vb-jpq7.3` at the requested Red Queen evidence/test scope. The corrupt prefixed slot-write extra envelope mutation is killed by implementation and public tests; legacy non-prefixed collect/frame extra remains compatible; current runtime taint-plus-frame-extra envelopes hydrate through recovery and collect paths; runtime encode errors are not erased; embedded/split `.ok()` scanner evasions are caught; latest Moon evidence is current at `tool_e54cfc867001em3UkY7dnDZZ7z` with `12169` tests; and proof limitations are preserved without overclaim.
