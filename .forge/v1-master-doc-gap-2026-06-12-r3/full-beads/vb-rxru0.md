# P0-4r2 runtime-action-mock-arms

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> `crates/vb_runtime/src/action.rs` (212 lines), `crates/vb_cli/src/commands_journal.rs`,
> master doc §19 (line 876-1005), master doc §75 (line 4317-4324).

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications (decided by black-hat round 3):
- The action is `dispatch_action(action: ActionId, input: ActionInput) -> ActionResult<ActionOutcome>` from master §19; there is no `ActionExecutor` trait. The fix is direct match arms inside the existing `dispatch_generic` body.
- The three action names are real master §75 names: `github.issue.create` (ActionId=7), `ai.classify_ticket` (ActionId=12), `http.request`. http.request is in the user spec; not in master §75 enumerations.
- The new marker must ride on the existing `ActionTicket` payload (the round-2 attempt to add a new file `crates/vb_runtime/src/action/mocks.rs` was black-hat-rejected as over-engineering).

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL map the action names `github.issue.create`, `ai.classify_ticket`, `http.request` to typed `MockMarker` payload variants when `ActionRegistry::dispatch_generic` is invoked for them.
- THE SYSTEM SHALL fall through to the existing table-driven `dispatch_generic` path for all other action names without behavior change.

### Event-Driven
- WHEN `ActionRegistry::dispatch(&self, input, contract)` is called with a registered contract for `github.issue.create` (ActionId=7), THE SYSTEM SHALL return `ActionOutcome::Suspended(ticket)` where `ticket.payload` carries `MockMarker::GitHubIssueCreate`.
- WHEN `ActionRegistry::dispatch` is called with a registered contract for `ai.classify_ticket` (ActionId=12), THE SYSTEM SHALL return `ActionOutcome::Suspended(ticket)` where `ticket.payload` carries `MockMarker::AiClassifyTicket`.
- WHEN `ActionRegistry::dispatch` is called with a registered contract for `http.request`, THE SYSTEM SHALL return `ActionOutcome::Suspended(ticket)` where `ticket.payload` carries `MockMarker::HttpRequest`.
- WHEN `ActionRegistry::dispatch` is called with an unregistered action, THE SYSTEM SHALL return the existing typed `Err(UnknownAction)` (no panic).

### Unwanted
- THE SYSTEM SHALL NOT introduce a new `ActionExecutor` trait (master §19 static-dispatch contract is the only contract).
- THE SYSTEM SHALL NOT add 3 mock actions named `Echo`, `ComputeHash`, `Delay` — these are fabricated and unrelated to the master §75 action names.
- THE SYSTEM SHALL NOT create a new file `crates/vb_runtime/src/action/mocks.rs`; the match arms live inside `dispatch_generic` at `crates/vb_runtime/src/action.rs:182-194`.
- THE SYSTEM SHALL NOT panic on any action id in the range 0..=u16::MAX (Kani harness `dispatch_never_panics` proves this).

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs:
  - field: `input`
    type: `&ActionInput` (vb_core::action::ActionInput)
    constraints: `run, step, action, ticket` must be well-formed; `action` must be a valid `ActionId`.
    example_valid: `ActionInput { run: RunId::new(1), step: StepIdx::new(0), action: ActionId::new(7), ticket: ActionTicket {...}, input: SlotIdx::new(0) }`
    example_invalid: `ActionInput { action: ActionId::new(u16::MAX + 1), ... }` (cannot construct — `ActionId::new` is bounded)
  - field: `contract`
    type: `&ActionContract`
    constraints: must be registered in the `ActionRegistry` instance for the dispatch path to succeed.
    example_valid: `ActionContract { id: ActionId::new(7), name: "github.issue.create", ... }`
    example_invalid: unregistered ActionId (returns `Err(UnknownAction)`)
