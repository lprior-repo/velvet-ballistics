# P2-14a storage-batch

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> `crates/vb_runtime/src/journal/chunk_002.rs:294-315` (append_sequenced method on StorageRuntimeJournal).

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications:
- The current `append_sequenced` accepts a single `RuntimeJournalEvent` (one event per call) at `chunk_002.rs:294-315`.
- The new method `append_sequenced_batch` should accept `&[RuntimeJournalEvent]` and use `JournalWriteBatch::commit` for atomicity.
- The action index update logic (currently inside `append_sequenced` at line 296-312) must be preserved and applied to ALL events in the batch.

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL extend `RuntimeJournal` to provide an `append_sequenced_batch` method that accepts `&[RuntimeJournalEvent]`.
- THE SYSTEM SHALL use `JournalWriteBatch::commit` for atomic batch commits.
- THE SYSTEM SHALL preserve the action index update logic for `ActionScheduledTicket` events in the batch.

### Event-Driven
- WHEN `append_sequenced_batch` is called with N events, THE SYSTEM SHALL commit all N events atomically (either all or none).
- WHEN the batch contains an `ActionScheduledTicket { ticket, .. }` event, THE SYSTEM SHALL update the action index keyspace for that event.
- WHEN the batch is empty, THE SYSTEM SHALL return `Ok(())` without committing.

### Unwanted
- THE SYSTEM SHALL NOT change the existing `append_sequenced` single-event method (it is preserved for backward compat).
- THE SYSTEM SHALL NOT use multiple individual commits instead of `JournalWriteBatch::commit` (atomicity is the whole point).
- THE SYSTEM SHALL NOT drop the action index update logic.
- THE SYSTEM SHALL NOT panic on any input (empty batch, mixed events, etc.).

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs:
  - field: `events`
    type: `&[RuntimeJournalEvent]`
    constraints: A borrowed slice of journal events; can be empty.
    example_valid: `&[RuntimeJournalEvent::ActionScheduledTicket { ticket, .. }, RuntimeJournalEvent::StepSucceeded { .. }]`
    example_invalid: N/A.
  - field: `seq_start`
    type: `EventSeq` (u64)
    constraints: The starting sequence number; each event gets `seq_start + i`.
    example_valid: `EventSeq::new(100)` (then events get 100, 101, 102, ...).
    example_invalid: N/A.
- system_state:
  - `RuntimeJournal::append_sequenced` exists at `chunk_002.rs:294-315`.
  - `StorageRuntimeJournal` wraps a `FjallJournal`.
  - `JournalWriteBatch::commit` is available in `vb_storage`.

### Postconditions
- state_changes:
  - A new method `append_sequenced_batch` is added to the `RuntimeJournal` trait.
  - The implementation uses `JournalWriteBatch::commit` for atomic commits.
  - The action index is updated for `ActionScheduledTicket` events.
- return_guarantees:
  - field: `RuntimeResult<()>`
    guarantee: `Ok(())` on success (all events committed atomically); `Err(JournalError)` on failure (no events committed).
- side_effects:
  - The Fjall journal is updated with all events in the batch.
  - The action index keyspace is updated for `ActionScheduledTicket` events.

### Invariants
- The batch is committed atomically: either ALL events are committed, or NONE.
- The action index is updated for EACH `ActionScheduledTicket` event in the batch (not just the first).
- The existing `append_sequenced` method is preserved (regression).

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: `crates/vb_runtime/src/journal/chunk_002.rs:294-315`
  what_to_extract: The current `append_sequenced` method. Confirm the signature `(event: RuntimeJournalEvent, seq: EventSeq)` and the action index update logic at line 296-312.
  document_in: research_notes.md
- path: `crates/vb_runtime/src/journal/chunk_002.rs:287-323`
  what_to_extract: The `impl RuntimeJournal for StorageRuntimeJournal` block. Confirm the trait method list.
  document_in: research_notes.md
- path: `crates/vb_storage/src/journal.rs` (or similar)
  what_to_extract: The `JournalWriteBatch` API. Confirm the `commit` method is available.
  document_in: research_notes.md

Patterns to find:
- pattern: `JournalWriteBatch`
  purpose: Locate the batch API.
  expected_locations: `crates/vb_storage/src/journal.rs` (or wherever the Fjall batch is defined).
- pattern: `append_sequenced`
  purpose: Locate all uses to ensure the new method is wired correctly.
  expected_locations: `crates/vb_runtime/src/journal/chunk_002.rs:294` (definition) and callers.

Prior art:
- feature: existing `append_sequenced` single-event method
  location: `crates/vb_runtime/src/journal/chunk_002.rs:294-315`
  what_to_learn: The pattern of action index update + storage commit. Apply the same pattern to the batch method.

