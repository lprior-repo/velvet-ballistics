# P0-3r cli-resume-analysis

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> `crates/vb_cli/src/commands_journal.rs:386-428` (analyze_resume function).

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications (round-1 corrections applied):
- The bug is in `analyze_resume` at `commands_journal.rs:386-428` where `suspended_at_step` is `None` for journals without `WaitScheduledEvent` or `AskScheduledEvent`, but `can_resume: true` is still returned.
- The fix is to add a guard `if suspended_at_step.is_none()` BEFORE the terminal check, returning `can_resume: false` with a reason.
- `cmd_resume` IS already correctly wired at `run_ops.rs:150` (the round-1 "no-op" framing was fabricated; this is a fix-in-place).
- Master §33.3 lifecycle: resume is for suspended runs, not runs that have just started.

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL return `can_resume: false` from `analyze_resume` when the journal contains NO `WaitScheduledEvent` or `AskScheduledEvent`.
- THE SYSTEM SHALL preserve the existing `can_resume: true` behavior when the journal contains a `WaitScheduledEvent` or `AskScheduledEvent` (regression-protected).
- THE SYSTEM SHALL preserve the existing `can_resume: false` behavior for terminal journals (regression-protected).

### Event-Driven
- WHEN `analyze_resume` is called with a journal containing zero suspension events and a non-terminal terminal event, THE SYSTEM SHALL return `ResumeAnalysis { suspended_at_step: None, can_resume: false, reason: "run has no suspension events; nothing to resume" }`.
- WHEN `analyze_resume` is called with a journal containing `WaitScheduledEvent { step: 5 }` and no terminal event, THE SYSTEM SHALL return `ResumeAnalysis { suspended_at_step: Some(5), can_resume: true, .. }` (preserved).
- WHEN `analyze_resume` is called with a journal containing `AskScheduledEvent { step: 3 }` and no terminal event, THE SYSTEM SHALL return `ResumeAnalysis { suspended_at_step: Some(3), can_resume: true, .. }` (preserved).
- WHEN `analyze_resume` is called with a terminal journal (RunFinished/RunFailed/RunCancelled), THE SYSTEM SHALL return `can_resume: false` regardless of suspension events (preserved).

### Unwanted
- THE SYSTEM SHALL NOT return `can_resume: true` for journals with zero suspension events (the bug).
- THE SYSTEM SHALL NOT modify the `cmd_resume` wiring in `run_ops.rs:150` (it is already correct).
- THE SYSTEM SHALL NOT change the suspension scan logic (preserved).
- THE SYSTEM SHALL NOT panic on any input (empty journal, all-terminal, mixed events).

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs:
  - field: `events`
    type: `&[JournalEvent]`
    constraints: A borrowed slice of journal events; can be empty.
    example_valid: `&[StepSucceeded { ... }, WaitScheduledEvent { step: 5 }]`
    example_invalid: N/A.
- system_state:
  - `analyze_resume` is at `commands_journal.rs:386-428`.
  - `JournalEvent::WaitScheduledEvent` and `JournalEvent::AskScheduledEvent` are the two suspension indicators.
  - `cmd_resume` at `run_ops.rs:150` calls `analyze_resume` to decide whether to resume.

### Postconditions
- state_changes:
  - A new guard `if suspended_at_step.is_none()` is added BEFORE the terminal check.
  - The new branch returns `can_resume: false` with reason `"run has no suspension events; nothing to resume"`.
- return_guarantees:
  - field: `ResumeAnalysis.can_resume`
    guarantee: `true` iff (`suspended_at_step.is_some()` AND `!is_terminal`).
  - field: `ResumeAnalysis.reason`
    guarantee: Non-empty for all non-success cases; documents why resume is or is not possible.
- side_effects: None. `analyze_resume` is a pure function.