- system_state:
  - `ActionRegistry::dispatch` is implemented at `crates/vb_runtime/src/action.rs:122-136` and delegates to `dispatch_generic` at line 182-194.
  - Master §19 specifies a static dispatch by `ActionId`; the `ActionOutcome` enum has exactly 3 variants: `Ready`, `Suspended`, `Failed`.

### Postconditions
- state_changes:
  - For an action in the mock-marker set, the returned `ActionOutcome::Suspended(ticket)` carries a `MockMarker` enum field on the ticket (a new field) with one of the three variants.
  - For all other actions, the dispatch path is unchanged (table-driven, returns `ActionOutcome::Suspended(ticket)` without a MockMarker).
  - The `ActionRegistry` does not mutate across dispatch calls.
- return_guarantees:
  - field: `ActionOutcome`
    guarantee: One of `Ready(())`, `Suspended(ActionTicket)`, or `Failed { ... }`. The mock-marker set returns `Suspended(ticket)`; the marker distinguishes which mock would handle it.
  - field: `ActionResult<ActionOutcome>::Err`
    guarantee: `UnknownAction` for unregistered action ids, never panics.
- side_effects: None. `dispatch` is a pure function over the registry state.

### Invariants
- The set of action names that produce a `MockMarker` is EXACTLY `{github.issue.create, ai.classify_ticket, http.request}`. No other name triggers a marker.
- The marker enum has EXACTLY 3 variants, one per mock action. Adding a 4th mock action is a separate bead.
- `dispatch` is total: for every `(ActionId, ActionInput)` pair, the function returns `Ok(_)` or `Err(UnknownAction)`, never `unreachable!()` or `panic!()`.
- The Kani harness `dispatch_never_panics` proves invariant 3 for `ActionId < 100` (bounded exploration).

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: `crates/vb_runtime/src/action.rs:122-194`
  what_to_extract: The full body of `ActionRegistry::dispatch` and the `dispatch_generic` table-driven path. Confirm the function signature, the match on ActionId, and the table look-up.
  document_in: research_notes.md
- path: `crates/vb_core/src/action.rs` (look up `ActionInput`, `ActionOutcome`, `ActionTicket` field list)
  what_to_extract: The `ActionTicket` struct fields so the new `MockMarker` field can be added without breaking the public surface.
  document_in: research_notes.md
- path: `master doc §19` (line 876-1005)
  what_to_extract: Confirm static dispatch is by `ActionId` and `ActionOutcome = {Ready, Suspended, Failed}`. There is NO `ActionExecutor` trait.
  document_in: research_notes.md
- path: `master doc §75` (line 4317-4324)
  what_to_extract: The 12 enumerated action names and their ActionId assignments. `github.issue.create` = ActionId 7, `ai.classify_ticket` = ActionId 12.
  document_in: research_notes.md

Patterns to find:
- pattern: `MockMarker` (look in `crates/vb_runtime/src/action.rs` and `crates/vb_core/src/action.rs`)
  purpose: Verify the marker does NOT exist; this bead adds it.
  expected_locations: NONE — this is a new enum.
- pattern: `dispatch_generic` (search the runtime crate)
  purpose: Locate the function to modify.
  expected_locations: `crates/vb_runtime/src/action.rs:182-194`.

Prior art:
- feature: `ActionRegistry::dispatch` static-dispatch path
  location: `crates/vb_runtime/src/action.rs:122-136`
  what_to_learn: The pattern of `match self.contracts.get(&input.action) { ... }` with fallthrough. Apply same shape for the 3-action mock arm.

External docs:
- url: master doc §19 (verified line range)
  section: dispatch action
  extract: confirm `dispatch_action` signature and `ActionOutcome` enum.

Research questions (all answered):
- Q: Should the MockMarker be a new field on ActionTicket or a new struct? A: New field on `ActionTicket` (round-3 decision: minimal API change, follows existing pattern of typed payload markers).
- Q: Should the match happen before or after the contract lookup? A: After (consistent with the existing `dispatch_generic` flow; the contract must be present for the marker to be meaningful).