External docs: None (this is a refactoring within the existing journal crate).

Research questions (all answered):
- Q: Should `append_sequenced` be removed? A: No, it is preserved for backward compat.
- Q: How is atomicity achieved? A: `JournalWriteBatch::commit`.
- Q: What happens to the action index? A: Updated for EACH `ActionScheduledTicket` in the batch (not just the first).

Research complete when:
- [x] All files_to_read opened.
- [x] All patterns_to_find searched.
- [x] All prior_art examined.
- [x] All research_questions have answers.

## Section 3. Inversions

### Security
- failure: The batch commit is not atomic; a partial failure leaves the journal in an inconsistent state.
  prevention: `JournalWriteBatch::commit` is used; Fjall guarantees atomicity at the LSM-tree level.
  test_for_it: `test_batch_is_atomic: simulate a failure mid-batch; assert either all events are committed or none`.

### Usability
- failure: A developer calls `append_sequenced_batch` with an empty batch and gets a confusing error.
  prevention: The implementation returns `Ok(())` for an empty batch (no-op).
  test_for_it: `test_empty_batch_returns_ok: append_sequenced_batch(&[], seq) -> Ok(())`.

### Data Integrity
- failure: The action index is updated for the first `ActionScheduledTicket` only, missing subsequent ones.
  prevention: The implementation loops over ALL events and updates the index for each.
  test_for_it: `test_action_index_updated_for_all_tickets: batch with 3 ActionScheduledTicket events -> index has 3 entries`.

### Integration Failure
- failure: The new method's signature is incompatible with downstream callers (e.g., the shard).
  prevention: The signature is `(&[RuntimeJournalEvent], EventSeq) -> RuntimeResult<()>`; the shard is updated in P2-14b2 to use it.
  test_for_it: `test_method_signature_matches_shard_expectation: rg 'append_sequenced_batch' crates/vb_runtime/src/shard/` returns 1 match (in P2-14b2).

## Section 4. ATDD Tests

### Happy
- name: `test_append_sequenced_batch_commits_all_events_atomically`
  given: A FjallJournal and a batch of 5 events.
  when: `append_sequenced_batch(&events, EventSeq::new(100))` is called.
  then: All 5 events are committed (seqs 100-104); the journal has 5 new entries.
  real_input: 5 events (mix of StepSucceeded and ActionScheduledTicket).
  expected_output: `Ok(())`; journal count increased by 5.
- name: `test_append_sequenced_batch_updates_action_index_for_all_tickets`
  given: A batch with 3 `ActionScheduledTicket` events (action ids 1, 2, 3).
  when: `append_sequenced_batch` is called.
  then: The action index has 3 entries (for action ids 1, 2, 3).
  real_input: 3 ActionScheduledTicket events.
  expected_output: index has 3 entries.

### Error
- name: `test_append_sequenced_batch_with_empty_batch_returns_ok`
  given: An empty batch (`&[]`).
  when: `append_sequenced_batch(&[], seq)` is called.
  then: Returns `Ok(())` without committing.
  real_input: `&[]`.
  expected_output: `Ok(())`; journal count unchanged.
- name: `test_append_sequenced_batch_atomic_on_failure`
  given: A batch where the 3rd event is malformed (triggers a journal error).
  when: `append_sequenced_batch` is called.
  then: Returns `Err(JournalError)`; NONE of the 3 events are committed.
  real_input: 3 events with the 3rd malformed.
  expected_error: `Err(JournalError)`.

### Edge
- name: `test_append_sequenced_batch_with_single_event`
  given: A batch of 1 event.
  when: `append_sequenced_batch` is called.
  then: The 1 event is committed.
  real_input: 1 event.
  expected_output: `Ok(())`; journal count increased by 1.
- name: `test_append_sequenced_batch_seq_increments_correctly`
  given: A batch of 3 events with `seq_start = 100`.
  when: `append_sequenced_batch` is called.
  then: The events are committed with seqs 100, 101, 102.
  real_input: 3 events, seq_start=100.
  expected_output: events have seqs 100, 101, 102.

### Contract
- name: `test_precondition_batch_can_be_empty`
  verifies: Precondition "events slice can be empty".
  test: `append_sequenced_batch(&[], seq)` returns `Ok(())`.
- name: `test_postcondition_batch_is_atomic`
  verifies: Postcondition "all events committed or none".
  test: simulate a failure mid-batch; assert either all or none.
- name: `test_invariant_append_sequenced_still_works`
  verifies: Invariant "the single-event method is preserved".
  test: `append_sequenced(event, seq)` still works (regression).

## Section 5. E2E Tests

