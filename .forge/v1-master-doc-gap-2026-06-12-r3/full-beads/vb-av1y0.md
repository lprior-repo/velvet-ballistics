# P0-5b2 recover-pending-actions

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> `crates/vb_storage/src/recovery/recover.rs`, `crates/vb_storage/src/recovery/replay/summary.rs`,
> `crates/vb_storage/src/recovery/types.rs`, `crates/vb_runtime/src/runtime/mod.rs`.

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications:
- The data is ALREADY exposed on the `RecoveryFrameSeed` returned by `recover_runtime_frame_seed` (recover.rs:251-260, types.rs:377-396). The gap is that `Runtime::recover` does not call this function directly.
- The new public accessor name is `pending_actions_from_events` (with `_from_events` suffix), not `recover_pending_actions`. The real signature is `(events: &[JournalEvent]) -> Vec<RecoveredPendingAction>`.
- The return type is `Vec<RecoveredPendingAction>` (the 2-field struct from types.rs:290-297), NOT `Vec<ActionTicket>` (which has 7 fields and a different meaning).

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL expose a public function `pending_actions_from_events(events: &[JournalEvent]) -> Vec<RecoveredPendingAction>` in `vb_storage::recovery::replay::summary`.
- THE SYSTEM SHALL preserve the existing private `recovered_pending_actions` function (no rename, no signature change).
- THE SYSTEM SHALL NOT add a new trait method to satisfy this gap (the public accessor is a free function).

### Event-Driven
- WHEN `pending_actions_from_events` is called with `events` containing N `ActionScheduled` events and M `ActionCompleted` events (M < N), THE SYSTEM SHALL return a `Vec<RecoveredPendingAction>` of length N - M (the uncompleted actions).
- WHEN `pending_actions_from_events` is called with `events = []`, THE SYSTEM SHALL return an empty `Vec`.
- WHEN `pending_actions_from_events` is called with `events` containing only terminal events (`RunFinished`, `RunFailedEvent`, `RunCancelled`), THE SYSTEM SHALL return an empty `Vec`.

### Unwanted
- THE SYSTEM SHALL NOT add a function `recover_pending_actions(events: &[JournalEvent]) -> Vec<ActionTicket>` — the signature is wrong (real signature has a different return type).
- THE SYSTEM SHALL NOT use `ActionTicket` as the return type — the real type is `RecoveredPendingAction` (2 fields: `step: StepIdx`, `action: ActionId`).
- THE SYSTEM SHALL NOT modify the existing private `recovered_pending_actions(pending_actions: HashSet<(ActionId, StepIdx)>)` — its signature is correct for the internal accumulator.

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs:
  - field: `events`
    type: `&[JournalEvent]`
    constraints: Each element is a `JournalEvent` variant. The slice can be empty.
    example_valid: `&[JournalEvent::ActionScheduled { step: StepIdx::new(0), action: ActionId::new(7), ... }, JournalEvent::ActionCompleted { ... }]`
    example_invalid: N/A (the slice is borrowed; no ownership constraints)
- system_state:
  - The private `recovered_pending_actions(pending_actions: HashSet<(ActionId, StepIdx)>) -> Vec<RecoveredPendingAction>` exists at `summary.rs:814-821`.
  - The private `recover_pending_actions_from_events_inner(events: &[JournalEvent])` accumulator helper must exist (it is the inner scan that builds the HashSet).

### Postconditions
- state_changes:
  - A new public function `pending_actions_from_events` is added at the module level of `summary.rs` (after line 821, or as a `pub use` re-export).
  - No other state changes; the existing private functions are unchanged.
- return_guarantees:
  - field: `Vec<RecoveredPendingAction>`
    guarantee: Contains exactly the actions in the input events that were `ActionScheduled` but not `ActionCompleted`. The set ordering follows `HashSet` iteration order, which is deterministic for a given input (the HashSet is built from a deterministic sequence of events).
    guarantee length: `events.iter().filter(is_action_scheduled).count() - events.iter().filter(is_action_completed).count()`