### Invariants
- For any journal, `can_resume` is true iff `suspended_at_step.is_some() AND !is_terminal` (the truth table invariant).
- The truth table {no_suspension, terminal, no_suspension_and_terminal, suspension_and_non_terminal} is exhaustively tested.
- The terminal check is preserved (RunFinished/RunFailed/RunCancelled all return `can_resume: false`).

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: `crates/vb_cli/src/commands_journal.rs:386-428`
  what_to_extract: The full `analyze_resume` function body. Confirm the bug: `suspended_at_step` is `None` when no WaitScheduled/AskScheduled events exist, but `can_resume: true` is returned.
  document_in: research_notes.md
- path: `crates/vb_cli/src/commands_journal.rs:375-381`
  what_to_extract: The `ResumeAnalysis` struct fields. Confirm `suspended_at_step, can_resume, reason`.
  document_in: research_notes.md
- path: `crates/vb_cli/src/run_ops.rs:150`
  what_to_extract: The `cmd_resume` entry point. Confirm it calls `lifecycle::resume` and surfaces `analyze_resume`'s reason field.
  document_in: research_notes.md
- path: `crates/vb_cli/src/lifecycle.rs`
  what_to_extract: The `resume` function. Verify the call site.
  document_in: research_notes.md
- path: `crates/vb_core/src/journal/events.rs`
  what_to_extract: The `JournalEvent` enum. Confirm `WaitScheduledEvent` and `AskScheduledEvent` are distinct variants.
  document_in: research_notes.md

Patterns to find:
- pattern: `WaitScheduledEvent`
  purpose: Confirm the variant exists.
  expected_locations: `crates/vb_core/src/journal/events.rs`.
- pattern: `suspended_at_step`
  purpose: Locate the variable that is checked.
  expected_locations: `crates/vb_cli/src/commands_journal.rs:388-399`.

Prior art:
- feature: existing `analyze_retry` function (sibling)
  location: `crates/vb_cli/src/commands_journal.rs:329-369`
  what_to_learn: The pattern of an analyzer function with a binary decision. Apply the same shape to `analyze_resume`.

External docs:
- url: master doc §33.3
  section: CLI lifecycle
  extract: confirm "resume is for suspended runs, not runs that have just started".

Research questions (all answered):
- Q: Is the bug in `analyze_resume` or in `cmd_resume`? A: In `analyze_resume` (line 386-428). `cmd_resume` is already correct.
- Q: What is the new reason string? A: `"run has no suspension events; nothing to resume"`.
- Q: If both WaitScheduled and AskScheduled exist, which step wins? A: Last one wins (the current behavior is preserved).

Research complete when:
- [x] All files_to_read opened.
- [x] All patterns_to_find searched.
- [x] All prior_art examined.
- [x] All research_questions have answers.

## Section 3. Inversions

### Security
- failure: An attacker resumes a completed run, replaying the action and bypassing the audit trail.
  prevention: `analyze_resume` rejects both terminal events AND missing suspension events.
  test_for_it: `completed_run_cannot_be_resumed_via_analyze_resume: proptest with 1000 random terminal+resume sequences; assert can_resume=false on all terminals`.

### Usability
- failure: Operator resumes a fresh run with no suspension; resume silently treats the run as suspended and the IPC channel fails with a confusing "no pending action" error.
  prevention: `analyze_resume` returns `can_resume: false` when no suspension events exist; `cmd_resume` surfaces a typed CLI error.
  test_for_it: `cli_resume_on_fresh_run_returns_validation_failed: vb resume <fresh-run-id> exits with code 3 and message "no suspension events"`.

### Data Integrity
- failure: The fix accidentally changes the behavior for terminal journals (regression).
  prevention: The new guard is BEFORE the terminal check but does NOT replace it. Terminal journals still return `can_resume: false` with the terminal reason.
  test_for_it: `test_analyze_resume_on_terminal_journal_returns_can_resume_false: journal = [..., RunFinished] -> can_resume: false, reason: "run is finished, not suspended"`.

### Integration Failure
- failure: A downstream tool parses `ResumeAnalysis.reason` and assumes it's empty for suspension cases. The fix accidentally adds a reason for successful resumes.
  prevention: The new branch is ONLY for `suspended_at_step.is_none()`; the success branch keeps `reason: String::new()`.
  test_for_it: `test_suspended_run_has_empty_reason: journal = [..., WaitScheduledEvent] -> reason: ""`.

