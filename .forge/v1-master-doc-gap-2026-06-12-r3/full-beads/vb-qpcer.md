# P2-14b2 shard-tick-coalesce

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> `crates/vb_runtime/src/shard/config.rs` (lines 20-62), `crates/vb_runtime/src/shard/impl_parts/dispatch.rs` (lines 1-50).

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications:
- The unit is TICKS, not wall-clock time. The sync tick architecture has no time anchor (per dispatch.rs:3-17, tick is synchronous and processes one command per call). The rejected P2-14b used `coalesce_window_us`; this revision uses `coalesce_window_ticks: u32`.
- The real file is `crates/vb_runtime/src/shard/impl_parts/dispatch.rs` (NOT `shard/tick.rs` which does NOT exist).
- The default for the new field is `1` (no coalescing) to preserve current single-command-per-tick behavior.

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL add a new field `coalesce_window_ticks: u32` to `ShardConfig` (default: 1).
- THE SYSTEM SHALL preserve the existing `tick()` single-command-per-call behavior when `coalesce_window_ticks == 1`.
- THE SYSTEM SHALL use tick counts, not wall-clock time, for the coalescing window (sync tick architecture has no time anchor).

### Event-Driven
- WHEN `Shard::tick()` is called and `coalesce_window_ticks == 1`, THE SYSTEM SHALL dispatch exactly one command per call (current behavior).
- WHEN `Shard::tick()` is called and `coalesce_window_ticks == 10`, THE SYSTEM SHALL accumulate up to 10 commands across 10 tick calls, then dispatch them as a batch.
- WHEN the coalescing window expires (counter reaches 0), THE SYSTEM SHALL call `append_sequenced_batch` (P2-14a) to commit the accumulated events atomically.

### Unwanted
- THE SYSTEM SHALL NOT use wall-clock time (`SystemTime`, `Instant`, `Duration`) for the window. Sync ticks have no time anchor.
- THE SYSTEM SHALL NOT cite `crates/vb_runtime/src/shard/tick.rs` — that file does NOT exist. The real file is `impl_parts/dispatch.rs`.
- THE SYSTEM SHALL NOT add `coalesce_window_us: u64` (the unit is `ticks`, not microseconds).
- THE SYSTEM SHALL NOT mutate the existing 5 fields of `ShardConfig`; the new field is additive.

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs:
  - field: `ShardConfig`
    type: `struct` with 6 fields (5 existing + new `coalesce_window_ticks`)
    constraints: `coalesce_window_ticks >= 1` (0 would be a degenerate "infinite coalesce" case; rejected).
    example_valid: `ShardConfig { command_queue_capacity: 1024, trace_capacity: 4096, step_budget_per_tick: 1000, max_active_runs: 1024, policy: RuntimePolicy::Strict, coalesce_window_ticks: 1 }`
    example_invalid: `ShardConfig { coalesce_window_ticks: 0, ... }` (degenerate; rejected)
- system_state:
  - `ShardConfig` has 5 fields at `config.rs:27-38`; this bead adds a 6th.
  - `Shard::tick()` is at `impl_parts/dispatch.rs:3-17`; this bead adds a coalescing layer.

### Postconditions
- state_changes:
  - `ShardConfig` has 6 fields (5 existing + new `coalesce_window_ticks: u32`).
  - `Default for ShardConfig` returns `coalesce_window_ticks: 1` (no coalescing by default).
  - `Shard` has a new field `current_coalesce_window_remaining: u32` to track the active window.
- return_guarantees:
  - field: `ShardConfig::coalesce_window_ticks`
    guarantee: Always >= 1 (enforced by `is_valid_coalesce_window_ticks`).
- side_effects: None directly. The window tracking is internal to `Shard`.

### Invariants
- For `coalesce_window_ticks == 1`, the dispatch rate is exactly 1 command per tick (regression-protected).
- For `coalesce_window_ticks == N`, the dispatch rate is exactly 1 batch of up to N commands per N ticks.
- The window counter is bounded by `u32` (max 4 billion ticks); no overflow risk in realistic workloads.
- The sync tick architecture is preserved: `tick()` remains synchronous and deterministic for replay (master §68 invariant 4).

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: `crates/vb_runtime/src/shard/config.rs:27-38`
  what_to_extract: The 5 existing fields of `ShardConfig`: `command_queue_capacity: usize`, `trace_capacity: usize`, `step_budget_per_tick: u64`, `max_active_runs: usize`, `policy: vb_core::policy::RuntimePolicy`.
  document_in: research_notes.md
