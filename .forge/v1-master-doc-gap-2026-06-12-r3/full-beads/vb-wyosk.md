# P2-15r index-status-workflow

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> `crates/vb_storage/src/indexes.rs` (lines 1-68), `crates/vb_storage/src/tests/chunk_032.rs` (lines 1-213),
> `crates/vb_storage/src/recovery/types.rs` (lines 300-310, 339-346), `crates/vb_storage/src/admission.rs` (lines 230-310).

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications (round-2 corrections applied):
- This is an AUDIT bead, not a "wire vs remove" binary decision. The actual scope is to determine whether `index_status` and `index_workflow` Fjall keyspaces are populated during submit/recover.
- There is NO `pending_actions` API or gate — only `UnsupportedRecoveryState::pending_actions: bool` (a flag, not a method).
- Master doc §44.15 does NOT exist; §44 has 24 numbered points. The "operational affordances" framing is INVENTED.
- This bead has NO dependencies (round-2 had vb-v1jiq (P0-5b) depending on this — a P0→P2 inversion; removed).

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL audit whether `submit_artifact` calls `put_status_index` and `put_workflow_index` for new runs.
- THE SYSTEM SHALL audit whether `recover_runtime_frame_seed` rebuilds the index entries from the journal.
- THE SYSTEM SHALL record the audit result in a `bd remember` note.

### Event-Driven
- WHEN the audit finds the keyspaces ARE populated (likely), THE SYSTEM SHALL close the bead with the audit result.
- WHEN the audit finds the keyspaces are NOT populated, THE SYSTEM SHALL add the missing `put_status_index` and `put_workflow_index` calls.
- WHEN the keyspaces are populated but the index entries are stale (e.g., wrong timestamp), THE SYSTEM SHALL add a refresh call.

### Unwanted
- THE SYSTEM SHALL NOT cite "master §44.15" or "operational affordances" — these do NOT exist in the master doc.
- THE SYSTEM SHALL NOT add a `pending_actions` gate or method — only the `bool` flag exists.
- THE SYSTEM SHALL NOT remove the dead code without audit — the round-2 "wire vs remove" decision is binary, but the actual scope is "audit the indexes".

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs: None (this is an audit bead).
- system_state:
  - `put_status_index` exists at `crates/vb_storage/src/indexes.rs:15-24`.
  - `put_workflow_index` exists at `crates/vb_storage/src/indexes.rs:27-35`.
  - `index_status` is a Fjall keyspace (field on `FjallJournal`).
  - `index_workflow` is a Fjall keyspace.
  - `submit_artifact` exists at `crates/vb_storage/src/admission.rs:230-236`.
  - `recover_runtime_frame_seed` exists at `crates/vb_storage/src/recovery/recover.rs:251-260`.

### Postconditions
- state_changes:
  - If the audit finds a gap: a one-line call to `journal.put_status_index(...)` and `journal.put_workflow_index(...)` is added in `submit_artifact_for_policy` at `admission.rs:304`.
  - If the audit finds a gap: a corresponding call is added in `recover_all_incomplete_runs` at `recover.rs:289` per-run.
  - The audit result is recorded in a `bd remember` note.
- return_guarantees:
  - field: `audit result`
    guarantee: One of "keyspaces populated" (close bead) or "keyspaces missing" (implement the fix).
- side_effects: None (read-only audit, plus the fix if needed).

### Invariants
- The `UnsupportedRecoveryState::pending_actions: bool` flag is the ONLY place `pending_actions` is referenced; there is no API or method by that name.
- The audit does NOT change the recovery flow if the keyspaces are already populated.
- If the fix is needed, it is ADDITIVE (one-line call), not a refactor.

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: `crates/vb_storage/src/indexes.rs:15-24`
  what_to_extract: The `put_status_index` method signature: `(state: IndexStatusState, timestamp: u64, run: RunId) -> Result<(), JournalError>`.
  document_in: research_notes.md
- path: `crates/vb_storage/src/indexes.rs:27-35`
  what_to_extract: The `put_workflow_index` method signature: `(workflow: WorkflowId, run: RunId) -> Result<(), JournalError>`.
  document_in: research_notes.md