- side_effects: None. The function is pure (reads the events, no I/O).

### Invariants
- For any input `events`, the returned `Vec<RecoveredPendingAction>` is a subset of the scheduled actions in the input.
- For any input `events` where all scheduled actions are completed, the returned `Vec` is empty.
- The function does not panic on any input (empty slice, all-terminal events, malformed action ids).

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: `crates/vb_storage/src/recovery/replay/summary.rs:814-821`
  what_to_extract: The existing private `recovered_pending_actions` function signature and body. Confirm it takes `HashSet<(ActionId, StepIdx)>` and returns `Vec<RecoveredPendingAction>`.
  document_in: research_notes.md
- path: `crates/vb_storage/src/recovery/types.rs:290-297`
  what_to_extract: The `RecoveredPendingAction` struct fields. Confirm it has exactly 2 fields: `pub step: StepIdx` and `pub action: ActionId`.
  document_in: research_notes.md
- path: `crates/vb_storage/src/recovery/types.rs:377-396`
  what_to_extract: The `RecoveryFrameSeed` struct fields. Confirm `pending_actions: Vec<RecoveredPendingAction>` is a field.
  document_in: research_notes.md
- path: `crates/vb_storage/src/recovery/recover.rs:251-260`
  what_to_extract: The public `recover_runtime_frame_seed` function signature. Confirm `(journal: &FjallJournal, run: RunId) -> RecoveryResult<RecoveryFrameSeed>`.
  document_in: research_notes.md
- path: `crates/vb_runtime/src/runtime/mod.rs:343-362`
  what_to_extract: The existing `Runtime::recover` implementation. Confirm it calls `recover_all_incomplete_runs` but does NOT call `recover_runtime_frame_seed` directly.
  document_in: research_notes.md

Patterns to find:
- pattern: `recover_pending_actions_from_events_inner`
  purpose: Locate the inner scan function that builds the HashSet accumulator.
  expected_locations: `crates/vb_storage/src/recovery/replay/summary.rs` (in the same file as `recovered_pending_actions`).
- pattern: `pending_actions_from_events`
  purpose: Verify the new function does NOT exist; this bead adds it.
  expected_locations: NONE — this is a new function.

Prior art:
- feature: `recover_runtime_frame_seed` (already public, returns seed with pending_actions)
  location: `crates/vb_storage/src/recovery/recover.rs:251-260`
  what_to_learn: The pattern of returning `RecoveryFrameSeed` (a struct with multiple fields). For the new public accessor, we return ONLY the pending_actions subset.

External docs: None (this is a refactoring within the existing recovery crate).

Research questions (all answered):
- Q: Should the new function live in `summary.rs` or `recover.rs`? A: In `summary.rs` (alongside the private `recovered_pending_actions` it delegates to).
- Q: Does the new function need a feature gate? A: No (it is a pure helper, no test-util gating required).

Research complete when:
- [x] All files_to_read opened and key info extracted.
- [x] All patterns_to_find searched.
- [x] All prior_art examined.
- [x] All research_questions have answers documented.

## Section 3. Inversions

### Security
- failure: A test or observability tool calls `pending_actions_from_events` with attacker-controlled `events`, and the function leaks raw action ids or step indices to logs.
  prevention: The function returns a structured `Vec<RecoveredPendingAction>` (a 2-field struct), not raw bytes. Log output is bounded to 2 fields per entry.
  test_for_it: `test_return_type_is_structured_not_raw: pending_actions_from_events(&events).iter().all(|a| a.step == StepIdx::new(_) && a.action == ActionId::new(_))`.