Research complete when:
- [x] All files_to_read opened and key info extracted.
- [x] All patterns_to_find searched.
- [x] All prior_art examined.
- [x] All research_questions have answers documented.

## Section 3. Inversions

### Security
- failure: An attacker registers a contract for an action named `system.shutdown` (a fabricated name) and forces the dispatch path to return a `MockMarker::GitHubIssueCreate` marker, tricking downstream consumers into executing a GitHub API call.
  prevention: The mock-marker set is HARD-CODED to 3 names from master §75. Any other name falls through to the table-driven path (no marker). The marker is a HINT, not a permission; downstream still uses the contract's name.
  test_for_it: `test_no_marker_for_fabricated_name: register "system.shutdown" with ActionId=99; call dispatch; assert result is `ActionOutcome::Suspended(ticket)` with NO MockMarker field set, OR `Err(UnknownAction)` if 99 is unregistered.

### Usability
- failure: A developer writing a mock executor cannot tell which of the 3 mock actions a suspended ticket is for, because the marker is missing.
  prevention: The `MockMarker` enum is added to `ActionTicket` with 3 variants. `Debug` and `PartialEq` are derived so a single match arm identifies the action.
  test_for_it: `test_marker_distinguishes_three_actions: dispatch 3 actions; assert the 3 returned tickets carry distinct MockMarker variants.`

### Data Integrity
- failure: The marker accidentally collides with a real action id (e.g., a v2 action with id 7 is added and the marker fires for it, hiding the collision).
  prevention: The mock arm is gated on the action NAME (`github.issue.create`), not on the ActionId. If a v2 action is added with the same id but a different name, the mock arm does NOT fire.
  test_for_it: `test_marker_uses_name_not_id: register a contract with ActionId=7 but a different name "future.action"; assert no MockMarker fires.`

### Integration Failure
- failure: Downstream code (e.g., `vb_cli`) parses `ActionOutcome::Suspended` and assumes `ticket.payload` is always a `Value` blob, but the new `MockMarker` field changes the layout.
  prevention: The `MockMarker` is an enum with `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`; it does NOT change the `payload` field type. New `match` arms on the marker are added in a backward-compatible way (`#[non_exhaustive]` on the marker if needed).
  test_for_it: `test_payload_field_unchanged: dispatch a non-mock action; assert ticket.payload is still `Value` (or whatever the existing type is) — no layout change.`

## Section 4. ATDD Tests

### Happy
- name: `test_dispatch_github_issue_create_returns_marker`
  given: A fresh `ActionRegistry`; a contract registered for `github.issue.create` (ActionId=7) with a no-op closure.
  when: `ActionRegistry::dispatch` is called with an `ActionInput` whose `action == ActionId::new(7)`.
  then:
    - Returns `Ok(ActionOutcome::Suspended(ticket))`.
    - `ticket.payload.MockMarker == Some(MockMarker::GitHubIssueCreate)`.
  real_input: `ActionInput { run: RunId::new(1), step: StepIdx::new(0), action: ActionId::new(7), input: SlotIdx::new(0), ticket: ActionTicket { run: RunId::new(1), step: StepIdx::new(0), seq: 0, action: ActionId::new(7), attempt: 1, idempotency_key: 0, capacity: 1 } }`
  expected_output: `Ok(Suspended(ticket_with_marker))`
- name: `test_dispatch_ai_classify_ticket_returns_marker`
  given: A contract for `ai.classify_ticket` (ActionId=12).
  when: `ActionRegistry::dispatch` is called with `action == ActionId::new(12)`.
  then: Returns `Suspended(ticket)` with `MockMarker::AiClassifyTicket`.
  real_input: same shape as above, action=12.
  expected_output: `Ok(Suspended(ticket_with_marker))`
