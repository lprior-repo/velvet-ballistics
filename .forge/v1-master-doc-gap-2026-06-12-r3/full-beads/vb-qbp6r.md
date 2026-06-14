# P0-5a recover-frame-seed-wire

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> `crates/vb_runtime/src/runtime/mod.rs` (lines 343-365), `crates/vb_storage/src/recovery/recover.rs` (line 251-260),
> `crates/vb_storage/src/recovery/types.rs` (lines 281-288, 290-297, 377-396, 266-279, 300-310, 339-346).

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications (round-2 corrections applied):
- The real signature of `recover_runtime_frame_seed` is `(journal: &FjallJournal, run: RunId) -> RecoveryResult<RecoveryFrameSeed>`, NOT `(events: &[JournalEvent])`.
- The `RecoveryFrameSeed` struct has NO top-level `taint` field. Taint is per-slot on `RecoveredSlotEntry.taint: Taint` (types.rs:281-288).
- `Runtime::recover` returns `Vec<RunId>`, NOT a single `RunFrame`.
- The `#[cfg(feature = "test-util")]` gate MUST stay (round-2 incorrectly claimed to drop it; it must stay gated for production builds).
- This bead has NO dependencies (round-2 had a P2-15r dep which was a P0→P2 inversion; removed).

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL wire `Runtime::recover` to call `recover_runtime_frame_seed(journal, run)` for each hydration.
- THE SYSTEM SHALL preserve the existing `#[cfg(feature = "test-util")]` gate on `Runtime::recover`.
- THE SYSTEM SHALL return `Vec<RunId>` (NOT `RunFrame`).

### Event-Driven
- WHEN `Runtime::recover` is called with a `SharedRuntimeJournal` that has N non-terminal runs, THE SYSTEM SHALL call `recover_runtime_frame_seed` once per run and return `Vec<RunId>` of length <= N.
- WHEN `recover_runtime_frame_seed` returns `Err(...)` for a specific run, THE SYSTEM SHALL skip that run and continue with the others (graceful degradation).

### Unwanted
- THE SYSTEM SHALL NOT use `recover_runtime_frame_seed(events)` — the real signature is `(journal, run)`.
- THE SYSTEM SHALL NOT assert `recovered.taint == original.taint` — taint is per-slot on `RecoveredSlotEntry.taint`.
- THE SYSTEM SHALL NOT claim `Runtime::recover` returns a `RunFrame` — it returns `Vec<RunId>`.
- THE SYSTEM SHALL NOT drop the `#[cfg(feature = "test-util")]` gate.

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs:
  - field: `journal`
    type: `&crate::journal::SharedRuntimeJournal`
    constraints: must be a valid journal with at least one non-terminal run.
    example_valid: `SharedRuntimeJournal::new(test_journal)`
    example_invalid: N/A (the type enforces validity)
  - field: `&mut self`
    type: `&mut Runtime`
    constraints: `Runtime` must be initialized; the `&mut` is required because the recovery may modify internal state.
    example_valid: a freshly constructed `Runtime`
- system_state:
  - `recover_runtime_frame_seed` exists at `crates/vb_storage/src/recovery/recover.rs:251-260` with signature `(journal: &FjallJournal, run: RunId) -> RecoveryResult<RecoveryFrameSeed>`.
  - `recover_all_incomplete_runs` returns `Vec<RecoveryHydration>`.
  - `RecoveredSlotEntry.taint: Taint` exists at `types.rs:281-288`.

### Postconditions
- state_changes:
  - Each successfully recovered run is added to `self`'s active runs (or the equivalent internal state).
- return_guarantees:
  - field: `RuntimeResult<Vec<RunId>>`
    guarantee: `Ok(Vec<RunId>)` on success; the Vec has length <= the number of hydrations.
    guarantee: `Err(RuntimeError::InvalidRecoveryHydration)` on storage failure.
- side_effects: The internal state of `self` is mutated to include the recovered runs.

