# §17 Dead-Letter Recovery Plan

This document tracks the remediation plan for each of the 11 Section 17
runtime error codes that are defined in `velvet-ballistics-MASTER.md` §17
but are absent from the runtime `runtime_code()` surface. The full
audit trail lives in
`to-fix/13-section17-deadletter-recovery-plan.md`; this document is
the bead-acceptance summary.

The 11 codes are split into 4 SHIP-BLOCKER items and 7 mechanical
follow-ups, in priority order. Each entry below lists: defect,
production construction site, fix site, and acceptance test name.

## SHIP-BLOCKER Items (must land first)

### 1. `SECRET_UNAVAILABLE`
- **Defect:** `vb_storage::JournalError::SecretUnavailable` is mapped
  to `ARTIFACT_MALFORMED_CODE` by the catch-all bucket in
  `crates/vb_storage/src/error/codes.rs:113-124`. A
  security-classified failure is reported as ARTIFACT_MALFORMED,
  losing the secret-rotation signal.
- **Construction site:** `vb_storage::JournalError::SecretUnavailable`
  at `crates/vb_storage/src/error/mod.rs:206-207`.
- **Fix site:**
  - `crates/vb_storage/src/error/codes.rs` — split `SecretUnavailable`
    into its own arm with a new `SECRET_UNAVAILABLE_CODE =
    DiagnosticCode::new(0x4040)`.
  - `crates/vb_runtime/src/error/conversions.rs` — match
    `JournalError::SecretUnavailable` and construct
    `RuntimeError::SecretUnavailable`.
  - `crates/vb_runtime/src/error/diagnostics.rs` — add a
    `Self::SecretUnavailable => Some("SECRET_UNAVAILABLE")` arm.
- **Acceptance test:**
  `runtime_error_secret_unavailable_emits_secret_unavailable_code`
  (drives the `From<JournalError>` route end-to-end).

### 2. `REPLAY_DIVERGED`
- **Defect:** `RecoveryError::ReplayDivergence` is caught at
  `crates/vb_cli/src/replay.rs:101-114` and routed to
  `CliExitCode::StorageError` (exit 5) instead of
  `CliExitCode::ReplayDivergence` (exit 8). The duplicate
  `cmd_replay` in `crates/vb_cli/src/storage.rs:266-303` is also dead.
- **Construction site:** `RecoveryError::ReplayDivergence` in
  `crates/vb_storage/src/recovery/types.rs`.
- **Fix site:**
  - `crates/vb_cli/src/replay.rs` — split the error match:
    `ReplayDivergence` → exit 8; other storage errors → exit 5.
  - `crates/vb_cli/src/storage.rs` — delete the duplicate `cmd_replay`.
  - `crates/vb_runtime/src/error/diagnostics.rs` — add a
    `RuntimeError::ReplayDivergence` arm.
- **Acceptance test:**
  `cmd_replay_returns_divergence_exit_code_on_replay_failure`
  (CLI integration test).

### 3. `WAIT_TIMEOUT` and `ASK_TIMEOUT`
- **Defect:** `ShardCommand::TimerFired` for a timeout-bearing
  `WaitEvent` or `Ask` node does nothing distinct from a normal
  timer resume. Incident triage cannot distinguish "deadline
  elapsed" from "stale fire".
- **Construction site:** `handle_timer` at
  `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:79-114`.
- **Fix site:**
  - `crates/vb_runtime/src/error/mod.rs` — add `WaitTimeout { run,
    step, deadline }` and `AskTimeout { run, step, deadline }`.
  - `crates/vb_runtime/src/shard/timer.rs` — add `is_timeout: bool`
    flag on `PendingTimer`, default `false`.
  - `crates/vb_runtime/src/shard/transitions.rs` — set `is_timeout`
    on WaitEvent/Ask timer registration.
  - `crates/vb_runtime/src/error/diagnostics.rs` — arms for the two
    new variants.
  - `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` — if
    `is_timeout` is true, return the timeout variant before
    advancing the run.
