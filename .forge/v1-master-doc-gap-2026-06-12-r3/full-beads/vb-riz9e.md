# P0-2r cli-retry-analysis

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> `crates/vb_cli/src/commands_journal.rs:329-369` (analyze_retry function).

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications (round-1 corrections applied):
- The bug is in `analyze_retry` at `commands_journal.rs:351-353` where `is_failed` matches BOTH `RunFailedEvent` and `RunCancelled`, returning `can_retry: true` for both.
- The fix is to split the check: `is_failed` matches ONLY `RunFailedEvent`; a new `is_cancelled` matches `RunCancelled` and returns `can_retry: false` with a reason.
- `cmd_retry` IS already correctly wired to `lifecycle::retry` at `run_ops.rs:25` (the round-1 "no-op" framing was fabricated; this is a fix-in-place).
- Master §33.3 lifecycle: retry is for failed runs; cancelled runs require explicit submit.

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL return `can_retry: false` from `analyze_retry` when the journal's terminal event is `RunCancelled`.
- THE SYSTEM SHALL preserve the existing `can_retry: true` behavior for `RunFailedEvent` (regression-protected).
- THE SYSTEM SHALL preserve the existing `can_retry: false` behavior for `RunFinished` and empty/non-terminal journals (regression-protected).

### Event-Driven
- WHEN `analyze_retry` is called with a journal ending in `RunCancelled`, THE SYSTEM SHALL return `RetryAnalysis { can_retry: false, reason: "run was cancelled, not failed; use submit to restart", .. }`.
- WHEN `analyze_retry` is called with a journal ending in `RunFailedEvent`, THE SYSTEM SHALL return `RetryAnalysis { can_retry: true, failed_at_step: Some(step), .. }` (preserved).
- WHEN `analyze_retry` is called with a journal ending in `RunFinished` or empty, THE SYSTEM SHALL return `RetryAnalysis { can_retry: false, reason: "run did not fail (no retry needed)", .. }` (preserved).

### Unwanted
- THE SYSTEM SHALL NOT return `can_retry: true` for `RunCancelled` journals (the bug).
- THE SYSTEM SHALL NOT modify the `cmd_retry` wiring in `run_ops.rs:25` (it is already correct).
- THE SYSTEM SHALL NOT change the `failure_step` calculation logic (preserved).
- THE SYSTEM SHALL NOT panic on any input (empty journal, all-terminal, mixed events).

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs:
  - field: `events`
    type: `&[JournalEvent]`
    constraints: A borrowed slice of journal events; can be empty.
    example_valid: `&[StepSucceeded { ... }, RunFailedEvent { ... }]`
    example_invalid: N/A (the slice is borrowed; no ownership constraints).
- system_state:
  - `analyze_retry` is at `commands_journal.rs:329-369`.
  - `JournalEvent::RunCancelled` and `JournalEvent::RunFailedEvent` are distinct variants.
  - `cmd_retry` at `run_ops.rs:25` calls `analyze_retry` to decide whether to retry.

### Postconditions
- state_changes:
  - The `is_failed` check is split into `is_failed` (matches only `RunFailedEvent`) and `is_cancelled` (matches `RunCancelled`).
  - A new branch returns `can_retry: false` with reason `"run was cancelled, not failed; use submit to restart"`.
- return_guarantees:
  - field: `RetryAnalysis.can_retry`
    guarantee: `true` iff the journal's terminal event is `RunFailedEvent`; `false` otherwise.
  - field: `RetryAnalysis.reason`
    guarantee: Non-empty for all non-success cases; documents why retry is or is not possible.
- side_effects: None. `analyze_retry` is a pure function.

### Invariants
- For any journal whose terminal event is `RunFinished`, `analyze_retry` returns `can_retry: false` with reason `"run did not fail (no retry needed)"` (regression-protected).
- For any journal whose terminal event is `RunFailedEvent`, `analyze_retry` returns `can_retry: true` (regression-protected).
- For any journal whose terminal event is `RunCancelled`, `analyze_retry` returns `can_retry: false` with reason `"run was cancelled, not failed; use submit to restart"` (new fix).
- The `failure_step` calculation (`failed_step.or(last_successful_step.map(|s| s.saturating_add(1)))`) is preserved.

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: `crates/vb_cli/src/commands_journal.rs:329-369`
  what_to_extract: The full `analyze_retry` function body. Confirm the `is_failed` check at line 347-350 matches BOTH `RunFailedEvent` AND `RunCancelled` (the bug).
  document_in: research_notes.md