- name: `test_dispatch_http_request_returns_marker`
  given: A contract for `http.request`.
  when: `ActionRegistry::dispatch` is called with the corresponding ActionId.
  then: Returns `Suspended(ticket)` with `MockMarker::HttpRequest`.
  real_input: same shape, action=http.request's assigned id.
  expected_output: `Ok(Suspended(ticket_with_marker))`

### Error
- name: `test_dispatch_unknown_action_returns_typed_error`
  given: A fresh `ActionRegistry`; no contracts registered.
  when: `ActionRegistry::dispatch` is called with `action == ActionId::new(999)`.
  then: Returns `Err(UnknownAction)`. No panic.
  real_input: `ActionInput { action: ActionId::new(999), ... }`
  expected_error: `Err(ActionError::UnknownAction { action: 999 })`
- name: `test_dispatch_unknown_action_returns_typed_error_compile_path`
  given: A workflow YAML referencing action_id=99 (unregistered).
  when: The compiled workflow is run; the action is invoked.
  then: Returns `Err(UnknownAction)`. No panic.
  real_input: YAML inline test fixture.
  expected_error: typed CLI error exit code 3.

### Edge
- name: `test_dispatch_marker_arm_skipped_for_unrelated_action`
  given: A contract for `github.issue.create` (id=7) and a separate contract for `noop` (id=0).
  when: `dispatch` is called with `action == ActionId::new(0)`.
  then: Returns `Suspended(ticket)` WITHOUT a MockMarker (the mock arm only fires for the 3 mock names).
  real_input: `ActionInput { action: ActionId::new(0), ... }`
  expected: `MockMarker` is `None` on the ticket; the regular `dispatch_generic` path is taken.
- name: `test_dispatch_marker_arm_with_empty_registry`
  given: A fresh `ActionRegistry` (no contracts).
  when: `dispatch` is called with `action == ActionId::new(7)`.
  then: Returns `Err(UnknownAction)` — the marker arm is unreachable because the contract is missing.
  real_input: `ActionInput { action: ActionId::new(7), ... }`
  expected_error: `Err(UnknownAction)` (the marker arm is gated behind contract presence).

### Contract
- name: `test_precondition_registry_has_contract_for_action_id`
  verifies: Precondition "contract must be registered".
  test: With an empty registry, `dispatch` returns `Err(UnknownAction)` for any action id.
- name: `test_postcondition_suspended_ticket_carries_marker`
  verifies: Postcondition "the returned Suspended ticket carries a MockMarker".
  test: For each of the 3 mock actions, the ticket's `MockMarker` field is set to the corresponding variant.
- name: `test_invariant_dispatch_is_total`
  verifies: Invariant "dispatch returns Ok(_) or Err(UnknownAction), never panics".
  test: proptest with 1000 random `ActionId` values in `0..=u16::MAX`; all return either `Ok(_)` or `Err(UnknownAction)`. No panics. (Kani harness covers a smaller bounded subset; proptest covers the rest.)

## Section 5. E2E Tests