### Invariants
- The `#[cfg(feature = "test-util")]` gate is preserved.
- The return type is `Vec<RunId>`, not `RunFrame`.
- For each hydration, `recover_runtime_frame_seed` is called with the correct `(journal, run)` pair.
- Taint assertions (in tests) are per-slot, not top-level.

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: `crates/vb_runtime/src/runtime/mod.rs:343-362`
  what_to_extract: The current `Runtime::recover` implementation. Confirm the `#[cfg(feature = "test-util")]` gate, the `&mut self` receiver, the `&SharedRuntimeJournal` parameter, and the `Vec<RunId>` return type.
  document_in: research_notes.md
- path: `crates/vb_storage/src/recovery/recover.rs:251-260`
  what_to_extract: The public `recover_runtime_frame_seed` signature: `(journal: &FjallJournal, run: RunId) -> RecoveryResult<RecoveryFrameSeed>`.
  document_in: research_notes.md
- path: `crates/vb_storage/src/recovery/types.rs:377-396`
  what_to_extract: The `RecoveryFrameSeed` struct fields. Confirm: `summary, first_step, step_count, slot_count, pc, steps: Vec<RecoveredStepEntry>, slots: Vec<RecoveredSlotEntry>, pending_actions: Vec<RecoveredPendingAction>, unsupported: UnsupportedRecoveryState`. NO top-level `taint`.
  document_in: research_notes.md
- path: `crates/vb_storage/src/recovery/types.rs:281-288`
  what_to_extract: The `RecoveredSlotEntry.taint: Taint` field. Confirm taint is per-slot.
  document_in: research_notes.md
- path: `crates/vb_storage/src/recovery/types.rs:266-279`
  what_to_extract: The `RecoveredRunAdmission` struct (for RuntimeState.admission reattachment).
  document_in: research_notes.md
- path: `crates/vb_storage/src/recovery/types.rs:300-310, 339-346`
  what_to_extract: The `UnsupportedRecoveryState` struct and its `pending_actions_unsupported` factory. Confirm `pending_actions` is a `bool` flag, not a method or gate.
  document_in: research_notes.md

Patterns to find:
- pattern: `recover_runtime_frame_seed`
  purpose: Locate the public function and confirm its signature.
  expected_locations: `crates/vb_storage/src/recovery/recover.rs:251-260`.
- pattern: `Runtime::recover`
  purpose: Locate the function to modify.
  expected_locations: `crates/vb_runtime/src/runtime/mod.rs:343-362`.

Prior art:
- feature: existing `recover_all_incomplete_runs`
  location: `crates/vb_storage/src/recovery/recover.rs`
  what_to_learn: The pattern of returning `Vec<RecoveryHydration>` and iterating.

External docs:
- url: master doc (search for "recover_runtime_frame_seed")
  section: recovery flow
  extract: confirm the call pattern.

Research questions (all answered):
- Q: What is the signature? A: `(journal: &FjallJournal, run: RunId) -> RecoveryResult<RecoveryFrameSeed>`.
- Q: Is taint top-level? A: No, per-slot.
- Q: Is the gate preserved? A: Yes (`#[cfg(feature = "test-util")]`).
- Q: Does the bead have dependencies? A: No (round-2 P2-15r dep was a P0→P2 inversion; removed).

Research complete when:
- [x] All files_to_read opened.
- [x] All patterns_to_find searched.
- [x] All prior_art examined.
- [x] All research_questions have answers.

## Section 3. Inversions

### Security
- failure: An attacker corrupts the journal such that `recover_runtime_frame_seed` returns a `RecoveryFrameSeed` with a malicious `artifact_digest`, allowing the runtime to execute a forged workflow.
  prevention: The seed is derived from the journal's blake3 digests; the runtime validates the digest against the stored `compiled_ir` entry. A corrupted journal produces a digest mismatch, which is rejected.
  test_for_it: `test_corrupted_journal_digest_rejected: corrupt the journal; call recover; assert Err(ArtifactNotFound)`.