- path: `crates/vb_storage/src/tests/chunk_032.rs:67-91`
  what_to_extract: The `status_index_multiple_runs_same_state` test. Confirm it verifies `journal.index_status.get(key.as_slice())`.
  document_in: research_notes.md
- path: `crates/vb_storage/src/tests/chunk_032.rs:24-30`
  what_to_extract: The `workflow_index_stores_and_queries_by_workflow_id` test. Confirm it verifies `journal.index_workflow.get(key.as_slice())`.
  document_in: research_notes.md
- path: `crates/vb_storage/src/recovery/types.rs:300-310`
  what_to_extract: The `UnsupportedRecoveryState` struct. Confirm `pending_actions: bool` is a field, not a method.
  document_in: research_notes.md
- path: `crates/vb_storage/src/recovery/types.rs:339-346`
  what_to_extract: The `pending_actions_unsupported` factory. Confirm it returns `Self` with `pending_actions: true`.
  document_in: research_notes.md
- path: `crates/vb_storage/src/admission.rs:230-310`
  what_to_extract: The `submit_artifact` and `submit_artifact_for_policy` functions. Search for `put_status_index` / `put_workflow_index` calls.
  document_in: research_notes.md
- path: `crates/vb_storage/src/recovery/recover.rs:251-296`
  what_to_extract: The `recover_runtime_frame_seed` and `recover_all_incomplete_runs` functions. Search for `put_status_index` / `put_workflow_index` calls.
  document_in: research_notes.md

Patterns to find:
- pattern: `put_status_index`
  purpose: Locate all call sites. The audit must determine if `submit_artifact` and `recover_runtime_frame_seed` call it.
  expected_locations: `crates/vb_storage/src/indexes.rs` (definition) and call sites in `admission.rs` and `recover.rs`.
- pattern: `put_workflow_index`
  purpose: Same as above.
  expected_locations: same.
- pattern: `pending_actions` (as a method, not a field)
  purpose: Verify there is NO method named `pending_actions` — only the `bool` field.
  expected_locations: NONE — there is no method.

Prior art:
- feature: existing `put_status_index` and `put_workflow_index` methods
  location: `crates/vb_storage/src/indexes.rs:15-35`
  what_to_learn: The method signatures and the Fjall keyspace names.

External docs:
- url: master doc §44 (search for "operational affordances")
  section: (does not exist)
  extract: confirm the section does not exist.

Research questions (all answered):
- Q: Is this an audit or a fix? A: Audit FIRST; if a gap is found, fix it.
- Q: Is there a `pending_actions` API? A: No, only a `bool` flag on `UnsupportedRecoveryState`.
- Q: Does master §44.15 exist? A: No.
- Q: Does this bead have dependencies? A: No (round-2 had vb-v1jiq; that was a P0→P2 inversion; removed).

Research complete when:
- [x] All files_to_read opened.
- [x] All patterns_to_find searched.
- [x] All prior_art examined.
- [x] All research_questions have answers.

## Section 3. Inversions

### Security
- failure: The audit concludes "keyspaces are populated" but they are actually stale (e.g., populated by a test but not by `submit_artifact`).
  prevention: The audit reads the production code paths (`admission.rs:230-310` and `recover.rs:251-296`), not just the test paths. The conclusion is based on the production code, not the tests.
  test_for_it: `test_audit_reads_production_code: the audit searches admission.rs and recover.rs, not chunk_032.rs (tests)`.

### Usability
- failure: The audit conclusion is ambiguous (e.g., "the keyspaces MAY be populated"), and the bead is left open.
  prevention: The audit produces a binary result: "keyspaces ARE populated" or "keyspaces are NOT populated". The bead closes with a `bd remember` note documenting the result.
  test_for_it: `test_audit_binary_result: the bd remember note is either "POPULATED" or "MISSING — fix applied"`.

### Data Integrity
- failure: The audit accidentally drops the existing keyspace data when reading it.
  prevention: The audit is READ-ONLY. It uses `journal.index_status.iter()` or `get()` to inspect, not `insert()` or `remove()`.
  test_for_it: `test_audit_is_read_only: the audit does not modify the FjallJournal; the keyspace contents are unchanged after the audit`.