```
pipeline_test:
  name: test_runtime_dispatch_with_all_three_mock_markers_e2e
  description: Real FjallJournal, real Runtime, real ActionRegistry; submit a workflow
               with all 3 mock actions; assert the suspended tickets carry the right markers.
  setup:
    - open a real FjallJournal in a tempdir
    - create a Runtime with a real ShardConfig
    - build a CompiledWorkflow with 3 Do steps: action_id=7, 12, http.request
  execute:
    command: "moon run -- vb run /tmp/test-workflow.yaml --poll-deadline-ms 1000"
    timeout_ms: 5000
  verify:
    - exit_code: 0
    - stdout_contains: "3 actions suspended"
    - stdout_contains: "github.issue.create" "ai.classify_ticket" "http.request"
    - each suspended ticket has a distinct MockMarker
  cleanup:
    - delete /tmp/test-workflow.yaml
    - close FjallJournal

e2e_scenarios:
  - name: e2e_real_runtime_dispatch_all_three_mocks
    description: prove all 3 MockMarker variants fire in a real run
    steps:
      - submit workflow with 3 mock actions
      - poll until 3 tickets are suspended
      - verify each ticket's MockMarker is one of the 3 variants
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] All research_requirements files have been read (action.rs:122-194, vb_core::action.rs, master §19, master §75)"
    - "[x] All research_questions have documented answers"
    - "[x] All assumptions validated (no ActionExecutor trait; ActionOutcome = {Ready, Suspended, Failed})"
  evidence_required:
    - "Research notes file with key extracts from each source"
    - "Confirmed: NO `ActionExecutor` trait exists; NO `crates/vb_runtime/src/action/mocks.rs` exists"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All acceptance tests written (3 happy, 2 error, 2 edge, 3 contract = 10 tests)"
    - "[ ] All tests fail with the expected compile error (MockMarker does not exist yet)"
  evidence_required:
    - "Test files in `crates/vb_runtime/src/action/tests.rs`"
    - "Compile error output referencing the missing `MockMarker` enum"

### Implementation
- name: "Implementation Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] All 10 tests pass"
    - "[ ] No unwrap() or expect() in new code"
    - "[ ] moon run :ci passes"
  evidence_required:
    - "Test output showing all pass"
    - "CI output showing green"

### Integration
- name: "Integration Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] E2E test passes with real FjallJournal and real Runtime"
    - "[ ] No regressions in existing runtime tests"
  evidence_required:
    - "E2E test output"
    - "Manual verification notes (3 markers observed in 3 distinct tickets)"

## Section 6. Implementation Tasks

### Phase 0: Research
- [ ] Read `crates/vb_runtime/src/action.rs:122-194` (parallel: research group)
- [ ] Read `crates/vb_core/src/action.rs` (parallel: research group)
- [ ] Read master doc §19 (line 876-1005) (parallel: research group)
- [ ] Read master doc §75 (line 4317-4324) (parallel: research group)
- [ ] Confirm no `ActionExecutor` trait exists (parallel: research group)
- [ ] Document the 3 mock action names with their ActionId assignments (parallel: research group)

### Phase 1: Tests
- [ ] Add `MockMarker` enum + field declaration to `crates/vb_core/src/action.rs` (test scaffold) (parallel: tests group)
- [ ] Write `test_dispatch_github_issue_create_returns_marker` (parallel: tests group)
- [ ] Write `test_dispatch_ai_classify_ticket_returns_marker` (parallel: tests group)
- [ ] Write `test_dispatch_http_request_returns_marker` (parallel: tests group)
- [ ] Write `test_dispatch_unknown_action_returns_typed_error` (parallel: tests group)
- [ ] Write `test_dispatch_marker_arm_skipped_for_unrelated_action` (parallel: tests group)
- [ ] Write `test_dispatch_marker_arm_with_empty_registry` (parallel: tests group)
- [ ] Write 3 contract verification tests (parallel: tests group)
- [ ] Confirm all 10 tests fail with "MockMarker not found" compile error (gate: tests exist + fail)

### Phase 2: Implementation
- [ ] Add `MockMarker` enum (3 variants) to `crates/vb_core/src/action.rs` (depends: tests; sequential)
- [ ] Add `MockMarker` field to `ActionTicket` struct (depends: enum defined; sequential)
- [ ] Modify `dispatch_generic` at `crates/vb_runtime/src/action.rs:182-194` to add a match arm for the 3 mock action names (depends: enum + field; sequential)
- [ ] Resolve the action name to ActionId for the 3 mocks using `ActionRegistry::resolve_by_name` (depends: match arm; sequential)
- [ ] Confirm all 10 tests pass (gate: tests pass)

### Phase 3: Integration
- [ ] Write the E2E test in `crates/workspace_tests/tests/` (depends: tests + impl; sequential)
- [ ] Run the E2E test (sequential)
- [ ] Confirm no regressions in `cargo test -p vb_runtime` (sequential)

### Phase 4: Documentation
- [ ] Update the Kani harness at `crates/vb_runtime/src/kani/action_dispatch.rs` to include the new `MockMarker` field in the ticket (depends: impl; parallel)
- [ ] Run `moon run :ci` and confirm green (depends: all of the above; parallel)
- [ ] Close the bead with `bd close vb-rxru0` (depends: CI green; sequential)

## Section 7. Failure Modes

- symptom: "Compile error: cannot find type `MockMarker`"
  likely_cause: Test was written before the enum was defined in `vb_core::action`.
  where_to_look:
    - file: `crates/vb_runtime/src/action/tests.rs`
    - function: any test referencing `MockMarker`
    - what_to_check: "Is the use statement `use vb_core::action::MockMarker;` present?"
  fix_pattern: Add the import. The enum is in `vb_core::action` once Phase 2 step 1 completes.
- symptom: "Test fails: ticket.MockMarker is None even though action is github.issue.create"
  likely_cause: The match arm in `dispatch_generic` is not firing. Either the action name resolution is wrong, or the arm is after the fallthrough.
  where_to_look:
    - file: `crates/vb_runtime/src/action.rs`
    - function: `dispatch_generic` (line 182-194)
    - what_to_check: "Is the new match arm BEFORE the existing table-driven path?"
  fix_pattern: Move the new match arm to the top of `dispatch_generic`. It must run BEFORE the existing `self.contracts.get(&input.action)` lookup.
- symptom: "Test fails: UnknownAction error for an action that has a contract"
  likely_cause: The `MockMarker` arm short-circuits the contract lookup. The mock arm should NOT skip the contract check; it should run AFTER the contract is verified.
  where_to_look:
    - file: `crates/vb_runtime/src/action.rs:122-136`
    - function: `ActionRegistry::dispatch`
    - what_to_check: "Does the contract check happen before the mock arm?"
  fix_pattern: Reorder: contract check first; if contract is present and the name is one of the 3 mocks, set the marker; otherwise fall through.

debugging_commands:
- scenario: "When the marker doesn't appear in the suspended ticket"
  run: "RUST_LOG=vb_runtime=trace moon run -- vb run /tmp/test-workflow.yaml"
  look_for: "Trace log line indicating which match arm fired in dispatch_generic"
- scenario: "When all actions return UnknownAction"
  run: "rg 'pub fn resolve_by_name' crates/vb_runtime/src/action.rs"
  look_for: "Confirm the function exists and returns the expected ActionId for the 3 mock names"

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT add a new `ActionExecutor` trait (verified: master §19 uses static dispatch).
- DO NOT add 3 mock actions named `Echo`, `ComputeHash`, `Delay` (fabricated; the real names are `github.issue.create`, `ai.classify_ticket`, `http.request`).
- DO NOT create `crates/vb_runtime/src/action/mocks.rs` (round-2 over-engineering; the match arms go INSIDE `dispatch_generic`).
- DO NOT change the `ActionOutcome` enum (it is `enum { Ready, Suspended, Failed }` per master §19; adding a 4th variant is a separate bead).
- DO NOT use the action ID (e.g., `7`) to gate the marker; use the action NAME. Round-2 used the id; the real spec uses the name.

VERIFY that:
- `ActionOutcome::Suspended` exists: `rg "Suspended" crates/vb_core/src/action.rs` (must find at least one match).
- `ActionTicket` struct has the documented fields: `rg "pub struct ActionTicket" crates/vb_core/src/action.rs` (must find the struct).
- `dispatch_generic` is at lines 182-194: `rg "fn dispatch_generic" crates/vb_runtime/src/action.rs` (must return the line number).
- No `ActionExecutor` trait exists: `rg "trait ActionExecutor" crates/` (must return ZERO matches).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    rg "MockMarker" crates/  # confirm marker is wired

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-rxru0/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-rxru0/progress.txt` and continue from "Current Task". The source-code read references in Section 2.5 are stable; do not skip them on resume.
Key invariants:
- The 3 mock action names are HARDCODED from master §75; do not change them.
- The marker is a HINT, not a permission; downstream still uses the contract's name.
- The match arm lives INSIDE `dispatch_generic`, not in a new trait or file.