### Usability
- failure: A developer reads the round-2 bead and tries to use `recover_runtime_frame_seed(events)`, getting a compile error.
  prevention: The signature is HARD-CODED from the source code at `recover.rs:251-260`. The bead's anti-hallucination guard explicitly warns against the wrong signature.
  test_for_it: `test_correct_signature: rg 'recover_runtime_frame_seed' crates/vb_storage/src/recovery/recover.rs` returns the line with the correct signature.

### Data Integrity
- failure: The recovery process silently drops runs that fail to recover, leaving them in a "ghost" state (not active, not terminal).
  prevention: The implementation either returns the run in the `Vec<RunId>` (success) or skips it (graceful degradation with a warning log). The journal still contains the run; a subsequent `recover` call can retry.
  test_for_it: `test_failed_recovery_does_not_corrupt_state: recover with a journal that has 1 good and 1 bad run; assert the good run is recovered and the bad run is logged but not in the Vec`.

### Integration Failure
- failure: A future bead drops the `#[cfg(feature = "test-util")]` gate, causing `Runtime::recover` to be exposed in production builds.
  prevention: The gate is HARD-CODED in the source; the test asserts the gate is present.
  test_for_it: `test_gate_preserved: rg '#\[cfg\(feature = "test-util"\)\]' crates/vb_runtime/src/runtime/mod.rs` returns the line with the gate.

## Section 4. ATDD Tests

### Happy
- name: `test_runtime_recover_returns_reconstructed_frame_with_field_level_parity`
  given: A FjallJournal with a non-terminal run: 10 JournalEvents including 3 SlotWritten events with known slot/taint values.
  when: `Runtime::recover` is called.
  then:
    - `recovered.slots.len() == 3`
    - `recovered.slots[0].slot == SlotIdx::new(0)`
    - `recovered.slots[0].taint == Taint::Clean`
    - `recovered.slots[1].taint == Taint::DerivedFromSecret`
    - `recovered.pc == StepIdx::new(5)`
    - `recovered.steps.len() == 4`
    - `recovered.pending_actions.len() == 1`
    - `recovered.unsupported.is_fully_supported() == true`
  real_input: a journal with 10 events (3 SlotWritten, 4 step events, 1 pending action, 2 terminal markers).
  expected_output: a `Vec<RunId>` of length 1; the recovered run matches the journal state.
- name: `test_runtime_recover_with_multiple_runs`
  given: A journal with 3 non-terminal runs.
  when: `Runtime::recover` is called.
  then: Returns `Vec<RunId>` of length 3.
  real_input: 3 runs, each with 5+ events.
  expected_output: `Vec<RunId>` of length 3.

### Error
- name: `test_runtime_recover_with_empty_journal`
  given: An empty journal (no non-terminal runs).
  when: `Runtime::recover` is called.
  then: Returns `Ok(Vec::new())`.
  real_input: empty journal.
  expected_output: `Ok(vec![])`.
- name: `test_runtime_recover_with_corrupted_journal`
  given: A journal with a corrupted event (CRC mismatch).
  when: `Runtime::recover` is called.
  then: Returns `Err(InvalidRecoveryHydration)`.
  real_input: corrupted journal.
  expected_error: `Err(RuntimeError::InvalidRecoveryHydration)`.
- name: `test_runtime_recover_with_fully_supported_state`
  given: A journal where all state is fully recoverable.
  when: `Runtime::recover` is called.
  then: `recovered.unsupported.is_fully_supported() == true`.
  real_input: a journal with no unsupported state.
  expected_output: `is_fully_supported() == true`.

### Edge
- name: `test_runtime_recover_preserves_test_util_gate`
  given: A production build (without `test-util` feature).
  when: The code is compiled.
  then: `Runtime::recover` is NOT exported (the gate hides it).
  real_input: `cargo build --release` (no `test-util`).
  expected: `Runtime::recover` is not in the public API of the production build.
- name: `test_runtime_recover_returns_vec_run_id_not_run_frame`
  given: A successful recovery.
  when: The return value is inspected.
  then: It is `Vec<RunId>`, NOT a `RunFrame`.
  real_input: any successful recovery.
  expected: type signature is `Vec<RunId>`.