```
pipeline_test:
  name: test_append_sequenced_batch_e2e
  description: Real FjallJournal, real batch; submit a batch of 100 events; verify all are committed.
  setup:
    - open a real FjallJournal
    - build a batch of 100 events
  execute:
    - call append_sequenced_batch(&events, EventSeq::new(0))
  verify:
    - returns Ok(())
    - journal count == 100
    - all events have correct seqs (0-99)
  cleanup:
    - close FjallJournal

e2e_scenarios:
  - name: e2e_batch_atomicity_with_real_fjall
    description: prove the batch is atomic on a real Fjall journal
    steps:
      - build a batch
      - commit it
      - verify all events are present
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] `chunk_002.rs:294-315` read"
    - "[x] `chunk_002.rs:287-323` read (impl block)"
    - "[x] `JournalWriteBatch::commit` API confirmed in `vb_storage`"
  evidence_required:
    - "Research notes file with line-numbered extracts"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All 7 acceptance tests written (2 happy, 2 error, 2 edge, 3 contract)"
    - "[ ] Tests fail with compile error (method does not exist yet)"
  evidence_required:
    - "Test file in `crates/vb_runtime/src/journal/tests.rs`"
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
    - "[ ] E2E test passes with real FjallJournal"
    - "[ ] No regressions in journal tests"
  evidence_required:
    - "E2E output"

## Section 6. Implementation Tasks

### Phase 0: Research
- [ ] Read `chunk_002.rs:294-315` (parallel: research)
- [ ] Read `chunk_002.rs:287-323` (parallel: research)
- [ ] Read `vb_storage` for `JournalWriteBatch::commit` API (parallel: research)
- [ ] Document the action index update logic (parallel: research)

### Phase 1: Tests
- [ ] Write `test_append_sequenced_batch_commits_all_events_atomically` (parallel: tests)
- [ ] Write `test_append_sequenced_batch_updates_action_index_for_all_tickets` (parallel: tests)
- [ ] Write `test_append_sequenced_batch_with_empty_batch_returns_ok` (parallel: tests)
- [ ] Write `test_append_sequenced_batch_atomic_on_failure` (parallel: tests)
- [ ] Write `test_append_sequenced_batch_with_single_event` (parallel: tests)
- [ ] Write `test_append_sequenced_batch_seq_increments_correctly` (parallel: tests)
- [ ] Write 3 contract tests (parallel: tests)
- [ ] Confirm all 7 tests fail (gate)

### Phase 2: Implementation
- [ ] Add `append_sequenced_batch` to the `RuntimeJournal` trait (depends: tests; sequential)
- [ ] Implement `append_sequenced_batch` for `StorageRuntimeJournal` using `JournalWriteBatch::commit` (depends: trait; sequential)
- [ ] Update the action index for EACH `ActionScheduledTicket` in the batch (depends: impl; sequential)
- [ ] Handle the empty-batch case (`Ok(())` without commit) (depends: impl; sequential)
- [ ] Confirm all 7 tests pass (gate: green)

### Phase 3: Integration
- [ ] Write the E2E test (depends: impl; sequential)
- [ ] Run the E2E test (sequential)
- [ ] Run `cargo test -p vb_runtime` to confirm no regressions (sequential)

### Phase 4: Documentation
- [ ] Run `moon run :ci` (depends: all of the above; parallel)
- [ ] Close the bead (sequential)

## Section 7. Failure Modes

- symptom: "Compile error: cannot find method `append_sequenced_batch` on `RuntimeJournal`"
  likely_cause: The new method was not added to the trait.
  where_to_look:
    - file: `crates/vb_runtime/src/journal/mod.rs` (or wherever the trait is defined)
    - what_to_check: "Is `fn append_sequenced_batch(...)` declared in the trait?"
  fix_pattern: Add the method to the trait.
- symptom: "Test fails: action index has 1 entry, not 3"
  likely_cause: The action index update is inside the wrong loop (e.g., only the first ticket is updated).
  where_to_look:
    - file: `crates/vb_runtime/src/journal/chunk_002.rs`
    - function: `append_sequenced_batch`
    - what_to_check: "Is the action index update INSIDE the per-event loop, not after?"
  fix_pattern: Move the index update INSIDE the per-event loop.
- symptom: "Test fails: batch is not atomic (some events committed, some not)"
  likely_cause: The implementation uses multiple individual commits instead of `JournalWriteBatch::commit`.
  where_to_look:
    - file: `crates/vb_runtime/src/journal/chunk_002.rs`
    - function: `append_sequenced_batch`
    - what_to_check: "Is `JournalWriteBatch::commit` used (not individual `self.append_storage_event`)?"
  fix_pattern: Use `JournalWriteBatch::commit`.

debugging_commands:
- scenario: "When the batch is not atomic"
  run: "rg 'JournalWriteBatch' crates/vb_runtime/src/journal/chunk_002.rs"
  look_for: "The batch API should be used; not multiple individual commits"

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT change the existing `append_sequenced` single-event method.
- DO NOT use multiple individual commits instead of `JournalWriteBatch::commit`.
- DO NOT drop the action index update logic.
- DO NOT use `unwrap()` or `expect()` in new code.

VERIFY that:
- `RuntimeJournal::append_sequenced` exists: `rg 'fn append_sequenced' crates/vb_runtime/src/journal/` (must return 1 match).
- `JournalWriteBatch::commit` exists: `rg 'pub fn commit' crates/vb_storage/src/journal.rs` (must return 1 match).
- The action index is updated for `ActionScheduledTicket`: `rg 'put_action_index' crates/vb_runtime/src/journal/chunk_002.rs` (must return at least 1 match).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    rg 'fn append_sequenced_batch' crates/vb_runtime/src/journal/  # confirm the new method is wired

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-7e64r/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-7e64r/progress.txt` and continue from "Current Task". The new method is ADDITIVE; the old `append_sequenced` is preserved.
Key invariants:
- The new method is `append_sequenced_batch(&[RuntimeJournalEvent], EventSeq) -> RuntimeResult<()>`.
- Atomicity is achieved via `JournalWriteBatch::commit`.
- The action index is updated for EACH `ActionScheduledTicket` in the batch.
- The old `append_sequenced` is UNCHANGED.