## Section 4. ATDD Tests

### Happy
- name: `test_analyze_resume_on_suspended_run_returns_can_resume_true`
  given: A journal with `[StepSucceeded, WaitScheduledEvent { step: 5 }]`.
  when: `analyze_resume` is called.
  then: Returns `ResumeAnalysis { suspended_at_step: Some(5), can_resume: true, reason: "" }` (regression).
  real_input: 2 events.
  expected_output: `can_resume: true`, `suspended_at_step: Some(5)`, `reason: ""`.
- name: `test_analyze_resume_on_ask_suspended_run_returns_can_resume_true`
  given: A journal with `[StepSucceeded, AskScheduledEvent { step: 3 }]`.
  when: `analyze_resume` is called.
  then: Returns `ResumeAnalysis { suspended_at_step: Some(3), can_resume: true, reason: "" }` (regression).
  real_input: 2 events.
  expected_output: `can_resume: true`, `suspended_at_step: Some(3)`.

### Error
- name: `test_analyze_resume_on_run_with_no_suspension_events_returns_can_resume_false`
  given: A journal with `[StepSucceeded, StepSucceeded]` (no suspension events).
  when: `analyze_resume` is called.
  then: Returns `ResumeAnalysis { suspended_at_step: None, can_resume: false, reason: "run has no suspension events; nothing to resume" }` (NEW FIX).
  real_input: 2 events.
  expected_output: `can_resume: false`, `reason: <no-suspension message>`.
- name: `test_analyze_resume_on_empty_journal_returns_can_resume_false`
  given: An empty journal.
  when: `analyze_resume` is called.
  then: Returns `can_resume: false` with the no-suspension reason (NEW FIX).
  real_input: `&[]`.
  expected_output: `can_resume: false`.

### Edge
- name: `test_analyze_resume_on_terminal_finished_run_returns_can_resume_false`
  given: A journal ending in `RunFinished`.
  when: `analyze_resume` is called.
  then: Returns `can_resume: false` with the terminal reason (regression).
  real_input: `[..., RunFinished]`.
  expected_output: `can_resume: false`, `reason: <finished message>`.
- name: `test_analyze_resume_with_both_suspension_kinds_uses_last`
  given: A journal with `[WaitScheduledEvent { step: 5 }, AskScheduledEvent { step: 7 }]`.
  when: `analyze_resume` is called.
  then: Returns `suspended_at_step: Some(7)` (the last one wins; preserved).
  real_input: 2 events.
  expected_output: `suspended_at_step: Some(7)`, `can_resume: true`.

### Contract
- name: `test_precondition_journal_can_be_empty`
  verifies: Precondition "events slice can be empty".
  test: `analyze_resume(&[])` returns `can_resume: false`.
- name: `test_postcondition_can_resume_iff_suspended_and_non_terminal`
  verifies: Postcondition "can_resume is true iff suspended_at_step.is_some() AND !is_terminal".
  test: proptest with 100 random journal sequences; assert the truth table.
- name: `test_invariant_terminal_journal_branch_preserved`
  verifies: Invariant "terminal branch is unchanged".
  test: assert `can_resume: false` and the terminal reason for RunFinished/RunFailed/RunCancelled.

## Section 5. E2E Tests