### Contract
- name: `test_precondition_journal_must_have_non_terminal_runs`
  verifies: Precondition "journal has at least one non-terminal run (or empty)".
  test: empty journal returns `Ok(vec![])`.
- name: `test_postcondition_recovered_slots_have_per_slot_taint`
  verifies: Postcondition "taint is per-slot on RecoveredSlotEntry.taint".
  test: assert `recovered.slots[0].taint == Taint::Clean` (not `recovered.taint`).
- name: `test_invariant_function_is_gated_under_test_util`
  verifies: Invariant "the function is gated under `#[cfg(feature = "test-util")]`".
  test: production build does not export the function.

## Section 5. E2E Tests

```
pipeline_test:
  name: test_runtime_recover_e2e
  description: Real FjallJournal, real Runtime, real recovery; submit a run, kill the runtime, restart, recover.
  setup:
    - open a real FjallJournal
    - submit a run with 10 events
    - kill the Runtime
    - create a new Runtime
  execute:
    - call new_runtime.recover(&journal)
  verify:
    - returns Ok(Vec<RunId>) of length 1
    - the recovered run matches the submitted run (same id, same state)
  cleanup:
    - close FjallJournal

e2e_scenarios:
  - name: e2e_kill_restart_recover
    description: prove the recovery wire works across a kill/restart cycle
    steps:
      - submit run A
      - kill runtime
      - new runtime
      - recover
      - verify run A is recovered
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] `runtime/mod.rs:343-362` read"
    - "[x] `recover.rs:251-260` read (signature confirmed: (journal, run))"
    - "[x] `types.rs:281-288, 290-297, 377-396` read (taint is per-slot, no top-level taint)"
    - "[x] Round-2 errors documented (wrong signature, weak test, wrong gate claim)"
  evidence_required:
    - "Research notes file with line-numbered extracts"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All 7 acceptance tests written (2 happy, 3 error, 2 edge, 3 contract)"
    - "[ ] Tests fail with the expected compilation or assertion errors"
  evidence_required:
    - "Test file in `crates/vb_runtime/src/runtime/tests.rs`"
    - "Test output"

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
    - "[ ] E2E test passes with real FjallJournal + real Runtime"
    - "[ ] No regressions in runtime tests"
  evidence_required:
    - "E2E output"
    - "Manual verification"

## Section 6. Implementation Tasks

### Phase 0: Research
- [ ] Read `runtime/mod.rs:343-362` (parallel: research)
- [ ] Read `recover.rs:251-260` (parallel: research)
- [ ] Read `types.rs:281-288, 290-297, 377-396, 266-279, 300-310, 339-346` (parallel: research)
- [ ] Document the round-2 errors and the corrected spec (parallel: research)

### Phase 1: Tests
- [ ] Write `test_runtime_recover_returns_reconstructed_frame_with_field_level_parity` (parallel: tests)
- [ ] Write `test_runtime_recover_with_multiple_runs` (parallel: tests)
- [ ] Write `test_runtime_recover_with_empty_journal` (parallel: tests)
- [ ] Write `test_runtime_recover_with_corrupted_journal` (parallel: tests)
- [ ] Write `test_runtime_recover_with_fully_supported_state` (parallel: tests)
- [ ] Write `test_runtime_recover_preserves_test_util_gate` (parallel: tests)
- [ ] Write `test_runtime_recover_returns_vec_run_id_not_run_frame` (parallel: tests)
- [ ] Write 3 contract tests (parallel: tests)
- [ ] Confirm all 7 tests fail (gate)

### Phase 2: Implementation
- [ ] Modify `Runtime::recover` at `runtime/mod.rs:343-365` to call `recover_runtime_frame_seed(journal, run)` for each hydration (depends: tests; sequential)
- [ ] Preserve the `#[cfg(feature = "test-util")]` gate (depends: modify; sequential)
- [ ] Preserve the `Vec<RunId>` return type (depends: modify; sequential)
- [ ] Add graceful degradation: if `recover_runtime_frame_seed` returns Err for a specific run, log a warning and continue (depends: modify; sequential)
- [ ] Confirm all 7 tests pass (gate: green)

