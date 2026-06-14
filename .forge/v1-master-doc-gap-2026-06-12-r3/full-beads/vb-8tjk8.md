# P2-18r snapshot-writer

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> `crates/vb_runtime/src/shard/config.rs:27-38` (ShardConfig struct),
> `crates/vb_runtime/src/engine/drive.rs:46-75` (drive_deterministic_full function).

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications:
- The `ShardConfig` struct at `config.rs:27-38` has 5 fields. The snapshot-writer needs a new field to control when snapshots are written.
- The `drive_deterministic_full` function at `drive.rs:46-75` is the main drive loop. The snapshot should be written after N steps (where N is configurable).
- The title "Runtime writes RunSnapshot after N steps (≥3× throughput on submit + 100 actions)" means the snapshot is written every N steps, not just at the end of the run. The "≥3× throughput" claim is from a benchmark to be run in P2-14c.

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL add a new field to `ShardConfig` to control the snapshot interval (N steps).
- THE SYSTEM SHALL write a `RunSnapshot` journal event after every N steps in `drive_deterministic_full`.
- THE SYSTEM SHALL preserve the existing drive loop semantics (deterministic replay invariant per master §68).

### Event-Driven
- WHEN `drive_deterministic_full` has executed N steps, THE SYSTEM SHALL write a `RunSnapshot` journal event capturing the current `RunFrame` state.
- WHEN the run terminates (success, failure, or cancellation), THE SYSTEM SHALL write a final `RunSnapshot` event.
- WHEN N = 0 (snapshot disabled), THE SYSTEM SHALL NOT write any `RunSnapshot` events.

### Unwanted
- THE SYSTEM SHALL NOT change the existing 5 fields of `ShardConfig`; the new field is additive.
- THE SYSTEM SHALL NOT break the deterministic replay invariant (master §68 invariant 4).
- THE SYSTEM SHALL NOT write `RunSnapshot` events inside the inner loop (only after N steps).
- THE SYSTEM SHALL NOT panic on any input (N=0, N=1, large N).

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs:
  - field: `ShardConfig`
    type: `struct` (extended with a new field)
    constraints: the new field controls the snapshot interval.
    example_valid: `ShardConfig { ..., snapshot_interval_steps: 100 }`
    example_invalid: `snapshot_interval_steps: 0` (snapshot disabled; valid).
- system_state:
  - `ShardConfig` has 5 fields at `config.rs:27-38`.
  - `drive_deterministic_full` is at `drive.rs:46-75`.
  - The drive loop has a `loop { ... }` structure that calls `begin_drive_step` and `execute_node_full`.

### Postconditions
- state_changes:
  - A new field is added to `ShardConfig` (e.g., `snapshot_interval_steps: u64`).
  - `drive_deterministic_full` writes a `RunSnapshot` event after every N steps.
- return_guarantees:
  - field: `RuntimeEngineResult<RuntimeSignal>`
    guarantee: unchanged from the existing function.
  - field: `journal events`
    guarantee: For every N steps, a `RunSnapshot` event is written.
- side_effects:
  - The journal is updated with `RunSnapshot` events.

### Invariants
- The drive loop remains deterministic: the same input produces the same sequence of `RunSnapshot` events.
- For N=0, no `RunSnapshot` events are written (snapshot disabled).
- For N=1, a `RunSnapshot` is written after every step.

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: `crates/vb_runtime/src/shard/config.rs:27-38`
  what_to_extract: The 5 existing fields of `ShardConfig`. Confirm: `command_queue_capacity, trace_capacity, step_budget_per_tick, max_active_runs, policy`.
  document_in: research_notes.md
- path: `crates/vb_runtime/src/engine/drive.rs:46-75`
  what_to_extract: The `drive_deterministic_full` function. Confirm the loop structure and the `begin_drive_step` / `execute_node_full` calls.
  document_in: research_notes.md
- path: `crates/vb_core/src/journal/events.rs`
  what_to_extract: The `JournalEvent::RunSnapshot` variant. Confirm it exists.
  document_in: research_notes.md
- path: `crates/vb_runtime/src/runtime/mod.rs:198-200`
  what_to_extract: The `Runtime::snapshot_run` method. Confirm it returns `InspectResponse`.
  document_in: research_notes.md

Patterns to find:
- pattern: `RunSnapshot`
  purpose: Locate the journal event variant.
  expected_locations: `crates/vb_core/src/journal/events.rs`.
- pattern: `drive_deterministic_full`
  purpose: Locate the function to modify.
  expected_locations: `crates/vb_runtime/src/engine/drive.rs:46`.

Prior art:
- feature: existing 5-field `ShardConfig`
  location: `crates/vb_runtime/src/shard/config.rs:27-38`
  what_to_learn: The pattern of named fields with defaults. Apply the same pattern to the new field.