- path: `crates/vb_cli/src/commands_journal.rs:314-324`
  what_to_extract: The `RetryAnalysis` struct fields. Confirm `failed_at_step, last_successful_step, can_retry, reason`.
  document_in: research_notes.md
- path: `crates/vb_cli/src/run_ops.rs:25`
  what_to_extract: The `cmd_retry` entry point. Confirm it calls `lifecycle::retry` and surfaces `analyze_retry`'s reason field.
  document_in: research_notes.md
- path: `crates/vb_cli/src/lifecycle.rs`
  what_to_extract: The `retry` function. Verify the call site.
  document_in: research_notes.md
- path: `crates/vb_core/src/journal/events.rs`
  what_to_extract: The `JournalEvent` enum. Confirm `RunCancelled` and `RunFailedEvent` are distinct variants.
  document_in: research_notes.md

Patterns to find:
- pattern: `is_failed`
  purpose: Locate the buggy check at line 347-350.
  expected_locations: `crates/vb_cli/src/commands_journal.rs:347-350`.
- pattern: `RunCancelled`
  purpose: Confirm the variant exists.
  expected_locations: `crates/vb_core/src/journal/events.rs`.

Prior art:
- feature: existing `analyze_resume` function (related but separate)
  location: `crates/vb_cli/src/commands_journal.rs:386-428`
  what_to_learn: The pattern of an analyzer function with a binary decision. Apply the same shape to `analyze_retry`.

External docs:
- url: master doc §33.3
  section: CLI lifecycle
  extract: confirm "retry is for failed runs; cancelled runs require explicit submit".

Research questions (all answered):
- Q: Is the bug in `analyze_retry` or in `cmd_retry`? A: In `analyze_retry` (line 351-353). `cmd_retry` is already correct.
- Q: What is the new reason string? A: `"run was cancelled, not failed; use submit to restart"`.
- Q: Does the `failure_step` calculation change? A: No, it is preserved.

Research complete when:
- [x] All files_to_read opened.
- [x] All patterns_to_find searched.
- [x] All prior_art examined.
- [x] All research_questions have answers.

## Section 3. Inversions

### Security
- failure: An attacker cancels a run, then retries it via `vb retry <cancelled-id>`, defeating the cancel signal in audit logs.
  prevention: `analyze_retry` explicitly rejects `RunCancelled`; `cmd_retry` surfaces a typed CLI error.
  test_for_it: `cancelled_run_cannot_be_retried_via_analyze_retry: proptest with 1000 random cancel-then-retry sequences; assert can_retry=false on all`.

### Usability
- failure: Operator retries a cancelled run; the retry silently re-schedules from the last successful step and overwrites the cancel marker.
  prevention: `analyze_retry` returns `can_retry: false` for `RunCancelled`; `cmd_retry` surfaces the reason via a typed CLI error.
  test_for_it: `cli_retry_on_cancelled_run_returns_validation_failed: vb retry <cancelled-id> exits with code 3 and message "cancelled, use submit"`.

### Data Integrity
- failure: The fix accidentally changes the behavior for `RunFailedEvent` (regression).
  prevention: The `is_failed` check is split, not removed. `RunFailedEvent` is still matched (now in `is_failed` only).
  test_for_it: `test_analyze_retry_on_failed_run_returns_can_retry_true: journal = [StepSucceeded, RunFailedEvent] -> can_retry: true (regression)`.

### Integration Failure
- failure: A downstream tool parses `RetryAnalysis.reason` and assumes it's empty for `RunFailedEvent` (the old behavior). The fix accidentally adds a reason for `RunFailedEvent`.
  prevention: The new branch is ONLY for `RunCancelled`; the `RunFailedEvent` branch keeps `reason: String::new()`.
  test_for_it: `test_failed_event_has_empty_reason: journal = [..., RunFailedEvent] -> reason == ""`.

## Section 4. ATDD Tests