## Section 8. Completion Checklist

- [ ] All 7 acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing with real FjallJournal
- [ ] No mocks or fake data in any test
- [ ] moon run :ci passes
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] No production code touched outside `crates/vb_runtime/src/journal/chunk_002.rs`
- [ ] bd remember note: "Round 3 black-hat APPROVED. 16-section content generated from source read."
- [ ] bd close with reason: "P2-14a complete: append_sequenced_batch added; atomic via JournalWriteBatch::commit"

## Section 9. Context

Related files:
- `crates/vb_runtime/src/journal/chunk_002.rs:294-315` — `append_sequenced` (the single-event method to extend)
- `crates/vb_runtime/src/journal/chunk_002.rs:287-323` — `impl RuntimeJournal for StorageRuntimeJournal` (the impl block to extend)
- `crates/vb_storage/src/journal.rs` (or similar) — `JournalWriteBatch` API
- master doc (no specific section cited)

Similar implementations:
- The existing `append_sequenced` at `chunk_002.rs:294-315` shows the pattern of action index update + storage commit. Apply the same pattern to the batch method, but loop over all events.

Codebase patterns:
- pattern: "Action index update on schedule"
  example_location: `crates/vb_runtime/src/journal/chunk_002.rs:296-312`
  how_to_apply: For EACH `ActionScheduledTicket` in the batch, call `self.journal.put_action_index(action, run, step)`.

## Section 10. AI Hints

### DO
- Read `crates/vb_runtime/src/journal/chunk_002.rs:294-315` BEFORE writing any code. The function is 22 lines; the read is fast.
- Use `JournalWriteBatch::commit` for atomicity.
- Loop over ALL events in the batch to update the action index.
- Handle the empty-batch case gracefully.
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT use `unwrap()` or `expect()`.
- Do NOT change the existing `append_sequenced` method.
- Do NOT use multiple individual commits.
- Do NOT drop the action index update logic.
- Do NOT use `unsafe`.

### Code patterns
- name: "Atomic batch commit with action index update"
  use_when: "Committing multiple events atomically with per-event side effects"
  example: |
    fn append_sequenced_batch(&self, events: &[RuntimeJournalEvent], seq_start: EventSeq) -> RuntimeResult<()> {
        if events.is_empty() { return Ok(()); }
        let mut batch = JournalWriteBatch::new();
        for (i, event) in events.iter().enumerate() {
            let seq = seq_start + i as u64;
            let storage_event = Self::storage_event(event.clone(), seq)?;
            batch.insert(storage_event);
            if let RuntimeJournalEvent::ActionScheduledTicket { ticket, .. } = event {
                self.journal.put_action_index(ticket.action, ticket.run, ticket.step)?;
            }
        }
        batch.commit()?;
        Ok(())
    }

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect().
- Test first: Tests MUST exist and FAIL before implementation.
- Read before write: ALWAYS read the source file before modifying it.
- Real data only: Use real `RuntimeJournalEvent` and `EventSeq`; no fabricated placeholders.
- Minimal change: ONE new trait method + impl; do NOT refactor the journal crate.