### Usability
- failure: A developer calls the wrong function (`recover_pending_actions`) which doesn't exist; compile error confuses them.
  prevention: The new function is named `pending_actions_from_events` (with the `_from_events` suffix to make the input type clear) and the round-2 wrong name `recover_pending_actions` is NOT used.
  test_for_it: `test_documented_name_in_module: rg "pub fn pending_actions_from_events" crates/vb_storage/src/recovery/replay/summary.rs` returns exactly 1 match.

### Data Integrity
- failure: The function counts `ActionCompleted` events that don't have a corresponding `ActionScheduled` event, leading to a negative length (panic on `saturating_sub`).
  prevention: The inner accumulator counts scheduled events in a HashSet and removes completed events. The result is a set difference, not a subtraction. No underflow possible.
  test_for_it: `test_no_panic_on_orphan_completed: events = [ActionCompleted { step: 0, action: 7 }] (no scheduled) -> returns empty Vec, no panic`.

### Integration Failure
- failure: The new public accessor is added in a way that breaks the existing `recover_runtime_frame_seed` (e.g., shadowing the private function).
  prevention: The new function has a DIFFERENT name (`pending_actions_from_events` vs `recovered_pending_actions`). The private function is NOT renamed, removed, or shadowed.
  test_for_it: `test_private_function_still_exists: rg "fn recovered_pending_actions" crates/vb_storage/src/recovery/replay/summary.rs` returns at least 1 match.

## Section 4. ATDD Tests

### Happy
- name: `test_pending_actions_from_events_returns_collected_actions_in_set_order`
  given: A `Vec<JournalEvent>` containing 5 `ActionScheduled` and 3 `ActionCompleted` events (the 3 completed match the first 3 scheduled).
  when: `pending_actions_from_events(&events)` is called.
  then: Returns a `Vec<RecoveredPendingAction>` of length 2 (the 2 uncompleted actions).
  real_input: Build a journal with 8 events: `[ActionScheduled { step: 0, action: 1 }, ActionScheduled { step: 1, action: 2 }, ActionScheduled { step: 2, action: 3 }, ActionCompleted { step: 0 }, ActionCompleted { step: 1 }, ActionCompleted { step: 2 }, ActionScheduled { step: 3, action: 4 }, ActionScheduled { step: 4, action: 5 }]`.
  expected_output: `Vec` of length 2 containing `RecoveredPendingAction { step: 3, action: 4 }` and `RecoveredPendingAction { step: 4, action: 5 }` (or vice versa; HashSet order).
- name: `test_pending_actions_from_events_empty_input`
  given: `events = []`.
  when: `pending_actions_from_events(&events)` is called.
  then: Returns `Vec::new()`.
  real_input: `&[]`.
  expected_output: `Vec::new()`.

### Error
- name: `test_pending_actions_from_events_only_terminal_events`
  given: `events = [RunFinished, RunFailedEvent, RunCancelled]`.
  when: `pending_actions_from_events(&events)` is called.
  then: Returns `Vec::new()` (no scheduled actions, so no pending).
  real_input: `&[JournalEvent::RunFinished { ... }, JournalEvent::RunFailedEvent { ... }, JournalEvent::RunCancelled { ... }]`.
  expected_output: `Vec::new()`.
- name: `test_pending_actions_from_events_orphan_completed_event`
  given: `events = [ActionCompleted { step: 0, action: 7 }]` (no preceding scheduled).
  when: `pending_actions_from_events(&events)` is called.
  then: Returns `Vec::new()` (no panic on orphan completed).
  real_input: `&[JournalEvent::ActionCompleted { ... }]`.
  expected_output: `Vec::new()`.

### Edge
- name: `test_pending_actions_from_events_all_scheduled_no_completed`
  given: `events = [ActionScheduled { step: 0, action: 1 }, ActionScheduled { step: 1, action: 2 }, ActionScheduled { step: 2, action: 3 }]`.
  when: `pending_actions_from_events(&events)` is called.
  then: Returns a `Vec` of length 3.
  real_input: 3 scheduled, 0 completed.
  expected_output: `Vec` of length 3.