### Happy
- name: `test_analyze_retry_on_failed_run_returns_can_retry_true`
  given: A journal with `[StepSucceeded { step: 0 }, ActionFailedEvent { step: 1 }, RunFailedEvent { ... }]`.
  when: `analyze_retry` is called.
  then: Returns `RetryAnalysis { can_retry: true, failed_at_step: Some(1), last_successful_step: Some(0), reason: "" }`.
  real_input: 3 events (StepSucceeded, ActionFailedEvent, RunFailedEvent).
  expected_output: `can_retry: true`, `failed_at_step: Some(1)`, `reason: ""`.
- name: `test_analyze_retry_on_cancelled_run_returns_can_retry_false`
  given: A journal with `[StepSucceeded { step: 0 }, RunCancelled { ... }]`.
  when: `analyze_retry` is called.
  then: Returns `RetryAnalysis { can_retry: false, reason: "run was cancelled, not failed; use submit to restart", .. }`.
  real_input: 2 events.
  expected_output: `can_retry: false`, `reason: <cancellation message>`.

### Error
- name: `test_analyze_retry_on_empty_journal_returns_can_retry_false`
  given: An empty journal.
  when: `analyze_retry` is called.
  then: Returns `RetryAnalysis { can_retry: false, reason: "run did not fail (no retry needed)", .. }` (regression-protected).
  real_input: `&[]`.
  expected_output: `can_retry: false`, `reason: <no-retry-needed message>`.
- name: `test_analyze_retry_on_finished_run_returns_can_retry_false`
  given: A journal ending in `RunFinished`.
  when: `analyze_retry` is called.
  then: Returns `can_retry: false` with the no-retry-needed reason (regression-protected).
  real_input: `[..., RunFinished]`.
  expected_output: `can_retry: false`.

### Edge
- name: `test_analyze_retry_on_cancelled_run_with_no_successful_steps`
  given: A journal with `[RunCancelled]` (no preceding steps).
  when: `analyze_retry` is called.
  then: Returns `can_retry: false` with the cancellation reason; `failed_at_step: None`; `last_successful_step: None`.
  real_input: 1 event.
  expected_output: `can_retry: false`, `failed_at_step: None`.
- name: `test_analyze_retry_on_non_terminal_journal_returns_can_retry_false`
  given: A journal with `[StepSucceeded, StepSucceeded]` (no terminal event).
  when: `analyze_retry` is called.
  then: Returns `can_retry: false` with the no-retry-needed reason (the non-terminal check is preserved).
  real_input: 2 events.
  expected_output: `can_retry: false`.

### Contract
- name: `test_precondition_journal_can_be_empty`
  verifies: Precondition "events slice can be empty".
  test: `analyze_retry(&[])` returns `can_retry: false`.
- name: `test_postcondition_can_retry_iff_run_failed_event`
  verifies: Postcondition "can_retry is true iff terminal is RunFailedEvent".
  test: proptest with 100 random journal sequences covering all 4 terminal variants.
- name: `test_invariant_failed_event_branch_preserved`
  verifies: Invariant "RunFailedEvent branch is unchanged".
  test: assert `can_retry: true` and `reason: ""` for `RunFailedEvent` journals.

## Section 5. E2E Tests