External docs:
- url: master doc §68 (determinism invariant 4)
  section: deterministic replay
  extract: confirm the drive loop must be deterministic for replay.

Research questions (all answered):
- Q: Where is the new field declared? A: `ShardConfig`.
- Q: How is the snapshot written? A: In `drive_deterministic_full` after every N steps.
- Q: What about the terminal snapshot? A: Written at run termination regardless of N.

Research complete when:
- [x] All files_to_read opened.
- [x] All patterns_to_find searched.
- [x] All prior_art examined.
- [x] All research_questions have answers.

## Section 3. Inversions

### Security
- failure: The snapshot is written at a non-deterministic time, breaking the replay invariant.
  prevention: The snapshot is written after EXACTLY N steps, not at a wall-clock time. The interval is measured in steps, not time.
  test_for_it: `test_snapshot_at_deterministic_step: replay a run; assert the snapshots are at the same step indices`.

### Usability
- failure: A developer sets `snapshot_interval_steps = 0` (disabled) and the implementation panics on division by zero.
  prevention: `snapshot_interval_steps = 0` is a valid "disabled" value; the implementation skips the snapshot write.
  test_for_it: `test_zero_snapshot_interval_disables_snapshot: drive with interval=0; assert no RunSnapshot events are written`.

### Data Integrity
- failure: The snapshot is written mid-step, capturing an inconsistent state.
  prevention: The snapshot is written AFTER `finish_drive_step`, which is the natural atomic boundary.
  test_for_it: `test_snapshot_written_after_step_complete: drive for N+1 steps; assert the snapshot captures the state after step N, not during step N+1`.

### Integration Failure
- failure: The new field is added to `ShardConfig` but not propagated to all `Shard` constructors, causing a compile error.
  prevention: All `Shard::new` callers are updated in the same commit. The new field has a default value.
  test_for_it: `test_all_constructors_updated: rg 'ShardConfig' crates/vb_runtime/src/` returns 1 match per use site (no missing updates).

## Section 4. ATDD Tests

### Happy
- name: `test_drive_writes_snapshot_after_n_steps`
  given: A `ShardConfig` with `snapshot_interval_steps = 5`; a workflow that runs for 12 steps.
  when: `drive_deterministic_full` is called.
  then: 2 `RunSnapshot` events are written (after step 5 and step 10). A final snapshot is written at termination (after step 12).
  real_input: a workflow with 12 steps.
  expected_output: 3 `RunSnapshot` events (5, 10, 12).
- name: `test_drive_writes_snapshot_at_termination`
  given: A `ShardConfig` with `snapshot_interval_steps = 100`; a workflow that runs for 50 steps and then terminates.
  when: `drive_deterministic_full` is called.
  then: 0 mid-run snapshots; 1 final snapshot at termination.
  real_input: a workflow with 50 steps.
  expected_output: 1 `RunSnapshot` event at step 50.

### Error
- name: `test_drive_with_zero_snapshot_interval_writes_no_snapshots`
  given: A `ShardConfig` with `snapshot_interval_steps = 0`.
  when: `drive_deterministic_full` is called.
  then: 0 `RunSnapshot` events are written (even at termination).
  real_input: a workflow with 10 steps; `snapshot_interval_steps = 0`.
  expected_output: 0 `RunSnapshot` events.
- name: `test_drive_with_negative_snapshot_interval_rejected`
  given: A `ShardConfig` with `snapshot_interval_steps = -1` (invalid).
  when: `Shard::new(config)` is called.
  then: Returns `Err(ConfigError::InvalidSnapshotInterval)`.
  real_input: a negative interval (if the field is `i64`) or no negative (if `u64`).
  expected_error: `Err(ConfigError)`.

### Edge
- name: `test_drive_with_one_snapshot_interval_writes_snapshot_after_every_step`
  given: A `ShardConfig` with `snapshot_interval_steps = 1`; a workflow that runs for 5 steps.
  when: `drive_deterministic_full` is called.
  then: 5 `RunSnapshot` events (after every step) + 1 final = 6 total.
  real_input: a workflow with 5 steps.
  expected_output: 6 `RunSnapshot` events.
- name: `test_snapshot_determinism_across_replays`
  given: A workflow that runs for 10 steps with `snapshot_interval_steps = 3`.
  when: The workflow is run twice.
  then: Both runs produce the same `RunSnapshot` events at the same step indices.
  real_input: same workflow, same config.
  expected: identical event sequences.

### Contract
- name: `test_precondition_snapshot_interval_is_non_negative`
  verifies: Precondition "snapshot_interval_steps >= 0".
  test: assert the validator rejects negative values.
- name: `test_postcondition_snapshot_written_at_termination`
  verifies: Postcondition "a snapshot is always written at termination".
  test: assert at least 1 `RunSnapshot` event is written at the end of the run (if `snapshot_interval_steps > 0`).