## Section 8. Completion Checklist

- [ ] All 10 acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing with real FjallJournal + real Runtime
- [ ] No mocks or fake data in any test (mock action names are real master §75 names)
- [ ] Kani harness `dispatch_never_panics` updated and passing
- [ ] moon run :ci passes
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] No production code touched outside `crates/vb_runtime/src/action.rs` and `crates/vb_core/src/action.rs`
- [ ] bd close with reason: "P0-4r2 complete: MockMarker enum + dispatch_generic arms"

## Section 9. Context

Related files:
- `crates/vb_runtime/src/action.rs:122-136` — `ActionRegistry::dispatch` (the dispatch entry point)
- `crates/vb_runtime/src/action.rs:182-194` — `dispatch_generic` (the function to modify)
- `crates/vb_core/src/action.rs` — `ActionInput`, `ActionOutcome`, `ActionTicket` (struct definitions)
- `master doc §19` (line 876-1005) — static-dispatch contract
- `master doc §75` (line 4317-4324) — action name to ActionId mapping

Similar implementations:
- `crates/vb_runtime/src/action.rs` line 122-136 shows the existing dispatch pattern with `match self.contracts.get(...) { ... }` fallthrough. Apply the same shape to the new mock arm.
- `crates/vb_core/src/action.rs` has the existing `ActionTicket` struct; add the `MockMarker` field there.

