# vb-pcu4h Evidence Bundle

## Captured: 2026-07-01

## Targeted Test Run — 3 Strengthened Tests

Command:

```bash
cargo test -p vb_storage --lib -- \
  unresolved_action_marks_pending_action_recovery_unsupported \
  action_scheduled_ticket_advances_max_slot_and_step_dimensions \
  crash_after_schedule_then_recover_hydrates_resume_queue
```

Result: **3 passed, 1527 filtered out (1 suite, 0.00s)**

Full log: `three_strengthened_tests.log`

## Broad Recovery Test Run

Command:

```bash
cargo test -p vb_storage --lib recovery
```

Result: **250 passed, 1280 filtered out (1 suite, 0.41s)**

Full log: `vb_storage_recovery_tests.log`

## Workspace Tests Run

Command:

```bash
cargo test -p velvet-ballistics-workspace-tests
```

Result: see `workspace_tests.log`

Passing test suites include (extract):
- 3 passed
- 2 passed
- 16 passed
- 5 passed
- 11 passed
- 2 passed
- 31 passed (1 ignored)
- 76 passed
- 78 passed
- 7 passed
- 21 passed
- 28 passed (1 ignored)
- 8 passed
- 35 passed
- 34 passed
- 4 passed
- 31 passed
- 18 passed
- 25 passed
- 28 passed
- 22 passed
- 14 passed
- 13 passed
- 11 passed
- 2 passed (6 ignored)
- 7 passed
- 6 passed
- 13 passed
- 16 passed
- 70 passed
- 9 passed
- 9 passed
- 3 passed
- 8 passed
- 4 passed (1 ignored)
- 19 passed
- 2 passed (1 ignored)
- 47 passed
- 21 passed, **1 failed**: `given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied`

### Single Failure Classification

Test: `given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied`
Location: `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs:1466`
Panic: `assertion 'left == right' failed; left: false, right: true`

The test asserts that `crates/vb_runtime/src/admission.rs` contains the
string `"impl AcceptedArtifactStore for AlwaysPresentArtifactStore"`.
This is a static source-code grep regression test for admission layer
plumbing — **completely unrelated to recovery pending actions**.

This bead's only modification is
`crates/vb_storage/src/recovery/replay/summary/tests.rs`. The admission
test failure is pre-existing on the parent commit and is classified as a
`BLOCK_GLOBAL` prerequisite repair per the Holzman
`scope_aware_blocking` rule, not a defect in this bead's delivery scope.

## Test Outcomes Summary

| Scope | Result |
|-------|--------|
| 3 PRIMARY strengthened tests | PASS |
| All vb_storage recovery tests (250) | PASS |
| workspace_tests | MIXED — 1 pre-existing BLOCK_GLOBAL failure in strict admission (unrelated to recovery pending actions) |