### Integration Failure
- failure: The audit claims "keyspaces are populated" but the production code path is broken (e.g., the call is in a dead branch).
  prevention: The audit reads the FULL function body, not just the first 10 lines. It checks for the call in all branches (if/else, match, etc.).
  test_for_it: `test_audit_reads_full_function: the audit reads admission.rs:230-310 (80 lines) and recover.rs:251-296 (45 lines), not just the first few lines`.

## Section 4. ATDD Tests

### Happy
- name: `test_audit_finds_put_status_index_in_submit_artifact`
  given: The production code at `crates/vb_storage/src/admission.rs:230-310`.
  when: The audit searches for `put_status_index` in this range.
  then: If found, return "POPULATED". If not, return "MISSING".
  real_input: the full function body.
  expected_output: a binary result.
- name: `test_audit_finds_put_workflow_index_in_submit_artifact`
  given: The production code at `crates/vb_storage/src/admission.rs:230-310`.
  when: The audit searches for `put_workflow_index` in this range.
  then: Same as above.

### Error
- name: `test_audit_handles_missing_file_gracefully`
  given: The file `crates/vb_storage/src/admission.rs` does not exist (test isolation).
  when: The audit runs.
  then: Returns `Err(AuditError::FileNotFound)` (no panic).
  real_input: a non-existent file path.
  expected_error: `Err(AuditError)`.
- name: `test_audit_handles_unreadable_file_gracefully`
  given: The file exists but is not readable.
  when: The audit runs.
  then: Returns `Err(AuditError::PermissionDenied)`.
  real_input: a file with no read permission.
  expected_error: `Err(AuditError)`.

### Edge
- name: `test_audit_with_call_in_dead_branch_returns_missing`
  given: The function has `if false { put_status_index(...) }` (a dead branch).
  when: The audit searches for `put_status_index`.
  then: Returns "POPULATED" (the audit is grep-based, not control-flow-aware; the conservative answer is "POPULATED" if the call exists anywhere).
  real_input: a function with the call in a dead branch.
  expected: "POPULATED" (the audit errs on the side of "yes").
- name: `test_audit_with_no_call_returns_missing`
  given: The function has no `put_status_index` call.
  when: The audit searches.
  then: Returns "MISSING".
  real_input: a function without the call.
  expected: "MISSING".

### Contract
- name: `test_precondition_audit_is_read_only`
  verifies: Precondition "audit is read-only".
  test: assert the audit does not modify the FjallJournal.
- name: `test_postcondition_bd_remember_note_is_recorded`
  verifies: Postcondition "bd remember note is recorded".
  test: assert `bd remember` was called with the audit result.
- name: `test_invariant_no_pending_actions_method_exists`
  verifies: Invariant "no `pending_actions` method exists".
  test: `rg 'fn pending_actions' crates/vb_storage/` returns ZERO matches.

## Section 5. E2E Tests