- name: `test_pending_actions_from_events_all_completed_no_pending`
  given: `events = [ActionScheduled { step: 0, action: 1 }, ActionCompleted { step: 0 }]`.
  when: `pending_actions_from_events(&events)` is called.
  then: Returns `Vec::new()`.
  real_input: 1 scheduled, 1 completed, 0 pending.
  expected_output: `Vec::new()`.

### Contract
- name: `test_precondition_events_slice_can_be_empty`
  verifies: Precondition "`events` can be empty".
  test: `pending_actions_from_events(&[])` returns `Vec::new()`.
- name: `test_postcondition_return_length_equals_scheduled_minus_completed`
  verifies: Postcondition "return length = scheduled - completed".
  test: proptest with random event sequences; assert the invariant.
- name: `test_invariant_function_is_pure`
  verifies: Invariant "no side effects".
  test: Call `pending_actions_from_events(&events)` twice with the same input; assert the results are equal. (Pure function; deterministic output.)

## Section 5. E2E Tests

```
pipeline_test:
  name: test_recover_pending_actions_via_real_journal
  description: Real FjallJournal, real recovery path; submit 10 actions, complete 7, recover; assert 3 pending.
  setup:
    - open a real FjallJournal in a tempdir
    - submit a run with 10 ActionScheduled journal events
    - complete 7 of them by appending ActionCompleted events
    - keep 3 in the "scheduled but not completed" state
  execute:
    - Call pending_actions_from_events(&all_events)
    - Assert the result has exactly 3 entries
  verify:
    - len(result) == 3
    - each entry has valid StepIdx and ActionId
  cleanup:
    - close FjallJournal
    - delete tempdir

e2e_scenarios:
  - name: e2e_real_journal_pending_actions_count
    description: prove the count invariant holds on a real journal
    steps:
      - submit 10 actions
      - complete 7
      - recover pending count
      - verify == 3
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] All research_requirements files have been read"
    - "[x] All research_questions have answers"
    - "[x] Anti-hallucination guards confirmed (no ActionTicket, no new trait, no rename)"
  evidence_required:
    - "Research notes file with line-numbered extracts"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All 5 acceptance tests written (2 happy, 2 error, 2 edge, 3 contract = 9 tests)"
    - "[ ] All tests fail with compile error (function does not exist yet)"
  evidence_required:
    - "Test file in `crates/vb_storage/src/recovery_unit_tests.rs`"
    - "Compile error output"

### Implementation
- name: "Implementation Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] All 9 tests pass"
    - "[ ] No unwrap() or expect() in new code"
    - "[ ] moon run :ci passes"
  evidence_required:
    - "Test output"
    - "CI green"

### Integration
- name: "Integration Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] E2E test passes with real FjallJournal"
    - "[ ] No regressions in existing storage tests"
  evidence_required:
    - "E2E output"
    - "Manual verification"

## Section 6. Implementation Tasks

### Phase 0: Research
- [ ] Read `summary.rs:814-821` (parallel: research)
- [ ] Read `types.rs:290-297` and `types.rs:377-396` (parallel: research)
- [ ] Read `recover.rs:251-260` (parallel: research)
- [ ] Read `runtime/mod.rs:343-362` (parallel: research)
- [ ] Confirm the inner accumulator function exists (parallel: research)

### Phase 1: Tests
- [ ] Write `test_pending_actions_from_events_returns_collected_actions_in_set_order` (parallel: tests)
- [ ] Write `test_pending_actions_from_events_empty_input` (parallel: tests)
- [ ] Write `test_pending_actions_from_events_only_terminal_events` (parallel: tests)
- [ ] Write `test_pending_actions_from_events_orphan_completed_event` (parallel: tests)
- [ ] Write `test_pending_actions_from_events_all_scheduled_no_completed` (parallel: tests)
- [ ] Write `test_pending_actions_from_events_all_completed_no_pending` (parallel: tests)
- [ ] Write 3 contract tests (parallel: tests)
- [ ] Confirm all 9 tests fail with "function not found" (gate)

### Phase 2: Implementation
- [ ] Add `pub fn pending_actions_from_events` to `summary.rs` (after line 821) (depends: tests; sequential)
- [ ] The function calls `recover_pending_actions_from_events_inner(events)` and then `recovered_pending_actions(accumulator.pending_actions)` (depends: function decl; sequential)
- [ ] Confirm all 9 tests pass (gate: green)

### Phase 3: Integration
- [ ] Write the E2E test in `crates/workspace_tests/tests/` (depends: impl; sequential)
- [ ] Run the E2E test (sequential)
- [ ] Run `cargo test -p vb_storage` to confirm no regressions (sequential)

### Phase 4: Documentation
- [ ] Run `moon run :ci` (depends: all of the above; parallel)
- [ ] Close the bead with reason (sequential)

## Section 7. Failure Modes

- symptom: "Compile error: cannot find function `pending_actions_from_events`"
  likely_cause: Test was written before the function was added.
  where_to_look:
    - file: `crates/vb_storage/src/recovery/replay/summary.rs`
    - function: any test referencing `pending_actions_from_events`
    - what_to_check: "Is the function defined with `pub fn`?"
  fix_pattern: Add the function with the documented signature.
- symptom: "Test fails: result length is wrong (off by one)"
  likely_cause: The inner accumulator is counting `ActionScheduled` and `ActionCompleted` incorrectly (e.g., counting a related variant like `ActionScheduledTicket`).
  where_to_look:
    - file: `crates/vb_storage/src/recovery/replay/summary.rs`
    - function: `recover_pending_actions_from_events_inner`
    - what_to_check: "Are both `ActionScheduled` AND `ActionScheduledTicket` counted as scheduled?"
  fix_pattern: Decide which variant the journal emits (likely `ActionScheduled` for non-batch and `ActionScheduledTicket` for batch). Count both. Or check the production code at `chunk_002.rs:294-315` to see which one is emitted.
- symptom: "Test fails: hash order is non-deterministic across runs"
  likely_cause: The HashSet iteration order is unstable in some std versions. The test should not depend on order; use a sorted comparison or `HashSet` equality.
  where_to_look:
    - file: `crates/vb_storage/src/recovery_unit_tests.rs`
    - function: any test asserting the order of pending_actions
    - what_to_check: "Does the test use `assert_eq!` on Vecs in order, or does it use a set comparison?"
  fix_pattern: Use `let mut result = pending_actions_from_events(&events); result.sort_by_key(|a| a.step); assert_eq!(result, expected_sorted);`

debugging_commands:
- scenario: "When the count is off"
  run: "RUST_LOG=vb_storage=trace cargo test -p vb_storage pending_actions_from_events"
  look_for: "Trace log showing scheduled vs completed counts"
- scenario: "When the result is non-deterministic"
  run: "cargo test -p vb_storage pending_actions_from_events -- --nocapture"
  look_for: "Print the result Vec; check if order changes across runs"

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT add a function `recover_pending_actions(events: &[JournalEvent]) -> Vec<ActionTicket>` — wrong name, wrong return type.
- DO NOT use `ActionTicket` as the return type — use `RecoveredPendingAction` (2 fields).
- DO NOT add a new trait method (round-2 over-engineering).
- DO NOT rename the existing private `recovered_pending_actions`.
- DO NOT add a feature gate (the function is pure, no test-util needed).

VERIFY that:
- `RecoveredPendingAction` has 2 fields: `rg "pub struct RecoveredPendingAction" crates/vb_storage/src/recovery/types.rs` (must find it; check line 290-297).
- `recovered_pending_actions` is private: `rg "fn recovered_pending_actions" crates/vb_storage/src/recovery/replay/summary.rs` (must return exactly 1 match, no `pub`).
- The function signature uses `&[JournalEvent]`: `rg "fn pending_actions_from_events" crates/vb_storage/src/recovery/replay/summary.rs` (must find exactly 1 match after impl).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    rg "pending_actions_from_events" crates/  # confirm the new public accessor is wired

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-av1y0/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-av1y0/progress.txt` and continue from "Current Task". The inner-accumulator function name (`recover_pending_actions_from_events_inner`) is critical to the delegation pattern.
Key invariants:
- The new function is a PUBLIC wrapper that delegates to the PRIVATE `recovered_pending_actions`.
- The return type is `Vec<RecoveredPendingAction>` (2 fields), NOT `Vec<ActionTicket>`.
- The function lives in `summary.rs`, not `recover.rs`.