- path: `crates/vb_runtime/src/shard/config.rs:52-62`
  what_to_extract: The `Default for ShardConfig` implementation. Confirm the 5 default values.
  document_in: research_notes.md
- path: `crates/vb_runtime/src/shard/impl_parts/dispatch.rs:3-17`
  what_to_extract: The `Shard::tick` function signature `pub fn tick(&mut self) -> RuntimeResult<bool>` and its body. Confirm it pops ONE command from the queue.
  document_in: research_notes.md

Patterns to find:
- pattern: `shard/tick.rs`
  purpose: Verify the file does NOT exist (the rejected P2-14r cited it; that was wrong).
  expected_locations: NONE — the file does not exist.
- pattern: `ShardConfig`
  purpose: Locate all uses of `ShardConfig` to ensure the new field is propagated.
  expected_locations: `crates/vb_runtime/src/shard/config.rs` (definition), `crates/vb_runtime/src/shard/impl_parts/dispatch.rs` (use).

Prior art:
- feature: existing `ShardConfig` struct
  location: `crates/vb_runtime/src/shard/config.rs:27-38`
  what_to_learn: The pattern of named fields with defaults. Apply the same pattern to the new `coalesce_window_ticks` field.

External docs:
- url: master doc §68 (determinism invariant 4)
  section: deterministic replay
  extract: confirm that sync ticks must be deterministic for replay (no time-based accumulation).

Research questions (all answered):
- Q: Should the new field be `u32` or `u64`? A: `u32` (max 4 billion ticks is sufficient; u64 is overkill).
- Q: What is the default? A: 1 (preserves current behavior).
- Q: Where does the new field live? A: `ShardConfig` (alongside the other 5 fields).

Research complete when:
- [x] All files_to_read opened.
- [x] All patterns_to_find searched (no `shard/tick.rs`).
- [x] All prior_art examined.
- [x] All research_questions have answers.

## Section 3. Inversions

### Security
- failure: A future bead adds a wall-clock-based coalescing field (`coalesce_window_us`) that bypasses the tick-count guarantee, allowing non-deterministic replay attacks.
  prevention: The new field is HARD-CODED as tick-counts (`u32`). Any wall-clock field is a separate bead and requires a master-doc amendment.
  test_for_it: `test_no_wall_clock_field: rg "coalesce_window_us|coalesce_window_ms" crates/vb_runtime/src/shard/config.rs` returns ZERO matches.

### Usability
- failure: A developer sets `coalesce_window_ticks = 0` expecting "infinite coalesce" but the implementation treats it as a degenerate case and panics.
  prevention: The `is_valid_coalesce_window_ticks` validator returns false for 0; the constructor rejects 0.
  test_for_it: `test_zero_coalesce_window_rejected: ShardConfig { coalesce_window_ticks: 0, ... }` is rejected by the validator (returns false).

### Data Integrity
- failure: The window counter overflows after 4 billion ticks, causing a panic or wrap-around that silently breaks coalescing.
  prevention: The counter is `u32`; max 4 billion. At 1 tick per microsecond, overflow takes ~71 minutes. Realistic workloads reset the counter every N ticks; the counter is reset on window expiry.
  test_for_it: `test_window_counter_bounded: simulate 4 billion ticks; assert the counter wraps cleanly without panic`.

### Integration Failure
- failure: The `Shard::tick` function is called from a runtime loop that expects synchronous semantics; the new coalescing layer blocks the thread while waiting for the window to expire.
  prevention: The window is internal; the function returns immediately on each call (either dispatching 1 command or decrementing the counter). No blocking, no sleep.
  test_for_it: `test_tick_returns_immediately: Shard::tick() returns within 1 microsecond regardless of coalesce_window_ticks value`.

## Section 4. ATDD Tests

### Happy
- name: `test_shard_tick_with_window_1_dispatches_one_command_per_tick`
  given: A `Shard` with `coalesce_window_ticks = 1` and 100 Submit commands pushed to the queue.
  when: `tick()` is called 100 times.
  then: Each call dispatches exactly 1 command. 100 dispatches in 100 ticks.
  real_input: 100 `Submit` commands, `coalesce_window_ticks = 1`.
  expected_output: `dispatches_per_tick == 1` for each of 100 ticks.