```
pipeline_test:
  name: test_cli_resume_e2e
  description: Real CLI invocation, real journal, real resume logic.
  setup:
    - create a journal with no suspension events
  execute:
    command: "moon run -- vb resume <fresh-run-id>"
    timeout_ms: 5000
  verify:
    - exit_code: 3 (invalid input)
    - stderr_contains: "no suspension events"
  cleanup:
    - delete the journal

e2e_scenarios:
  - name: e2e_resume_fresh_run_rejected
    description: prove the CLI rejects resume on a fresh run
    steps:
      - submit a run (no suspension)
      - run `vb resume <id>`
      - assert exit code 3 and error message
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] `commands_journal.rs:386-428` read (bug location confirmed at 388-428)"
    - "[x] `commands_journal.rs:375-381` read (ResumeAnalysis struct)"
    - "[x] `run_ops.rs:150` read (cmd_resume wiring; already correct)"
    - "[x] `lifecycle.rs` read (resume function)"
    - "[x] `journal/events.rs` read (WaitScheduledEvent + AskScheduledEvent)"
  evidence_required:
    - "Research notes file with line-numbered extracts"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All 7 acceptance tests written (2 happy, 2 error, 2 edge, 3 contract)"
    - "[ ] Tests fail with the expected assertion error (can_resume is currently true for no-suspension journals)"
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
- [ ] Read `commands_journal.rs:386-428` (parallel: research)
- [ ] Read `commands_journal.rs:375-381` (parallel: research)
- [ ] Read `run_ops.rs:150` (parallel: research)
- [ ] Read `lifecycle.rs` (parallel: research)
- [ ] Read `journal/events.rs` (parallel: research)

### Phase 1: Tests
- [ ] Write `test_analyze_resume_on_suspended_run_returns_can_resume_true` (parallel: tests)
- [ ] Write `test_analyze_resume_on_ask_suspended_run_returns_can_resume_true` (parallel: tests)
- [ ] Write `test_analyze_resume_on_run_with_no_suspension_events_returns_can_resume_false` (parallel: tests)
- [ ] Write `test_analyze_resume_on_empty_journal_returns_can_resume_false` (parallel: tests)
- [ ] Write `test_analyze_resume_on_terminal_finished_run_returns_can_resume_false` (parallel: tests)
- [ ] Write `test_analyze_resume_with_both_suspension_kinds_uses_last` (parallel: tests)
- [ ] Write 3 contract tests (parallel: tests)
- [ ] Confirm all 7 tests fail (gate)

### Phase 2: Implementation
- [ ] In `commands_journal.rs:analyze_resume`: AFTER the suspension scan loop, BEFORE the terminal check, add a guard `if suspended_at_step.is_none() { return ResumeAnalysis { suspended_at_step: None, can_resume: false, reason: "run has no suspension events; nothing to resume".into() }; }` (depends: tests; sequential)
- [ ] Confirm all 7 tests pass (gate: green)

### Phase 3: Integration
- [ ] Write the E2E test (depends: impl; sequential)
- [ ] Run the E2E test (sequential)
- [ ] Run `cargo test -p vb_cli` to confirm no regressions (sequential)

### Phase 4: Documentation
- [ ] Run `moon run :ci` (depends: all of the above; parallel)
- [ ] Close the bead (sequential)

## Section 7. Failure Modes

- symptom: "Test fails: no-suspension journal still returns can_resume: true"
  likely_cause: The new guard was not added.
  where_to_look:
    - file: `crates/vb_cli/src/commands_journal.rs:386-428`
    - what_to_check: "Is the `if suspended_at_step.is_none() { return ...; }` guard present?"
  fix_pattern: Add the guard after the suspension scan loop.
- symptom: "Test fails: terminal journal returns can_resume: true (regression)"
  likely_cause: The new guard accidentally short-circuits the terminal check.
  where_to_look:
    - file: `crates/vb_cli/src/commands_journal.rs:386-428`
    - what_to_check: "Is the new guard BEFORE the terminal check (not replacing it)?"
  fix_pattern: Move the new guard to be BEFORE the terminal check.
- symptom: "Test fails: suspended run returns can_resume: false (regression)"
  likely_cause: The new guard has the wrong condition (e.g., `is_some()` instead of `is_none()`).
  where_to_look:
    - file: `crates/vb_cli/src/commands_journal.rs:386-428`
    - what_to_check: "Is the condition `suspended_at_step.is_none()`?"
  fix_pattern: Use `is_none()` (not `is_some()`).

debugging_commands:
- scenario: "When can_resume is still true for no-suspension journals"
  run: "rg 'suspended_at_step.is_none' crates/vb_cli/src/commands_journal.rs"
  look_for: "The new guard should be present at line ~400"

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT modify `cmd_resume` in `run_ops.rs:150` (it is already correctly wired).
- DO NOT change the suspension scan logic.
- DO NOT remove the terminal check; the new guard is ADDITIVE.
- DO NOT use `unwrap()` or `expect()` in new code.

VERIFY that:
- `analyze_resume` is at `commands_journal.rs:386-428`: `rg 'pub fn analyze_resume' crates/vb_cli/src/commands_journal.rs` (must return 1 match).
- `cmd_resume` is at `run_ops.rs:150`: `rg 'fn cmd_resume' crates/vb_cli/src/run_ops.rs` (must return 1 match).
- `JournalEvent::WaitScheduledEvent` exists: `rg 'WaitScheduledEvent' crates/vb_core/src/journal/events.rs` (must return at least 1 match).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    rg 'suspended_at_step.is_none' crates/vb_cli/src/commands_journal.rs  # confirm the new guard is wired

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-ujho9/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-ujho9/progress.txt` and continue from "Current Task". The fix is in `analyze_resume` only; `cmd_resume` is unchanged.
Key invariants:
- The fix is in `analyze_resume` at `commands_journal.rs:386-428`.
- `cmd_resume` is UNCHANGED.
- The new guard is `if suspended_at_step.is_none() { return ...; }` BEFORE the terminal check.
- The new reason string is `"run has no suspension events; nothing to resume"`.
- The terminal check is preserved (RunFinished/RunFailed/RunCancelled all return `can_resume: false`).

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
- [ ] bd close with reason: "P0-3r complete: analyze_resume now rejects no-suspension journals"