Codebase patterns:
- pattern: "Action contract registry"
  example_location: `crates/vb_runtime/src/action.rs:122-136`
  how_to_apply: Use the same `match` arm shape inside `dispatch_generic`; do not introduce a new dispatch mechanism.

## Section 10. AI Hints

### DO
- Read `crates/vb_runtime/src/action.rs:182-194` BEFORE writing any code. The function is short; the read is fast.
- Use the `ActionRegistry::resolve_by_name` helper to convert action names to ActionId (avoids hardcoding id numbers).
- Add the `MockMarker` enum in `vb_core::action` so it can be referenced from both `vb_core` and `vb_runtime`.
- Derive `Debug, Clone, Copy, PartialEq, Eq` on `MockMarker` so it can be matched without allocation.
- Keep the match arm INSIDE `dispatch_generic`; do NOT add a new method or trait.
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT use `unwrap()` or `expect()` in tests or production code.
- Do NOT use `panic!`, `todo!`, or `unimplemented!`.
- Do NOT introduce a new `ActionExecutor` trait.
- Do NOT add 3 mock actions named `Echo`, `ComputeHash`, `Delay`.
- Do NOT create `crates/vb_runtime/src/action/mocks.rs`.
- Do NOT use `unsafe`.
- Do NOT modify clippy configuration.

### Code patterns
- name: "Match arm with name-based resolution"
  use_when: "Adding a new dispatch path for a subset of action names"
  example: |
    // In dispatch_generic, after the contract check:
    if let Some(name) = self.contracts.name_for(input.action) {
        match name.as_str() {
            "github.issue.create" => { /* set MockMarker::GitHubIssueCreate */ }
            "ai.classify_ticket" => { /* set MockMarker::AiClassifyTicket */ }
            "http.request" => { /* set MockMarker::HttpRequest */ }
            _ => { /* fallthrough */ }
        }
    }

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect() in production code.
- Test first: Tests MUST exist and FAIL before implementation.
- No new traits: This is a v1 static-dispatch language; traits come in v2.
- Read before write: ALWAYS read the source file before modifying it.
- Real data only: Use real action names from master §75; no fabricated placeholders.