```
pipeline_test:
  name: test_cli_retry_e2e
  description: Real CLI invocation, real journal, real retry logic.
  setup:
    - create a journal with a RunCancelled terminal event
  execute:
    command: "moon run -- vb retry <cancelled-run-id>"
    timeout_ms: 5000
  verify:
    - exit_code: 3 (invalid input)
    - stderr_contains: "cancelled, use submit"
  cleanup:
    - delete the journal

e2e_scenarios:
  - name: e2e_retry_cancelled_run_rejected
    description: prove the CLI rejects retry on a cancelled run
    steps:
      - submit a run
      - cancel the run
      - run `vb retry <id>`
      - assert exit code 3 and error message
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] `commands_journal.rs:329-369` read (bug location confirmed at 351-353)"
    - "[x] `commands_journal.rs:314-324` read (RetryAnalysis struct)"
    - "[x] `run_ops.rs:25` read (cmd_retry wiring; already correct)"
    - "[x] `lifecycle.rs` read (retry function)"
    - "[x] `journal/events.rs` read (RunCancelled + RunFailedEvent are distinct)"
  evidence_required:
    - "Research notes file with line-numbered extracts"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All 7 acceptance tests written (2 happy, 2 error, 2 edge, 3 contract)"
    - "[ ] Tests fail with the expected assertion error (can_retry is currently true for RunCancelled)"
  evidence_required:
    - "Test file in `crates/vb_cli/src/commands_journal/tests.rs`"
    - "Test output showing the bug"

### Implementation
- name: "Implementation Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] All 7 tests pass"
    - "[ ] No unwrap() or expect() in new code"
    - "[ ] moon run :ci passes"
  evidence_required:
    - "Test output"
    - "CI green"

### Integration
- name: "Integration Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] E2E test passes with real CLI"
    - "[ ] No regressions in CLI tests"
  evidence_required:
    - "E2E output"
    - "Manual verification"

## Section 6. Implementation Tasks

### Phase 0: Research
- [ ] Read `commands_journal.rs:329-369` (parallel: research)
- [ ] Read `commands_journal.rs:314-324` (parallel: research)
- [ ] Read `run_ops.rs:25` (parallel: research)
- [ ] Read `lifecycle.rs` (parallel: research)
- [ ] Read `journal/events.rs` (parallel: research)

### Phase 1: Tests
- [ ] Write `test_analyze_retry_on_failed_run_returns_can_retry_true` (parallel: tests)
- [ ] Write `test_analyze_retry_on_cancelled_run_returns_can_retry_false` (parallel: tests)
- [ ] Write `test_analyze_retry_on_empty_journal_returns_can_retry_false` (parallel: tests)
- [ ] Write `test_analyze_retry_on_finished_run_returns_can_retry_false` (parallel: tests)
- [ ] Write `test_analyze_retry_on_cancelled_run_with_no_successful_steps` (parallel: tests)
- [ ] Write `test_analyze_retry_on_non_terminal_journal_returns_can_retry_false` (parallel: tests)
- [ ] Write 3 contract tests (parallel: tests)
- [ ] Confirm all 7 tests fail (gate)

### Phase 2: Implementation
- [ ] In `commands_journal.rs:347-350`: split the `is_failed` check. New `is_failed` matches only `RunFailedEvent`. Add `is_cancelled` matching `RunCancelled` (depends: tests; sequential)
- [ ] Add a new branch that returns `can_retry: false` with reason `"run was cancelled, not failed; use submit to restart"` when `is_cancelled` is true (depends: check; sequential)
- [ ] Preserve the `failure_step` calculation (depends: branches; sequential)
- [ ] Confirm all 7 tests pass (gate: green)

### Phase 3: Integration
- [ ] Write the E2E test (depends: impl; sequential)
- [ ] Run the E2E test (sequential)
- [ ] Run `cargo test -p vb_cli` to confirm no regressions (sequential)

### Phase 4: Documentation
- [ ] Run `moon run :ci` (depends: all of the above; parallel)
- [ ] Close the bead (sequential)

## Section 7. Failure Modes

- symptom: "Test fails: cancelled run still returns can_retry: true"
  likely_cause: The split was not applied. The old `is_failed` check still matches `RunCancelled`.
  where_to_look:
    - file: `crates/vb_cli/src/commands_journal.rs:347-369`
    - what_to_check: "Is `is_failed` matching ONLY `RunFailedEvent`? Is there a separate `is_cancelled` check?"
  fix_pattern: Split the `matches!` arm.
- symptom: "Test fails: failed run returns can_retry: false (regression)"
  likely_cause: The new `is_cancelled` branch accidentally catches `RunFailedEvent`.
  where_to_look:
    - file: `crates/vb_cli/src/commands_journal.rs:347-369`
    - what_to_check: "Is the `is_cancelled` matches! arm only for `RunCancelled`?"
  fix_pattern: Confirm the matches! arm.
- symptom: "Test fails: failure_step calculation is wrong"
  likely_cause: The `failure_step.or(...)` line was accidentally changed.
  where_to_look:
    - file: `crates/vb_cli/src/commands_journal.rs:361`
    - what_to_check: "Is the line `let failure_step = failed_step.or(last_successful_step.map(|s| s.saturating_add(1)));` preserved?"
  fix_pattern: Restore the line.

debugging_commands:
- scenario: "When can_retry is still true for RunCancelled"
  run: "rg 'is_failed|is_cancelled' crates/vb_cli/src/commands_journal.rs"
  look_for: "Confirm the split: is_failed matches RunFailedEvent only, is_cancelled matches RunCancelled only"

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT modify `cmd_retry` in `run_ops.rs:25` (it is already correctly wired).
- DO NOT change the `failure_step` calculation.
- DO NOT remove the `is_failed` check; split it.
- DO NOT use `unwrap()` or `expect()` in new code.

VERIFY that:
- `analyze_retry` is at `commands_journal.rs:329-369`: `rg 'pub fn analyze_retry' crates/vb_cli/src/commands_journal.rs` (must return 1 match).
- `cmd_retry` is at `run_ops.rs:25`: `rg 'fn cmd_retry' crates/vb_cli/src/run_ops.rs` (must return 1 match).
- `JournalEvent::RunCancelled` exists: `rg 'RunCancelled' crates/vb_core/src/journal/events.rs` (must return at least 1 match).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    rg 'is_cancelled' crates/vb_cli/src/commands_journal.rs  # confirm the new check is wired

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-riz9e/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-riz9e/progress.txt` and continue from "Current Task". The fix is in `analyze_retry` only; `cmd_retry` is unchanged.
Key invariants:
- The fix is in `analyze_retry` at `commands_journal.rs:347-369`.
- `cmd_retry` is UNCHANGED.
- `is_failed` matches ONLY `RunFailedEvent`; `is_cancelled` matches ONLY `RunCancelled`.
- The new reason string is `"run was cancelled, not failed; use submit to restart"`.
- The `failure_step` calculation is preserved.