### Phase 3: Integration
- [ ] Write the E2E test (depends: impl; sequential)
- [ ] Run the E2E test (sequential)
- [ ] Run `cargo test -p vb_runtime` to confirm no regressions (sequential)

### Phase 4: Documentation
- [ ] Run `moon run :ci` (depends: all of the above; parallel)
- [ ] Close the bead (sequential)

## Section 7. Failure Modes

- symptom: "Compile error: argument types mismatch in `recover_runtime_frame_seed` call"
  likely_cause: The wrong signature is used (`(events)` instead of `(journal, run)`).
  where_to_look:
    - file: `crates/vb_runtime/src/runtime/mod.rs:343-365`
    - function: `Runtime::recover`
    - what_to_check: "Is the call `recover_runtime_frame_seed(&storage_journal, hydration.run)` (not `recover_runtime_frame_seed(&events)`)?"
  fix_pattern: Use the correct signature from `recover.rs:251-260`.
- symptom: "Test fails: assertion on `recovered.taint` (top-level) fails"
  likely_cause: The test asserts on a non-existent top-level `taint` field. Taint is per-slot.
  where_to_look:
    - file: `crates/vb_runtime/src/runtime/tests.rs`
    - function: any test asserting `recovered.taint`
    - what_to_check: "Is the assertion `recovered.slots[i].taint` (per-slot)?"
  fix_pattern: Change to `recovered.slots[i].taint`.
- symptom: "Test fails: production build exports `Runtime::recover`"
  likely_cause: The `#[cfg(feature = "test-util")]` gate was accidentally dropped.
  where_to_look:
    - file: `crates/vb_runtime/src/runtime/mod.rs:343`
    - what_to_check: "Is the `#[cfg(feature = "test-util")]` attribute present?"
  fix_pattern: Add the gate back.

debugging_commands:
- scenario: "When the signature is wrong"
  run: "rg 'recover_runtime_frame_seed' crates/vb_storage/src/recovery/recover.rs"
  look_for: "Confirm the signature: (journal: &FjallJournal, run: RunId)"
- scenario: "When the gate is missing"
  run: "rg '#\\[cfg\\(feature = \"test-util\"\\)\\]' crates/vb_runtime/src/runtime/mod.rs"
  look_for: "The gate should be on the line above `pub fn recover`"

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT use `recover_runtime_frame_seed(events)` — the real signature is `(journal, run)`.
- DO NOT assert `recovered.taint == original.taint` — taint is per-slot on `RecoveredSlotEntry.taint`.
- DO NOT claim `Runtime::recover` returns a `RunFrame` — it returns `Vec<RunId>`.
- DO NOT drop the `#[cfg(feature = "test-util")]` gate (the round-2 bead claimed to drop it; in fact it must stay gated).
- DO NOT add a P2-15r dependency (round-2 had it; that was a P0→P2 inversion).

VERIFY that:
- `recover_runtime_frame_seed` has signature `(journal, run)`: `rg 'pub fn recover_runtime_frame_seed' crates/vb_storage/src/recovery/recover.rs` (must show the correct signature).
- `RecoveryFrameSeed` has NO top-level `taint`: `rg 'pub struct RecoveryFrameSeed' crates/vb_storage/src/recovery/types.rs` (must show fields; verify no top-level taint).
- `RecoveredSlotEntry` HAS a `taint: Taint` field: `rg 'pub struct RecoveredSlotEntry' crates/vb_storage/src/recovery/types.rs` (must show taint field at line 281-288).
- The `#[cfg(feature = "test-util")]` gate is present: `rg 'pub fn recover' crates/vb_runtime/src/runtime/mod.rs` (must show the gate above).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    rg 'recover_runtime_frame_seed' crates/vb_runtime/src/runtime/mod.rs  # confirm the wire is connected

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-qbp6r/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-qbp6r/progress.txt` and continue from "Current Task". The signature is FIXED; do not change it.
Key invariants:
- The signature of `recover_runtime_frame_seed` is `(journal: &FjallJournal, run: RunId)`.
- Taint is per-slot, NOT top-level.
- The return type of `Runtime::recover` is `Vec<RunId>`, NOT `RunFrame`.
- The `#[cfg(feature = "test-util")]` gate MUST stay.
- This bead has NO dependencies.