- **Acceptance test:**
  `red_wait_event_with_timeout_returns_timeout_outcome` and
  `red_ask_with_timeout_returns_ask_timeout_outcome`.

### 4. `STEP_SKIPPED_REFERENCE`
- **Defect:** A step reference that resolves to a `Skipped` step
  currently returns `CoreError::InvalidProgramCounter` or
  `CoreError::StepStateOutOfBounds`, both of which emit `STORAGE_ERROR`
  in runtime_code() space. The semantic of "we tried to follow a
  reference but the target was skipped" is lost.
- **Construction site:** `set_pc`/`step_once` traversal in
  `crates/vb_core/src/engine/step.rs` and `run_loop.rs`.
- **Fix site:**
  - `crates/vb_core/src/errors.rs` — add
    `StepSkippedReference { from, target }` plus
    `STEP_SKIPPED_REFERENCE_RUNTIME_CODE` and
    `STEP_SKIPPED_REFERENCE_CODE = DiagnosticCode::new(0x140E)`.
  - `crates/vb_core/src/engine/step.rs` — when the pc target's
    `StepState` is `Skipped`, construct the new error.
- **Acceptance test:**
  `step_skipped_reference_runtime_code_is_step_skipped_reference`.

## Mechanical Follow-Ups

### 5. `INPUT_MAPPING_FAILED`
- **Defect:** Input mapping errors originate in `vb_cli` only;
  runtime_code() never sees them and the exit code is
  `CompileFailed` (3) instead of `InputMappingFailed` (9).
- **Construction site:** `vb_cli::run_compiled::InputMappingError`.
- **Fix site:** move decode-and-validate logic into
  `vb_runtime::shard::helpers::validate_input_mapping`, add a new
  `RuntimeError::InputMappingFailed` variant, add a new
  `CliExitCode::InputMappingFailed = 9`.
- **Acceptance test:**
  `submit_with_inputs_returns_input_mapping_failed_on_decode_error`.

### 6. `FOR_EACH_ITEM_FAILED`
- **Defect:** A for-each body step failure surfaces as the body's
  `CoreError` (e.g. `TypeMismatch` → `INPUT_TYPE_MISMATCH`); the
  iteration context (item index, body step) is lost.
- **Construction site:** `for_each_start` / `for_each_next` body
  dispatch in `crates/vb_runtime/src/primitives/for_each.rs`.
- **Fix site:** wrap body errors at the runtime layer with
  `RuntimeError::ForEachItemFailed { item_index, body_step, source }`.
- **Acceptance test:**
  `red_for_each_body_failure_surfaces_for_each_item_failed`.

### 7. `TOGETHER_BRANCH_FAILED`
- **Defect:** Mirror of item 6 for the `Together` family.
- **Construction site:** `together_branch` body dispatch in
  `crates/vb_runtime/src/primitives/together.rs`.
- **Fix site:** wrap body errors with
  `RuntimeError::TogetherBranchFailed { branch, entry, source }`.
- **Acceptance test:**
  `red_together_branch_failure_surfaces_together_branch_failed`.

### 8. `COLLECT_PAGE_FAILED`
- **Defect:** `CoreError::CollectPageOrderViolation` is currently
  bucketed under `COLLECT_LIMIT_REACHED`. Page-order violations are
  semantically distinct from quota limits.
- **Construction site:** `crates/vb_runtime/src/primitives/collect/state.rs:128,148,158`.
- **Fix site:** split the runtime_code arm in
  `crates/vb_core/src/errors.rs:711` so that
  `CollectPageOrderViolation` emits `COLLECT_PAGE_FAILED` and the
  three limit variants remain `COLLECT_LIMIT_REACHED`.
- **Acceptance test:**
  `core_error_collect_page_order_violation_emits_collect_page_failed_code`.

### 9. `REDUCE_ITEM_FAILED`
- **Defect:** Mirror of item 6 for the `Reduce` family.
- **Construction site:** `reduce_next` body dispatch in
  `crates/vb_runtime/src/primitives/reduce.rs`.