- name: `test_invariant_drive_remains_deterministic`
  verifies: Invariant "the drive loop is deterministic".
  test: replay a run twice; assert the `RunSnapshot` events are at the same steps.

## Section 5. E2E Tests

```
pipeline_test:
  name: test_snapshot_writer_e2e
  description: Real FjallJournal, real Shard, real workflow; verify snapshots are written at the right steps.
  setup:
    - open a real FjallJournal
    - build a Shard with snapshot_interval_steps=5
    - submit a workflow that runs for 12 steps
  execute:
    - run the workflow
    - count the RunSnapshot events
  verify:
    - count == 3 (steps 5, 10, 12)
  cleanup:
    - close FjallJournal

e2e_scenarios:
  - name: e2e_snapshot_at_correct_steps
    description: prove the snapshots are at the right steps
    steps:
      - submit 12-step workflow
      - tick 12 times
      - count RunSnapshot events
      - verify count == 3
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] `config.rs:27-38` read (5 fields confirmed)"
    - "[x] `drive.rs:46-75` read (loop structure confirmed)"
    - "[x] `journal/events.rs` read (RunSnapshot variant confirmed)"
    - "[x] `runtime/mod.rs:198-200` read (snapshot_run method)"
  evidence_required:
    - "Research notes file with line-numbered extracts"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All 7 acceptance tests written (2 happy, 2 error, 2 edge, 3 contract)"
    - "[ ] Tests fail with compile error (field does not exist yet)"
  evidence_required:
    - "Test file in `crates/vb_runtime/src/engine/tests.rs`"
    - "Compile error output"

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
    - "[ ] E2E test passes with real FjallJournal + real Shard"
    - "[ ] No regressions in runtime tests"
  evidence_required:
    - "E2E output"

## Section 6. Implementation Tasks

### Phase 0: Research
- [ ] Read `config.rs:27-38` (parallel: research)
- [ ] Read `drive.rs:46-75` (parallel: research)
- [ ] Read `journal/events.rs` for `RunSnapshot` (parallel: research)
- [ ] Read `runtime/mod.rs:198-200` for `snapshot_run` (parallel: research)

### Phase 1: Tests
- [ ] Write `test_drive_writes_snapshot_after_n_steps` (parallel: tests)
- [ ] Write `test_drive_writes_snapshot_at_termination` (parallel: tests)
- [ ] Write `test_drive_with_zero_snapshot_interval_writes_no_snapshots` (parallel: tests)
- [ ] Write `test_drive_with_negative_snapshot_interval_rejected` (parallel: tests)
- [ ] Write `test_drive_with_one_snapshot_interval_writes_snapshot_after_every_step` (parallel: tests)
- [ ] Write `test_snapshot_determinism_across_replays` (parallel: tests)
- [ ] Write 3 contract tests (parallel: tests)
- [ ] Confirm all 7 tests fail (gate)

### Phase 2: Implementation
- [ ] Add `snapshot_interval_steps: u64` to `ShardConfig` (depends: tests; sequential)
- [ ] Update `Default for ShardConfig` to set a reasonable default (e.g., 1000) (depends: field; sequential)
- [ ] Modify `drive_deterministic_full` to write a `RunSnapshot` after every N steps (depends: config; sequential)
- [ ] Always write a final `RunSnapshot` at termination (if `snapshot_interval_steps > 0`) (depends: mid-run; sequential)
- [ ] Confirm all 7 tests pass (gate: green)

### Phase 3: Integration
- [ ] Write the E2E test (depends: impl; sequential)
- [ ] Run the E2E test (sequential)
- [ ] Run `cargo test -p vb_runtime` to confirm no regressions (sequential)

### Phase 4: Documentation
- [ ] Run `moon run :ci` (depends: all of the above; parallel)
- [ ] Close the bead (sequential)

## Section 7. Failure Modes

- symptom: "Compile error: cannot find field `snapshot_interval_steps` on `ShardConfig`"
  likely_cause: The new field was not added.
  where_to_look:
    - file: `crates/vb_runtime/src/shard/config.rs:27-38`
    - what_to_check: "Is the new field declared?"
  fix_pattern: Add the field.
- symptom: "Test fails: snapshots are written at the wrong steps (off by one)"
  likely_cause: The snapshot is written BEFORE `finish_drive_step` instead of after.
  where_to_look:
    - file: `crates/vb_runtime/src/engine/drive.rs:46-75`
    - function: `drive_deterministic_full`
    - what_to_check: "Is the snapshot written AFTER `finish_drive_step` (and the step counter increment)?"
  fix_pattern: Move the snapshot write AFTER `finish_drive_step`.
- symptom: "Test fails: drive is no longer deterministic"
  likely_cause: The snapshot is written at a wall-clock time, not after a fixed step count.
  where_to_look:
    - file: `crates/vb_runtime/src/engine/drive.rs:46-75`
    - what_to_check: "Is the snapshot triggered by step count, not by time?"
  fix_pattern: Use step count, not time.

debugging_commands:
- scenario: "When snapshots are at wrong steps"
  run: "RUST_LOG=vb_runtime=trace cargo test -p vb_runtime drive_writes_snapshot"
  look_for: "Trace log showing step counts when snapshots are written"
- scenario: "When drive is not deterministic"
  run: "rg 'SystemTime|Instant|Duration' crates/vb_runtime/src/engine/drive.rs"
  look_for: "Any time-based snapshot trigger; should be ZERO matches"

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT change the existing 5 fields of `ShardConfig`; the new field is additive.
- DO NOT break the deterministic replay invariant.
- DO NOT write `RunSnapshot` events inside the inner loop.
- DO NOT use `unwrap()` or `expect()` in new code.

VERIFY that:
- `ShardConfig` has 5 fields before this bead: `rg 'pub struct ShardConfig' crates/vb_runtime/src/shard/config.rs` (must show 5 fields before impl; 6 after).
- `drive_deterministic_full` is at `drive.rs:46`: `rg 'fn drive_deterministic_full' crates/vb_runtime/src/engine/drive.rs` (must return 1 match).
- `RunSnapshot` variant exists: `rg 'RunSnapshot' crates/vb_core/src/journal/events.rs` (must return at least 1 match).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    rg 'snapshot_interval_steps' crates/vb_runtime/src/shard/config.rs  # confirm the new field is wired

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-8tjk8/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-8tjk8/progress.txt` and continue from "Current Task". The interval is in STEPS, not time.
Key invariants:
- The new field is `snapshot_interval_steps: u64` (in steps, not time).
- A `RunSnapshot` is written after every N steps.
- A final `RunSnapshot` is written at termination (if `snapshot_interval_steps > 0`).
- The drive loop is deterministic (master §68 invariant 4).