## Section 8. Completion Checklist

- [ ] All 9 acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing with real FjallJournal
- [ ] No mocks or fake data in any test
- [ ] moon run :ci passes
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] No production code touched outside `crates/vb_storage/src/recovery/replay/summary.rs` (and the test file)
- [ ] bd close with reason: "P0-5b2 complete: pending_actions_from_events public accessor added"

## Section 9. Context

Related files:
- `crates/vb_storage/src/recovery/replay/summary.rs:814-821` — the private `recovered_pending_actions` to delegate to
- `crates/vb_storage/src/recovery/types.rs:290-297` — `RecoveredPendingAction` struct (the return type)
- `crates/vb_storage/src/recovery/types.rs:377-396` — `RecoveryFrameSeed` (which already exposes pending_actions)
- `crates/vb_storage/src/recovery/recover.rs:251-260` — `recover_runtime_frame_seed` (the public function that already returns pending_actions)
- `crates/vb_runtime/src/runtime/mod.rs:343-362` — `Runtime::recover` (which does NOT call `recover_runtime_frame_seed` directly; this is a separate gap, addressed by P0-5a)

Similar implementations:
- `recover_runtime_frame_seed` (public, returns `RecoveryFrameSeed` with `pending_actions` field) — same data, different shape. The new accessor returns ONLY the pending_actions subset for cases where the full seed is not needed.