- **Fix site:** wrap body errors with
  `RuntimeError::ReduceItemFailed { item_index, body_step,
  accumulator_taint, source }`.
- **Acceptance test:**
  `red_reduce_body_failure_surfaces_reduce_item_failed`.

### 10. `RESULT_REFERENCE_MISSING`
- **Defect:** A `Finish { result: SlotIdx }` whose slot is
  unwritten (because a preceding step was skipped via an error
  handler) returns `CoreError::SlotUninitialized`, which has no
  runtime_code() arm at all.
- **Construction site:** `Finish` node dispatch in
  `crates/vb_core/src/engine/run_loop.rs`.
- **Fix site:** add `CoreError::ResultReferenceMissing { step,
  slot }` plus `RESULT_REFERENCE_MISSING_RUNTIME_CODE` and
  `RESULT_REFERENCE_MISSING_CODE = DiagnosticCode::new(0x1410)`.
- **Acceptance test:**
  `finish_with_unwritten_result_slot_emits_result_reference_missing`.

### 11. `RETRY_EXHAUSTED` (bonus)
- **Defect:** A `RetryCheck` whose policy has been exhausted at
  runtime returns `EngineError::StepBudgetExhausted`; the
  retry-exhaustion semantic is lost.
- **Construction site:** `execute_retry_check` in
  `crates/vb_core/src/engine/retry_math.rs`.
- **Fix site:** add `CoreError::RetryExhausted { step, attempts,
  max }` plus `RETRY_EXHAUSTED_RUNTIME_CODE` and
  `RETRY_EXHAUSTED_CODE = DiagnosticCode::new(0x1411)`.
- **Acceptance test:**
  `retry_exhausted_at_runtime_emits_retry_exhausted_code`.

## Test Infrastructure

The 11 items share a common test-infra rewrite that must land first:

- Delete `SECTION_17_UNMAPPED` constant and the
  `section17_reverse_parity_unmapped_codes_have_no_sources` test at
  `crates/workspace_tests/tests/section17_runtime_code_reverse_parity.rs:35-50`.
- Collapse
  `crates/workspace_tests/tests/section17_runtime_code_coverage_report.rs:159-217`
  to a single `MAPPED = SECTION_17_GOLDEN` test.
- Add `crates/workspace_tests/tests/section17_test_helpers.rs`
  exposing `assert_section17_code(source_kind, error, expected)`.

Acceptance: `rg "SECTION_17_UNMAPPED" crates/` returns zero matches
after the rewrite.

## Tracking

| Code                       | Status    | Owner | Bead ID  |
|----------------------------|-----------|-------|----------|
| `SECRET_UNAVAILABLE`       | Pending   | TBD   | vb-13d2a |
| `REPLAY_DIVERGED`          | Pending   | TBD   | vb-13d2b |
| `WAIT_TIMEOUT`             | Pending   | TBD   | vb-13d2c |
| `ASK_TIMEOUT`              | Pending   | TBD   | vb-13d2c |
| `STEP_SKIPPED_REFERENCE`   | Pending   | TBD   | vb-13d2d |
| `INPUT_MAPPING_FAILED`     | Pending   | TBD   | vb-13d2e |
| `FOR_EACH_ITEM_FAILED`     | Pending   | TBD   | vb-13d2f |
| `TOGETHER_BRANCH_FAILED`   | Pending   | TBD   | vb-13d2g |
| `COLLECT_PAGE_FAILED`      | Pending   | TBD   | vb-13d2h |
| `REDUCE_ITEM_FAILED`       | Pending   | TBD   | vb-13d2i |
| `RESULT_REFERENCE_MISSING` | Pending   | TBD   | vb-13d2j |
| `RETRY_EXHAUSTED`          | Pending   | TBD   | vb-13d2k |
| Test infrastructure        | Pending   | TBD   | vb-13d2z |