## Section 8. Completion Checklist

- [ ] All 7 acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing with real FjallJournal + real Shard
- [ ] No mocks or fake data in any test
- [ ] moon run :ci passes
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] No production code touched outside `crates/vb_runtime/src/shard/config.rs` and `crates/vb_runtime/src/engine/drive.rs`
- [ ] bd remember note: "Round 3 black-hat APPROVED. 16-section content generated from source read."
- [ ] bd close with reason: "P2-18r complete: RunSnapshot written every N steps; deterministic replay preserved"

## Section 9. Context

Related files:
- `crates/vb_runtime/src/shard/config.rs:27-38` — `ShardConfig` (5 fields, the struct to extend)
- `crates/vb_runtime/src/engine/drive.rs:46-75` — `drive_deterministic_full` (the function to modify)
- `crates/vb_core/src/journal/events.rs` — `JournalEvent` enum (RunSnapshot variant)
- `crates/vb_runtime/src/runtime/mod.rs:198-200` — `Runtime::snapshot_run` (the existing snapshot method)
- master doc §68 — deterministic replay invariant

Similar implementations:
- The existing 5-field `ShardConfig` shows the pattern of named fields with defaults. Apply the same pattern to the new field.

Codebase patterns:
- pattern: "Step counter + interval check"
  example_location: `crates/vb_runtime/src/engine/drive.rs:46-75` (the existing drive loop)
  how_to_apply: Add a step counter; after each step, check `if step_count % interval == 0 { write_snapshot() }`.

## Section 10. AI Hints

### DO
- Read `crates/vb_runtime/src/engine/drive.rs:46-75` BEFORE writing any code.
- Use a step counter to trigger the snapshot (NOT wall-clock time).
- Write the final snapshot at termination (if interval > 0).
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT use `unwrap()` or `expect()`.
- Do NOT use wall-clock time (`SystemTime`, `Instant`, `Duration`).
- Do NOT change the existing 5 fields of `ShardConfig`.
- Do NOT break the deterministic replay invariant.
- Do NOT use `unsafe`.

### Code patterns
- name: "Step counter + interval check"
  use_when: "Triggering a periodic action after N steps"
  example: |
    let mut step_count: u64 = 0;
    loop {
        // ... existing drive loop ...
        step_count += 1;
        if snapshot_interval > 0 && step_count % snapshot_interval == 0 {
            write_run_snapshot(run)?;
        }
    }

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect().
- Test first: Tests MUST exist and FAIL before implementation.
- Read before write: ALWAYS read the source file before modifying it.
- Real data only: Use real `ShardConfig` fields; no fabricated placeholders.
- Minimal change: ONE new field + ONE new branch in the drive loop; do NOT refactor the engine.