- name: `test_shard_tick_with_window_10_dispatches_batch_after_10_ticks`
  given: A `Shard` with `coalesce_window_ticks = 10` and 100 Submit commands.
  when: `tick()` is called 100 times.
  then: Exactly 10 batch commits are issued (one per 10-tick window).
  real_input: 100 `Submit` commands, `coalesce_window_ticks = 10`.
  expected_output: `batch_commits == 10` after 100 ticks.

### Error
- name: `test_shard_tick_with_invalid_config_rejected`
  given: A `ShardConfig` with `coalesce_window_ticks = 0`.
  when: `Shard::new(config)` is called.
  then: Returns `Err(ConfigError::InvalidCoalesceWindow)`.
  real_input: `ShardConfig { coalesce_window_ticks: 0, ... }`.
  expected_error: `Err(ConfigError)`.
- name: `test_shard_tick_with_empty_queue_does_nothing`
  given: A `Shard` with `coalesce_window_ticks = 10` and an empty command queue.
  when: `tick()` is called 10 times.
  then: No commands are dispatched; the window counter is decremented (or the window is reset to 10 on expiry).
  real_input: empty queue, `coalesce_window_ticks = 10`.
  expected_output: `dispatches == 0` after 10 ticks.

### Edge
- name: `test_shard_tick_with_window_larger_than_queue_size`
  given: A `Shard` with `coalesce_window_ticks = 100` and only 5 commands in the queue.
  when: `tick()` is called 100 times.
  then: All 5 commands are dispatched in the first 5 ticks; the remaining 95 ticks decrement the window counter without dispatching.
  real_input: 5 commands, `coalesce_window_ticks = 100`.
  expected_output: `dispatches == 5`; window counter reaches 0 by tick 100.
- name: `test_shard_tick_with_window_1_is_equivalent_to_no_coalescing`
  given: Two `Shard` instances: one with `coalesce_window_ticks = 1`, one with the old behavior (no field).
  when: Both are ticked 100 times with 100 commands.
  then: Both produce the same dispatch pattern (1 per tick).
  real_input: 100 commands, `coalesce_window_ticks = 1`.
  expected_output: identical dispatches_per_tick.

### Contract
- name: `test_precondition_coalesce_window_ticks_must_be_positive`
  verifies: Precondition "`coalesce_window_ticks >= 1`".
  test: assert `is_valid_coalesce_window_ticks(0) == false`.
- name: `test_postcondition_default_is_1`
  verifies: Postcondition "default is 1 (no coalescing)".
  test: assert `ShardConfig::default().coalesce_window_ticks == 1`.
- name: `test_invariant_window_1_preserves_old_behavior`
  verifies: Invariant "for window=1, behavior is unchanged".
  test: assert the dispatch pattern for window=1 matches the pre-bead behavior exactly.

## Section 5. E2E Tests

