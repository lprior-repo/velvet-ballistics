# QA Evidence Audit — vb-jpq7.3

Date: 2026-05-23
Workspace: `/home/lewis/src/velvet-ballistics`
Scope: hands-on evidence audit only. No production code edited. No staging, commit, push, or bead close performed.

## Verdict

**BLOCKED FOR FINAL CLOSURE PACKAGING.** Current behavior/Moon/Kani evidence passes in the audited scope, but `.beads/vb-jpq7.3/black-hat-review.md` still contains `Verdict: **REJECT FOR CLOSURE**` and a final `**REJECT FOR CLOSURE.**` decision. Per closure instructions, the stale black-hat reject must be refreshed or explicitly superseded before the bead can be packaged as closed.

## Evidence audited

- Latest Moon CI raw log: `/home/lewis/.local/share/opencode/tool-output/tool_e54ad4ea40019LkG7p2r0N30AH`
  - Marker check passed: `Tasks: 25 completed (5 cached)`.
  - Marker check passed: `12167 tests run: 12167 passed (5 slow), 0 skipped`.
  - Marker check passed: `test integrity: PASS base=HEAD`.
  - Marker check passed: `velvet-ballastics:panic-surface | NoViolationFound`.
  - Marker check passed: `velvet-ballastics:ignored-fallible-results | NoViolationFound`.
  - Supply-chain task marker present: `velvet-ballastics:supply-chain`.
- Scoped Kani raw log: `/home/lewis/.local/share/opencode/tool-output/tool_e543ab843002yJmWdm7rPpi1ed`
  - Counted `12` occurrences of `VERIFICATION:- SUCCESSFUL`.
  - Counted `12` occurrences of `Complete - 1 successfully verified harnesses, 0 failures, 1 total.`
  - Counted `0` occurrences of `VERIFICATION:- FAILED`, `FAILURE`, and `UNSATISFIED`.
- JSON/JSONL parse audit passed:
  - `verification-ledger.jsonl`: 32 records.
  - `traceability-matrix.jsonl`: 9 records.
  - `proof-obligations.planned.jsonl`: 16 records.
  - `delivery-scope.jsonl`: 1 record.
  - `agent-invocation-ledger.jsonl`: 8 records.
  - `waiver-candidates.jsonl`: 6 records.
  - `verifier-lane-decisions.jsonl`: 72 records.
  - `kani-list.json`: valid JSON object.
- Review status inspected:
  - `proof-review.md`: `STATUS: APPROVED`, with explicit limitations.
  - `test-review.md`: `STATUS: APPROVED`, but still mentions older 9-test / older Moon evidence text.
  - `test-suite-review.md`: `STATUS: APPROVED`, but still mentions older 9-test evidence.
  - `black-hat-review.md`: **still rejects closure**. This is the closure-packaging blocker.

## Commands run by this audit

```bash
python3 - <<'PY'
# Parsed bead JSON/JSONL files.
PY
```

Observed output:

```text
PARSE_OK verification-ledger.jsonl records=32
PARSE_OK traceability-matrix.jsonl records=9
PARSE_OK proof-obligations.planned.jsonl records=16
PARSE_OK delivery-scope.jsonl records=1
PARSE_OK agent-invocation-ledger.jsonl records=8
PARSE_OK waiver-candidates.jsonl records=6
PARSE_OK verifier-lane-decisions.jsonl records=72
PARSE_OK kani-list.json type=dict size=6
```

```bash
python3 - <<'PY'
# Checked Moon and Kani raw log markers.
PY
```

Observed output:

```text
MARKER_OK moon_tasks Tasks: 25 completed (5 cached)
MARKER_OK moon_tests 12167 tests run: 12167 passed (5 slow), 0 skipped
MARKER_OK test_integrity test integrity: PASS base=HEAD
MARKER_OK panic_surface velvet-ballastics:panic-surface | NoViolationFound
MARKER_OK ignored_fallible velvet-ballastics:ignored-fallible-results | NoViolationFound
MARKER_OK supply_chain velvet-ballastics:supply-chain
KANI_SUCCESS_COUNT 12
KANI_COMPLETE_COUNT 12
KANI_BAD_MARKER_COUNT VERIFICATION:- FAILED 0
KANI_BAD_MARKER_COUNT FAILURE 0
KANI_BAD_MARKER_COUNT UNSATISFIED 0
```

```bash
bash scripts/check-ignored-fallible-results.sh
```

Observed output excerpt:

```text
FixturePass: DISCARD-003 embedded ok lossy exit=2
FixturePass: DISCARD-003 split ok lossy exit=2
ScanDomain: crates/*/src xtask/src
NonProductionExcluded: tests benches examples fuzz target .beads fixtures
NoViolationFound
```

```bash
rustup run nightly-2026-04-28 cargo test -p vb_storage hydrate_run_frame_from_events_rejects_corrupt_slot_taint_metadata
```

Observed output excerpt:

```text
test recovery::tests::hydrate_run_frame_tests::hydrate_run_frame_from_events_rejects_corrupt_slot_taint_metadata ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1019 filtered out; finished in 0.00s
```

```bash
rustup run nightly-2026-04-28 cargo test -p velvet-ballastics-workspace-tests --test vb_jpq7_3_fail_closed_storage_recovery_contract
```

Observed output:

```text
running 10 tests
test given_zero_replay_limit_when_constructed_then_limit_is_rejected_before_replay ... ok
test given_run_event_replay_api_when_public_contract_is_scanned_then_unbounded_vec_api_is_not_the_only_path ... ok
test given_snapshot_index_read_fails_when_events_for_run_starts_then_error_is_not_erased ... ok
test given_full_journal_slot_taint_metadata_is_corrupt_when_hydrating_then_recovery_fails_closed ... ok
test given_journal_shutdown_when_durability_barrier_fails_then_drop_does_not_discard_result ... ok
test given_public_hydration_tail_slot_cannot_be_dimensioned_when_recovery_runs_then_clean_taint_is_not_defaulted ... ok
test given_explicit_replay_limit_when_more_events_exist_then_too_many_events_and_code_are_returned ... ok
test given_snapshot_after_many_old_events_when_replaying_then_pre_snapshot_work_does_not_exhaust_limit ... ok
test given_first_tail_event_is_missing_when_replaying_run_then_sequence_gap_points_after_snapshot ... ok
test given_close_after_unpersisted_append_when_reopened_then_event_is_observable ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

## Blockers

1. **Closure-packaging blocker:** stale `.beads/vb-jpq7.3/black-hat-review.md` still says `REJECT FOR CLOSURE`. A refreshed APPROVE was not present at audit time.
2. **Artifact-staleness caution:** `proof-review.md`, `test-review.md`, and `test-suite-review.md` still contain older Moon/test-count references in places, although their status lines are approved and the latest evidence files point to the newer 12167-test Moon pass and 10-test workspace contract run.

## Files written

- `.beads/vb-jpq7.3/qa-review.md`
- `.beads/vb-jpq7.3/qa-enforcer-report.md`