```
pipeline_test:
  name: test_audit_e2e
  description: Real FjallJournal, real submit_artifact, real recovery; verify the indexes are populated.
  setup:
    - open a real FjallJournal
    - call submit_artifact with a valid digest
  execute:
    - call journal.index_status.get(key) and assert it returns Some(...)
    - call journal.index_workflow.get(key) and assert it returns Some(...)
  verify:
    - both index lookups return Some(...)
  cleanup:
    - close FjallJournal

e2e_scenarios:
  - name: e2e_submit_populates_indexes
    description: prove the indexes are populated after submit
    steps:
      - submit artifact
      - look up index_status
      - look up index_workflow
      - verify both have entries
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] `indexes.rs:15-35` read"
    - "[x] `chunk_032.rs:1-213` read (tests confirm the keyspaces are queried)"
    - "[x] `types.rs:300-310, 339-346` read (`pending_actions` is a bool field, not a method)"
    - "[x] `admission.rs:230-310` read (audit `put_status_index` / `put_workflow_index` calls)"
    - "[x] `recover.rs:251-296` read (audit same)"
    - "[x] Round-2 errors documented (§44.15 fabrication, `pending_actions` API fabrication)"
  evidence_required:
    - "Research notes file with line-numbered extracts"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All 7 acceptance tests written (2 happy, 2 error, 2 edge, 3 contract)"
    - "[ ] Tests pass (the audit can be a no-op if the keyspaces are already populated)"
  evidence_required:
    - "Test output"
    - "Audit result documented"

### Implementation
- name: "Implementation Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] All 7 tests pass"
    - "[ ] Audit is documented in a `bd remember` note"
    - "[ ] If a gap was found, the fix is applied"
    - "[ ] moon run :ci passes"
  evidence_required:
    - "Test output"
    - "bd remember note"

### Integration
- name: "Integration Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] E2E test passes (indexes are populated after submit)"
    - "[ ] No regressions in storage tests"
  evidence_required:
    - "E2E output"

## Section 6. Implementation Tasks

### Phase 0: Research
- [ ] Read `indexes.rs:15-35` (parallel: research)
- [ ] Read `chunk_032.rs:1-213` (parallel: research)
- [ ] Read `types.rs:300-310, 339-346` (parallel: research)
- [ ] Read `admission.rs:230-310` (parallel: research)
- [ ] Read `recover.rs:251-296` (parallel: research)
- [ ] Document the round-2 errors (parallel: research)
- [ ] Run the audit: `rg 'put_status_index|put_workflow_index' crates/vb_storage/src/admission.rs crates/vb_storage/src/recovery/recover.rs` (parallel: research)

### Phase 1: Tests
- [ ] Write `test_audit_finds_put_status_index_in_submit_artifact` (parallel: tests)
- [ ] Write `test_audit_finds_put_workflow_index_in_submit_artifact` (parallel: tests)
- [ ] Write `test_audit_handles_missing_file_gracefully` (parallel: tests)
- [ ] Write `test_audit_handles_unreadable_file_gracefully` (parallel: tests)
- [ ] Write `test_audit_with_call_in_dead_branch_returns_missing` (parallel: tests)
- [ ] Write `test_audit_with_no_call_returns_missing` (parallel: tests)
- [ ] Write 3 contract tests (parallel: tests)
- [ ] Confirm all 7 tests pass (gate)

### Phase 2: Implementation
- [ ] If audit finds a gap: add `journal.put_status_index(...)?; journal.put_workflow_index(...)?;` in `admission.rs:304` (depends: tests; sequential)
- [ ] If audit finds a gap: add a corresponding call in `recover.rs:289` per-run (depends: fix; sequential)
- [ ] Record the audit result in `bd remember` (depends: fix; sequential)
- [ ] If audit finds no gap: skip the fix; just record the result (depends: audit; sequential)

### Phase 3: Integration
- [ ] Run the E2E test (depends: impl; sequential)
- [ ] Confirm the indexes are populated (sequential)
- [ ] Run `cargo test -p vb_storage` to confirm no regressions (sequential)

### Phase 4: Documentation
- [ ] Run `moon run :ci` (depends: all of the above; parallel)
- [ ] Close the bead (sequential)

## Section 7. Failure Modes

- symptom: "Audit returns 'MISSING' but the keyspaces are actually populated"
  likely_cause: The audit is grep-based and misses calls in macros, dead branches, or commented-out code.
  where_to_look:
    - file: `crates/vb_storage/src/admission.rs:230-310`
    - what_to_check: "Are there any `put_status_index` calls in macros or commented code?"
  fix_pattern: Use a smarter search (e.g., ripgrep with `-(A|B)` for context).
- symptom: "Audit returns 'POPULATED' but the keyspaces are actually missing"
  likely_cause: The audit is reading a stale version of the file.
  where_to_look:
    - file: `crates/vb_storage/src/admission.rs`
    - what_to_check: "Is the file up to date with the latest commit?"
  fix_pattern: Re-read the file after a fresh `git pull` or `jj log`.
- symptom: "Test fails: `bd remember` note is not recorded"
  likely_cause: The `bd remember` command failed (e.g., network issue, auth issue).
  where_to_look:
    - file: `crates/vb_storage/src/recovery/types.rs`
    - what_to_check: "Is the audit result recorded in a `bd remember` note?"
  fix_pattern: Manually run `bd remember` to record the note.

debugging_commands:
- scenario: "When the audit is wrong"
  run: "rg 'put_status_index|put_workflow_index' crates/vb_storage/src/"
  look_for: "All call sites; cross-check with the audit conclusion"
- scenario: "When the bd remember note is missing"
  run: "bd remember --list"
  look_for: "The audit note should be in the list"

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT cite "master §44.15" or "operational affordances" — these do NOT exist in the master doc.
- DO NOT add a `pending_actions` gate or method — only the `bool` flag exists.
- DO NOT remove the dead code without audit — the round-2 "wire vs remove" decision is binary, but the actual scope is "audit the indexes".
- DO NOT add a P0 dependency (round-2 had vb-v1jiq; that was a P0→P2 inversion).

VERIFY that:
- Master §44.15 does NOT exist: `rg '44\.15' master doc` (must return ZERO matches).
- No `pending_actions` method exists: `rg 'fn pending_actions' crates/vb_storage/` (must return ZERO matches).
- The keyspaces are queried in tests: `rg 'index_status\.get|index_workflow\.get' crates/vb_storage/src/tests/` (must return at least 1 match).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    rg 'put_status_index|put_workflow_index' crates/vb_storage/src/  # confirm the audit is complete

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-wyosk/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-wyosk/progress.txt` and continue from "Current Task". The audit is read-only; the fix (if needed) is additive.
Key invariants:
- This is an AUDIT bead, not a "wire vs remove" binary decision.
- `pending_actions` is a `bool` flag on `UnsupportedRecoveryState`, NOT a method.
- Master §44.15 does NOT exist.
- This bead has NO dependencies.