```
pipeline_test:
  name: test_shard_tick_coalesce_e2e
  description: Real FjallJournal, real Shard, real CommandQueue; submit 100 commands; measure commit count.
  setup:
    - open a real FjallJournal
    - create a Shard with coalesce_window_ticks=10
    - push 100 Submit commands
  execute:
    - call Shard::tick() 100 times
    - count the journal commits
  verify:
    - commits == 10 (10 batches of 10 commands each)
    - all 100 commands appear in the journal
  cleanup:
    - close FjallJournal

e2e_scenarios:
  - name: e2e_coalesce_window_10_commits_10_batches
    description: prove the 10:1 reduction in journal commits
    steps:
      - submit 100 commands
      - tick 100 times
      - count commits
      - verify == 10
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] `config.rs:27-38` and `52-62` read"
    - "[x] `impl_parts/dispatch.rs:3-17` read"
    - "[x] Confirmed: NO `shard/tick.rs` exists"
    - "[x] Confirmed: tick is synchronous, no time anchor"
  evidence_required:
    - "Research notes file with line-numbered extracts"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All 7 acceptance tests written (2 happy, 2 error, 2 edge, 3 contract)"
    - "[ ] Tests fail with compile error (field does not exist yet)"
  evidence_required:
    - "Test file in `crates/vb_runtime/src/shard/tests.rs`"
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
    - "[ ] No regressions in shard tests"
  evidence_required:
    - "E2E output"
    - "Manual verification"

## Section 6. Implementation Tasks

### Phase 0: Research
- [ ] Read `config.rs:27-38` (parallel: research)
- [ ] Read `config.rs:52-62` (parallel: research)
- [ ] Read `impl_parts/dispatch.rs:3-17` (parallel: research)
- [ ] Confirm `shard/tick.rs` does NOT exist (parallel: research)
- [ ] Read P2-14a's `append_sequenced_batch` API (parallel: research; this is the batch commit API)

### Phase 1: Tests
- [ ] Write `test_shard_tick_with_window_1_dispatches_one_command_per_tick` (parallel: tests)
- [ ] Write `test_shard_tick_with_window_10_dispatches_batch_after_10_ticks` (parallel: tests)
- [ ] Write `test_shard_tick_with_invalid_config_rejected` (parallel: tests)
- [ ] Write `test_shard_tick_with_empty_queue_does_nothing` (parallel: tests)
- [ ] Write `test_shard_tick_with_window_larger_than_queue_size` (parallel: tests)
- [ ] Write `test_shard_tick_with_window_1_is_equivalent_to_no_coalescing` (parallel: tests)
- [ ] Write 3 contract tests (parallel: tests)
- [ ] Confirm all 7 tests fail (gate)

### Phase 2: Implementation
- [ ] Add `coalesce_window_ticks: u32` field to `ShardConfig` (depends: tests; sequential)
- [ ] Update `Default for ShardConfig` to set `coalesce_window_ticks: 1` (depends: field; sequential)
- [ ] Add `is_valid_coalesce_window_ticks` validator (depends: field; sequential)
- [ ] Add `current_coalesce_window_remaining: u32` field to `Shard` (depends: config; sequential)
- [ ] Modify `Shard::tick()` to implement the coalescing layer (depends: shard field; sequential)
- [ ] When the window expires, call `append_sequenced_batch` (depends: P2-14a; sequential)
- [ ] Confirm all 7 tests pass (gate: green)

### Phase 3: Integration
- [ ] Write the E2E test (depends: impl; sequential)
- [ ] Run the E2E test (sequential)
- [ ] Run `cargo test -p vb_runtime` to confirm no regressions (sequential)

### Phase 4: Documentation
- [ ] Run `moon run :ci` (depends: all of the above; parallel)
- [ ] Close the bead (sequential)

## Section 7. Failure Modes

- symptom: "Compile error: cannot find field `coalesce_window_ticks` on `ShardConfig`"
  likely_cause: Test was written before the field was added.
  where_to_look:
    - file: `crates/vb_runtime/src/shard/config.rs`
    - function: `ShardConfig` struct
    - what_to_check: "Is the new field declared?"
  fix_pattern: Add the field with the documented type and default.
- symptom: "Test fails: window counter never expires"
  likely_cause: The `current_coalesce_window_remaining` field is initialized to the wrong value (e.g., 0 instead of `coalesce_window_ticks`).
  where_to_look:
    - file: `crates/vb_runtime/src/shard/impl_parts/dispatch.rs`
    - function: `Shard::new` or the field initializer
    - what_to_check: "Is the counter initialized to `config.coalesce_window_ticks`?"
  fix_pattern: Initialize the counter to `config.coalesce_window_ticks` in the `Shard::new` constructor.
- symptom: "Test fails: dispatches per tick is wrong (e.g., 11 instead of 10)"
  likely_cause: The window is not decremented correctly (off-by-one in the counter).
  where_to_look:
    - file: `crates/vb_runtime/src/shard/impl_parts/dispatch.rs`
    - function: `Shard::tick`
    - what_to_check: "Is the counter decremented BEFORE the dispatch check?"
  fix_pattern: Decrement first, then check if the counter is 0; if 0, dispatch and reset.

debugging_commands:
- scenario: "When the window counter is wrong"
  run: "RUST_LOG=vb_runtime=trace cargo test -p vb_runtime shard_tick"
  look_for: "Trace log showing the counter value at each tick"
- scenario: "When the dispatches per tick is wrong"
  run: "rg 'current_coalesce_window_remaining' crates/vb_runtime/src/shard/"
  look_for: "All read/write sites of the counter; check for off-by-one"

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT cite `crates/vb_runtime/src/shard/tick.rs` — that file does NOT exist. The real file is `impl_parts/dispatch.rs`.
- DO NOT use wall-clock time (`SystemTime`, `Instant`, `Duration`) for the window. Sync ticks have no time anchor.
- DO NOT add `coalesce_window_us: u64` — the unit is `ticks`, not microseconds.
- DO NOT mutate the existing 5 fields of `ShardConfig`; the new field is additive.

VERIFY that:
- `shard/tick.rs` does NOT exist: `rg 'fn tick' crates/vb_runtime/src/shard/tick.rs` (must return ZERO matches; the file does not exist).
- The real dispatch is in `impl_parts/dispatch.rs`: `rg 'pub fn tick' crates/vb_runtime/src/shard/impl_parts/dispatch.rs` (must return exactly 1 match).
- The 5 existing fields of `ShardConfig` are unchanged: `rg 'pub struct ShardConfig' crates/vb_runtime/src/shard/config.rs` (must show 6 fields after impl, not fewer).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    rg 'coalesce_window_ticks' crates/vb_runtime/src/shard/  # confirm the field is wired

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-qpcer/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-qpcer/progress.txt` and continue from "Current Task". The unit is TICKS, not wall-clock; do not switch to `coalesce_window_us`.
Key invariants:
- The unit is TICKS, not wall-clock time.
- The real file is `shard/impl_parts/dispatch.rs`, NOT `shard/tick.rs`.
- The default is `1` (preserves current behavior).
- The field is `u32` (max 4 billion ticks).