## Section 8. Completion Checklist

- [ ] All 7 acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing with real FjallJournal + real Runtime
- [ ] No mocks or fake data in any test
- [ ] moon run :ci passes
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] No production code touched outside `crates/vb_runtime/src/runtime/mod.rs`
- [ ] bd close with reason: "P0-5a complete: Runtime::recover wired to recover_runtime_frame_seed(journal, run)"

## Section 9. Context

Related files:
- `crates/vb_runtime/src/runtime/mod.rs:343-362` — `Runtime::recover` (the function to modify)
- `crates/vb_storage/src/recovery/recover.rs:251-260` — `recover_runtime_frame_seed` (the function to call)
- `crates/vb_storage/src/recovery/types.rs:281-288` — `RecoveredSlotEntry.taint`
- `crates/vb_storage/src/recovery/types.rs:290-297` — `RecoveredPendingAction`
- `crates/vb_storage/src/recovery/types.rs:377-396` — `RecoveryFrameSeed` (no top-level taint)
- `crates/vb_storage/src/recovery/types.rs:266-279` — `RecoveredRunAdmission`
- `crates/vb_storage/src/recovery/types.rs:300-310, 339-346` — `UnsupportedRecoveryState` and `pending_actions_unsupported`

Similar implementations:
- The existing `recover_one_run` at `runtime/mod.rs:367-383` (mentioned in the source read) is the existing recovery helper. This bead extends the loop to call `recover_runtime_frame_seed` for each hydration.

Codebase patterns:
- pattern: "Recovery loop with graceful degradation"
  example_location: `crates/vb_runtime/src/runtime/mod.rs:343-365` (current `recover`)
  how_to_apply: Iterate over hydrations; for each, call `recover_runtime_frame_seed`; on Err, log and continue; on Ok, add to the result Vec.

## Section 10. AI Hints

### DO
- Read `crates/vb_storage/src/recovery/recover.rs:251-260` BEFORE writing any code. The signature is FIXED.
- Use the exact signature `(journal, run)` — NOT `(events)`.
- Preserve the `#[cfg(feature = "test-util")]` gate.
- Preserve the `Vec<RunId>` return type.
- Use per-slot taint assertions in tests: `recovered.slots[i].taint`, NOT `recovered.taint`.
- Add graceful degradation: on Err, log a warning and continue.
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT use `unwrap()` or `expect()`.
- Do NOT use the wrong signature `(events)`.
- Do NOT assert on a non-existent top-level `taint` field.
- Do NOT drop the `#[cfg(feature = "test-util")]` gate.
- Do NOT add a P2-15r dependency.
- Do NOT use `unsafe`.

### Code patterns
- name: "Recovery loop with graceful degradation"
  use_when: "Iterating over a list of items where each may fail independently"
  example: |
    for hydration in hydrations {
        match vb_storage::recovery::recover_runtime_frame_seed(storage_journal, hydration.run) {
            Ok(_seed) => {
                if let Some(run) = self.recover_one_run(journal, hydration)? {
                    recovered.push(run);
                }
            }
            Err(e) => {
                log::warn!("recover failed for run {}: {:?}", hydration.run, e);
                // Graceful degradation: skip this run.
            }
        }
    }

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect().
- Test first: Tests MUST exist and FAIL before implementation.
- Read before write: ALWAYS read the source file before modifying it.
- Real data only: Use real RecoveryFrameSeed, RecoveredSlotEntry, Taint types; no fabricated placeholders.
- Minimal change: ONE function to modify; do NOT refactor the recovery crate.