## Section 8. Completion Checklist

- [ ] All 7 acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing with real CLI
- [ ] No mocks or fake data in any test
- [ ] moon run :ci passes
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] No production code touched outside `crates/vb_cli/src/commands_journal.rs`
- [ ] bd remember note: "Round 3 black-hat APPROVED. 16-section content generated from source read."
- [ ] bd close with reason: "P0-2r complete: analyze_retry now rejects RunCancelled"

## Section 9. Context

Related files:
- `crates/vb_cli/src/commands_journal.rs:314-324` — `RetryAnalysis` struct
- `crates/vb_cli/src/commands_journal.rs:329-369` — `analyze_retry` (the function to fix)
- `crates/vb_cli/src/run_ops.rs:25` — `cmd_retry` (already correct)
- `crates/vb_cli/src/lifecycle.rs` — `retry` function
- `crates/vb_core/src/journal/events.rs` — `JournalEvent` enum (RunCancelled + RunFailedEvent)
- master doc §33.3 — CLI lifecycle (retry is for failed runs)

Similar implementations:
- `analyze_resume` at `commands_journal.rs:386-428` is a sibling function with similar shape. The fix in `analyze_retry` follows the same pattern (split the terminal check).

Codebase patterns:
- pattern: "Terminal event check + branch"
  example_location: `crates/vb_cli/src/commands_journal.rs:386-428` (analyze_resume)
  how_to_apply: Match on the terminal event; for each variant, return a typed analysis.

## Section 10. AI Hints

### DO
- Read `crates/vb_cli/src/commands_journal.rs:329-369` BEFORE writing any code. The function is 40 lines; the read is fast.
- Split the `is_failed` check; do NOT remove it.
- Add a new `is_cancelled` check that returns `can_retry: false` with the documented reason.
- Preserve the `failure_step` calculation exactly.
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT use `unwrap()` or `expect()`.
- Do NOT modify `cmd_retry` in `run_ops.rs:25`.
- Do NOT change the `failure_step` calculation.
- Do NOT use `unsafe`.

### Code patterns
- name: "Split a matches! arm into two"
  use_when: "Distinguishing two previously-conflated terminal events"
  example: |
    // Before (buggy):
    let is_failed = matches!(terminal, Some(RunFailedEvent { .. }) | Some(RunCancelled { .. }));
    // After (fixed):
    let is_failed = matches!(terminal, Some(RunFailedEvent { .. }));
    let is_cancelled = matches!(terminal, Some(RunCancelled { .. }));
    if is_cancelled { return RetryAnalysis { can_retry: false, reason: "cancelled, not failed; use submit".into(), .. }; }

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect().
- Test first: Tests MUST exist and FAIL before implementation.
- Read before write: ALWAYS read the source file before modifying it.
- Real data only: Use real JournalEvent variants; no fabricated placeholders.
- Minimal change: ONE function to fix; do NOT refactor the CLI.