## Section 8. Completion Checklist

- [ ] All 7 acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing with real FjallJournal + real Shard
- [ ] No mocks or fake data in any test
- [ ] moon run :ci passes
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] No production code touched outside `crates/vb_runtime/src/shard/config.rs` and `impl_parts/dispatch.rs`
- [ ] bd close with reason: "P2-14b2 complete: coalesce_window_ticks: u32 added; sync tick architecture preserved"

## Section 9. Context

Related files:
- `crates/vb_runtime/src/shard/config.rs:27-38` — `ShardConfig` struct (5 fields)
- `crates/vb_runtime/src/shard/config.rs:52-62` — `Default for ShardConfig`
- `crates/vb_runtime/src/shard/impl_parts/dispatch.rs:3-17` — `Shard::tick` (the function to modify)
- master doc §68 (determinism invariant 4) — sync tick architecture must be deterministic for replay

Similar implementations:
- The existing `step_budget_per_tick: u64` field shows the pattern of "per-tick" configuration. Apply the same naming and unit semantics to `coalesce_window_ticks`.

Codebase patterns:
- pattern: "Per-tick configuration field"
  example_location: `crates/vb_runtime/src/shard/config.rs:33` (`step_budget_per_tick: u64`)
  how_to_apply: Use the same per-tick naming convention; document the unit in the field's doc comment.

## Section 10. AI Hints

### DO
- Read `crates/vb_runtime/src/shard/impl_parts/dispatch.rs:3-17` BEFORE writing any code. The function is short; the read is fast.
- Use `u32` for `coalesce_window_ticks` (max 4 billion ticks is sufficient; u64 is overkill).
- Default to `1` to preserve current behavior.
- Add a doc comment explaining the unit is TICKS, not wall-clock.
- Add an `is_valid_coalesce_window_ticks` validator (returns false for 0).
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT use `unwrap()` or `expect()`.
- Do NOT cite `shard/tick.rs` (the file does not exist).
- Do NOT use wall-clock time.
- Do NOT add `coalesce_window_us`.
- Do NOT use `unsafe`.

### Code patterns
- name: "Per-tick configuration field with validator"
  use_when: "Adding a new bounded configuration field to a per-tick context"
  example: |
    pub struct ShardConfig {
        // ... existing 5 fields ...
        /// Number of ticks to accumulate commands before dispatching as a batch.
        /// Default: 1 (no coalescing). Must be >= 1.
        pub coalesce_window_ticks: u32,
    }
    pub const fn is_valid_coalesce_window_ticks(n: u32) -> bool { n >= 1 }

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect().
- Test first: Tests MUST exist and FAIL before implementation.
- Read before write: ALWAYS read the source file before modifying it.
- Real data only: Use real ShardConfig field names; no fabricated placeholders.
- Sync tick invariant: The sync tick architecture is preserved (master §68 invariant 4).