## Section 8. Completion Checklist

- [ ] All 7 acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing with real FjallJournal
- [ ] No mocks or fake data in any test
- [ ] moon run :ci passes
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] No production code touched outside `crates/vb_storage/src/admission.rs` and `recover.rs` (if fix is needed)
- [ ] bd remember note recorded with audit result
- [ ] bd close with reason: "P2-15r complete: audit found [POPULATED | MISSING — fix applied]"

## Section 9. Context

Related files:
- `crates/vb_storage/src/indexes.rs:15-24` — `put_status_index` method
- `crates/vb_storage/src/indexes.rs:27-35` — `put_workflow_index` method
- `crates/vb_storage/src/tests/chunk_032.rs:1-213` — tests verifying the keyspaces
- `crates/vb_storage/src/recovery/types.rs:300-310` — `UnsupportedRecoveryState`
- `crates/vb_storage/src/recovery/types.rs:339-346` — `pending_actions_unsupported` factory
- `crates/vb_storage/src/admission.rs:230-310` — `submit_artifact` (audit target)
- `crates/vb_storage/src/recovery/recover.rs:251-296` — `recover_runtime_frame_seed` (audit target)

Similar implementations:
- The existing `chunk_032.rs` tests show the pattern of asserting `journal.index_status.get(key.as_slice())`. Apply the same pattern to the audit.

Codebase patterns:
- pattern: "Grep-based audit"
  example_location: (none in current codebase; this is a NEW pattern)
  how_to_apply: Use ripgrep to search for the index method calls in the production code paths.

## Section 10. AI Hints

### DO
- Read `crates/vb_storage/src/indexes.rs:15-35` BEFORE writing any code.
- Use `rg 'put_status_index|put_workflow_index'` to audit the production code.
- Record the audit result in `bd remember`.
- If a gap is found, add the fix in `admission.rs:304` and `recover.rs:289` (per-run).
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT use `unwrap()` or `expect()`.
- Do NOT cite "master §44.15" or "operational affordances".
- Do NOT add a `pending_actions` method.
- Do NOT add a P0 dependency.
- Do NOT use `unsafe`.

### Code patterns
- name: "Grep-based audit"
  use_when: "Determining whether a function calls a specific method"
  example: |
    let status_calls = rg "put_status_index" admission_rs_path | count;
    let workflow_calls = rg "put_workflow_index" admission_rs_path | count;
    if status_calls > 0 && workflow_calls > 0 {
        "POPULATED"
    } else {
        "MISSING"
    }

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect().
- Test first: Tests MUST exist and FAIL before implementation.
- Read before write: ALWAYS read the source file before modifying it.
- Real data only: Use real `put_status_index` / `put_workflow_index`; no fabricated placeholders.
- Minimal change: AUDIT (read-only) + optional 1-line fix; do NOT refactor the storage crate.