Codebase patterns:
- pattern: "Public accessor delegating to a private function"
  example_location: `crates/vb_storage/src/recovery/replay/summary.rs:814-821` (private `recovered_pending_actions`)
  how_to_apply: Add a `pub fn` that takes the same input but returns a different shape, delegating to the private function for the heavy lifting.

## Section 10. AI Hints

### DO
- Read `crates/vb_storage/src/recovery/replay/summary.rs:814-821` BEFORE writing any code. The private function is 8 lines; the read is fast.
- Add the new function immediately after the private `recovered_pending_actions` (at line 821 or just after).
- Use `pub fn` for the new accessor so it can be called from outside the module.
- Document the function with a doc comment explaining its purpose (test/observability use).
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT use `unwrap()` or `expect()`.
- Do NOT add a new trait method.
- Do NOT rename the existing private function.
- Do NOT use `ActionTicket` as the return type.
- Do NOT use `unsafe`.
- Do NOT modify clippy configuration.

### Code patterns
- name: "Public accessor delegating to private function"
  use_when: "Adding a new public API for an existing private helper"
  example: |
    /// Public accessor for tests and observability.
    /// Delegates to the private `recovered_pending_actions`.
    /// Use `recover_runtime_frame_seed` to get the full seed including pending_actions.
    pub fn pending_actions_from_events(events: &[JournalEvent]) -> Vec<RecoveredPendingAction> {
        let accumulator = recover_pending_actions_from_events_inner(events);
        recovered_pending_actions(accumulator.pending_actions)
    }

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect().
- Test first: Tests MUST exist and FAIL before implementation.
- Read before write: ALWAYS read the source file before modifying it.
- Real data only: Use real JournalEvent variants; no fabricated placeholders.
- Minimal change: ONE new public function; do NOT refactor the recovery crate.