## Section 9. Context

Related files:
- `crates/vb_cli/src/commands_journal.rs:375-381` — `ResumeAnalysis` struct
- `crates/vb_cli/src/commands_journal.rs:386-428` — `analyze_resume` (the function to fix)
- `crates/vb_cli/src/run_ops.rs:150` — `cmd_resume` (already correct)
- `crates/vb_cli/src/lifecycle.rs` — `resume` function
- `crates/vb_core/src/journal/events.rs` — `JournalEvent` enum (WaitScheduledEvent + AskScheduledEvent)
- master doc §33.3 — CLI lifecycle (resume is for suspended runs)

Similar implementations:
- `analyze_retry` at `commands_journal.rs:329-369` is a sibling function with similar shape. The fix in `analyze_resume` follows the same pattern.

Codebase patterns:
- pattern: "Pre-terminal-check guard"
  example_location: `crates/vb_cli/src/commands_journal.rs:386-428` (current `analyze_resume`)
  how_to_apply: Add a guard BEFORE the terminal check; if the pre-condition is not met, return early with `can_resume: false` and a reason.

## Section 10. AI Hints

### DO
- Read `crates/vb_cli/src/commands_journal.rs:386-428` BEFORE writing any code. The function is 43 lines; the read is fast.
- Add the new guard AFTER the suspension scan loop, BEFORE the terminal check.
- Use `suspended_at_step.is_none()` as the condition.
- Use the documented reason string.
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT use `unwrap()` or `expect()`.
- Do NOT modify `cmd_resume` in `run_ops.rs:150`.
- Do NOT change the suspension scan logic.
- Do NOT remove the terminal check.
- Do NOT use `unsafe`.

### Code patterns
- name: "Pre-terminal-check guard"
  use_when: "Adding an early return for a missing pre-condition"
  example: |
    // After suspension scan loop, before terminal check:
    if suspended_at_step.is_none() {
        return ResumeAnalysis {
            suspended_at_step: None,
            can_resume: false,
            reason: "run has no suspension events; nothing to resume".into(),
        };
    }
    // Terminal check follows unchanged.

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect().
- Test first: Tests MUST exist and FAIL before implementation.
- Read before write: ALWAYS read the source file before modifying it.
- Real data only: Use real JournalEvent variants; no fabricated placeholders.
- Minimal change: ONE function to fix; do NOT refactor the CLI.
