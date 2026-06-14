# P1-12r simulate-structured

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> `crates/vb_cli/src/commands_workflow/mod.rs` (lines 17-60), `dot.rs`,
> master doc §75 (lines 4133-4170).

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications (round-2 corrections applied):
- The baseline `SimulationStep` has EXACTLY 3 fields (`index, kind_label, description`), NOT 4.
- The new spec is to ADD 1 field (`kind: StepKind` enum) and RENAME `kind_label` to `kind_label_text` (freeing the `kind` name).
- Total after the change: 4 fields (`index, kind_label_text, kind, description`).
- Master doc §75 specifies the wire-format `events` output, NOT the `SimulationStep` struct. The struct is a local Rust concern.

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL add a new field `kind: StepKind` (enum) to `SimulationStep`.
- THE SYSTEM SHALL rename the existing `kind_label: String` field to `kind_label_text: String`.
- THE SYSTEM SHALL preserve the existing 2 fields (`index: usize`, `description: String`).
- THE SYSTEM SHALL NOT add `action_id`, `mock_output`, or `suspension_reason` to `SimulationStep` (round-2 over-spec).

### Event-Driven
- WHEN `simulate_workflow` is called with a workflow containing a `Do` step, THE SYSTEM SHALL set `steps[0].kind = StepKind::Do` and `steps[0].kind_label_text = "Do"`.
- WHEN `simulate_workflow` is called with a workflow containing a `SetConst` step, THE SYSTEM SHALL set `steps[0].kind = StepKind::SetConst` and `steps[0].kind_label_text = "Set"`.
- WHEN `simulate_workflow` is called with a workflow containing a `Finish` step, THE SYSTEM SHALL set `steps[0].kind = StepKind::Finish` and `steps[0].kind_label_text = "Finish"`.

### Unwanted
- THE SYSTEM SHALL NOT claim 4 baseline fields — there are 3.
- THE SYSTEM SHALL NOT add `action_id`, `mock_output`, or `suspension_reason` to `SimulationStep`.
- THE SYSTEM SHALL NOT claim master §75 specifies `SimulationStep` fields — master §75 specifies the wire-format `events` output.

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs:
  - field: `workflow`
    type: `&CompiledWorkflow`
    constraints: must be a valid compiled workflow.
    example_valid: `CompiledWorkflow { node_count: 1, ... }`
    example_invalid: N/A (the type enforces validity)
- system_state:
  - `SimulationStep` has 3 fields at `commands_workflow/mod.rs:17-21`.
  - `SimulationResult` has 4 fields at `commands_workflow/mod.rs:23-28`.
  - `simulate_workflow` is at `commands_workflow/mod.rs:30-60`.
  - `node_kind_label` is in `dot.rs` (re-exported at line 11).
  - `describe_node_for_simulate` is at `commands_workflow/mod.rs:44-45`.

### Postconditions
- state_changes:
  - `SimulationStep` has 4 fields after the change: `index, kind_label_text, kind, description`.
  - `StepKind` enum is defined in `commands_workflow/mod.rs` (or a sibling module).
- return_guarantees:
  - field: `SimulationResult.steps[i].kind`
    guarantee: One of the `StepKind` variants.
  - field: `SimulationResult.steps[i].kind_label_text`
    guarantee: The string label for the kind (e.g., "Do", "Set", "Finish").
- side_effects: None. `simulate_workflow` is a pure function.

### Invariants
- The 4 fields of `SimulationStep` are EXACTLY: `index: usize, kind_label_text: String, kind: StepKind, description: String`.
- The `StepKind` enum has at least the variants: `Nop, SetConst, Copy, EvalExpr, BuildObject, BuildList, Do, Choose, ForEach, Together, Collect, Reduce, Repeat, WaitUntil, WaitEvent, Ask, RetryCheck, ErrorHandler, Finish` (and more as needed).
- The `kind_label_text` is the same string as `node_kind_label(node.kind).to_string()`.

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: `crates/vb_cli/src/commands_workflow/mod.rs:17-21`
  what_to_extract: The `SimulationStep` struct definition. Confirm EXACTLY 3 fields: `index, kind_label, description`.
  document_in: research_notes.md
- path: `crates/vb_cli/src/commands_workflow/mod.rs:23-28`
  what_to_extract: The `SimulationResult` struct. Confirm 4 fields: `steps, total_steps, action_count, branch_count`.
  document_in: research_notes.md
- path: `crates/vb_cli/src/commands_workflow/mod.rs:30-60`
  what_to_extract: The `simulate_workflow` function. Confirm it iterates `workflow.node_count()`, calls `node_kind_label` and `describe_node_for_simulate`, pushes a `SimulationStep`.
  document_in: research_notes.md
- path: `crates/vb_cli/src/commands_workflow/dot.rs` (or wherever `node_kind_label` is defined)
  what_to_extract: The `node_kind_label` function. Confirm it returns a `&'static str` for each `CompiledNodeKind` variant.
  document_in: research_notes.md
- path: master doc §75 (lines 4133-4170)
  what_to_extract: Confirm master §75 specifies the wire-format `events` output, NOT the `SimulationStep` struct.
  document_in: research_notes.md

Patterns to find:
- pattern: `SimulationStep`
  purpose: Locate all uses of the struct to ensure the rename and new field are propagated.
  expected_locations: `crates/vb_cli/src/commands_workflow/mod.rs:17-60` and any callers.
- pattern: `node_kind_label`
  purpose: Locate the function that produces the string label.
  expected_locations: `crates/vb_cli/src/commands_workflow/dot.rs:11` (re-export) or the original definition.

Prior art:
- feature: existing 3-field `SimulationStep`
  location: `crates/vb_cli/src/commands_workflow/mod.rs:17-21`
  what_to_learn: The pattern of a small struct with primitive/String fields. Apply the same shape to the 4-field version.

External docs:
- url: master doc §75 (lines 4133-4170)
  section: simulate output format
  extract: confirm the wire format is a list of event records, not a `SimulationStep` struct.

Research questions (all answered):
- Q: How many baseline fields? A: 3 (not 4 as round-2 claimed).
- Q: What is the new spec? A: Add `kind: StepKind` and rename `kind_label` to `kind_label_text`.
- Q: Does master §75 specify `SimulationStep` fields? A: No, master §75 specifies the wire-format `events` output.

Research complete when:
- [x] All files_to_read opened.
- [x] All patterns_to_find searched.
- [x] All prior_art examined.
- [x] All research_questions have answers.

## Section 3. Inversions

### Security
- failure: A future bead adds a 5th field `capability_requirements: Vec<Capability>` to `SimulationStep`, leaking capability info to the simulate output.
  prevention: The 4 fields are FIXED by this bead. Adding a 5th field requires a master-doc amendment.
  test_for_it: `test_no_extra_fields: assert SimulationStep has exactly 4 fields and the names are exactly: index, kind_label_text, kind, description`.

### Usability
- failure: A developer reads the round-2 bead and tries to add 4 new fields, resulting in a 7-field struct.
  prevention: The new spec is to add ONLY 1 new field (`kind`) and rename 1 field (`kind_label` → `kind_label_text`). Total: 4 fields.
  test_for_it: `test_correct_field_count: assert SimulationStep has 4 fields, not 7 or 3`.

### Data Integrity
- failure: The `kind` and `kind_label_text` fields are out of sync (e.g., `kind = Do` but `kind_label_text = "Set"`).
  prevention: Both fields are populated in the same loop iteration in `simulate_workflow`. The `kind_label_text` comes from `node_kind_label(node.kind).to_string()`, which is derived from the same `node.kind`.
  test_for_it: `test_kind_and_label_in_sync: for each step, assert kind_label_text matches the label of the kind variant`.

### Integration Failure
- failure: A downstream tool (e.g., a JSON serializer) accesses `step.kind_label` (the old name) and gets a compile error.
  prevention: The rename is a single, atomic change. All call sites are updated in the same commit.
  test_for_it: `test_no_old_field_name: rg 'kind_label[^_]' crates/vb_cli/src/commands_workflow/` returns ZERO matches (only `kind_label_text`).

## Section 4. ATDD Tests

### Happy
- name: `test_simulate_do_step_has_kind_do`
  given: A workflow with a single `Do { action: ActionId::new(7) }` step.
  when: `simulate_workflow` is called.
  then: `steps[0].kind == StepKind::Do`, `steps[0].kind_label_text == "Do"`, `steps[0].description.contains("Do action 7")`.
  real_input: a minimal workflow with one Do step.
  expected_output: a `SimulationResult` with 1 step, kind=Do.
- name: `test_simulate_set_step_has_kind_setconst`
  given: A workflow with a `SetConst { value: 0, output: 0 }` step.
  when: `simulate_workflow` is called.
  then: `steps[0].kind == StepKind::SetConst`, `steps[0].description == "Set constant value"`.
  real_input: a workflow with one Set step.
  expected_output: a `SimulationResult` with 1 step, kind=SetConst.

### Error
- name: `test_simulate_empty_workflow_returns_zero_steps`
  given: An empty workflow.
  when: `simulate_workflow` is called.
  then: Returns `SimulationResult { steps: vec![], total_steps: 0, action_count: 0, branch_count: 0 }`.
  real_input: empty workflow.
  expected_output: empty SimulationResult.
- name: `test_simulate_malformed_workflow_returns_typed_error`
  given: A malformed workflow (compilation error).
  when: `simulate_workflow` is called.
  then: Returns `Err(CompileError)`.
  real_input: a workflow with an unknown step kind.
  expected_error: `Err(CompileError::UnsupportedStepPrimitive)`.

### Edge
- name: `test_simulate_all_step_kinds_have_a_label`
  given: A workflow with all 19+ step kinds.
  when: `simulate_workflow` is called.
  then: Every step has a non-empty `kind_label_text`.
  real_input: a workflow with all step kinds.
  expected: every `kind_label_text` is non-empty.
- name: `test_simulate_step_kind_and_label_in_sync`
  given: A workflow with mixed step kinds.
  when: `simulate_workflow` is called.
  then: For each step, `kind_label_text` matches the label of the `kind` variant.
  real_input: a workflow with Do, Set, Finish steps.
  expected: every step has `kind_label_text` matching the `kind` variant's label.

### Contract
- name: `test_precondition_simulation_step_has_exactly_4_fields`
  verifies: Precondition "SimulationStep has exactly 4 fields".
  test: assert the struct has 4 fields: index, kind_label_text, kind, description.
- name: `test_postcondition_kind_label_text_is_non_empty_for_all_kinds`
  verifies: Postcondition "kind_label_text is non-empty for all kinds".
  test: proptest with random step kinds; assert `kind_label_text` is non-empty.
- name: `test_invariant_kind_and_label_in_sync`
  verifies: Invariant "kind and kind_label_text are in sync".
  test: assert `kind_label_text == kind_to_label(kind)` for every step.

## Section 5. E2E Tests

```
pipeline_test:
  name: test_simulate_workflow_cli_e2e
  description: Real CLI invocation, real workflow, real output.
  setup:
    - create /tmp/test-workflow.yaml with Do, Set, Finish steps
  execute:
    command: "moon run -- vb simulate /tmp/test-workflow.yaml"
    timeout_ms: 5000
  verify:
    - exit_code: 0
    - stdout_contains: "Do" "Set" "Finish"
    - JSON output (if --json) has 3 steps with kind fields
  cleanup:
    - delete /tmp/test-workflow.yaml

e2e_scenarios:
  - name: e2e_simulate_mixed_workflow
    description: prove the kind and label are populated for mixed step kinds
    steps:
      - submit workflow with Do, Set, Finish
      - simulate
      - verify 3 steps with correct kinds
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] `commands_workflow/mod.rs:17-60` read"
    - "[x] `dot.rs` (node_kind_label) read"
    - "[x] Master §75 (lines 4133-4170) read"
    - "[x] Round-2 errors documented (4 fields claim, 4 new fields plan)"
  evidence_required:
    - "Research notes file with line-numbered extracts"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All 7 acceptance tests written (2 happy, 2 error, 2 edge, 3 contract)"
    - "[ ] Tests fail with compile error (StepKind does not exist yet)"
  evidence_required:
    - "Test file in `crates/vb_cli/src/commands_workflow/tests.rs`"
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
    - "[ ] E2E test passes with real CLI"
    - "[ ] No regressions in CLI tests"
  evidence_required:
    - "E2E output"
    - "Manual verification"

## Section 6. Implementation Tasks

### Phase 0: Research
- [ ] Read `commands_workflow/mod.rs:17-60` (parallel: research)
- [ ] Read `dot.rs` for `node_kind_label` (parallel: research)
- [ ] Read master §75 (lines 4133-4170) (parallel: research)
- [ ] Document the round-2 errors and the corrected spec (parallel: research)

### Phase 1: Tests
- [ ] Write `test_simulate_do_step_has_kind_do` (parallel: tests)
- [ ] Write `test_simulate_set_step_has_kind_setconst` (parallel: tests)
- [ ] Write `test_simulate_empty_workflow_returns_zero_steps` (parallel: tests)
- [ ] Write `test_simulate_malformed_workflow_returns_typed_error` (parallel: tests)
- [ ] Write `test_simulate_all_step_kinds_have_a_label` (parallel: tests)
- [ ] Write `test_simulate_step_kind_and_label_in_sync` (parallel: tests)
- [ ] Write 3 contract tests (parallel: tests)
- [ ] Confirm all 7 tests fail (gate)

### Phase 2: Implementation
- [ ] Define `StepKind` enum with all variants in `commands_workflow/mod.rs` (depends: tests; sequential)
- [ ] Add `kind: StepKind` field to `SimulationStep` (depends: enum; sequential)
- [ ] Rename `kind_label` to `kind_label_text` (depends: field; sequential)
- [ ] Add `node_kind_to_step_kind(&node.kind) -> StepKind` helper (depends: enum; sequential)
- [ ] Update `simulate_workflow` to populate `kind` and `kind_label_text` (depends: helper; sequential)
- [ ] Confirm all 7 tests pass (gate: green)

### Phase 3: Integration
- [ ] Write the E2E test (depends: impl; sequential)
- [ ] Run the E2E test (sequential)
- [ ] Run `cargo test -p vb_cli` to confirm no regressions (sequential)

### Phase 4: Documentation
- [ ] Run `moon run :ci` (depends: all of the above; parallel)
- [ ] Close the bead (sequential)

## Section 7. Failure Modes

- symptom: "Compile error: cannot find type `StepKind`"
  likely_cause: Test was written before the enum was defined.
  where_to_look:
    - file: `crates/vb_cli/src/commands_workflow/mod.rs`
    - function: any test referencing `StepKind`
    - what_to_check: "Is the enum defined with `pub(crate) enum StepKind`?"
  fix_pattern: Define the enum with all required variants.
- symptom: "Test fails: `kind` is set to a wrong variant (e.g., `StepKind::Do` for a `Set` step)"
  likely_cause: The `node_kind_to_step_kind` helper is missing or has a wrong mapping.
  where_to_look:
    - file: `crates/vb_cli/src/commands_workflow/mod.rs`
    - function: `node_kind_to_step_kind`
    - what_to_check: "Is the mapping correct for each `CompiledNodeKind` variant?"
  fix_pattern: Add the correct mapping; ensure all `CompiledNodeKind` variants are handled.
- symptom: "Test fails: `kind_label_text` is empty"
  likely_cause: The `node_kind_label` function is not being called, or returns an empty string.
  where_to_look:
    - file: `crates/vb_cli/src/commands_workflow/mod.rs`
    - function: `simulate_workflow`
    - what_to_check: "Is `node_kind_label(&node.kind).to_string()` being called?"
  fix_pattern: Confirm the call to `node_kind_label`.

debugging_commands:
- scenario: "When the kind is wrong"
  run: "rg 'node_kind_to_step_kind' crates/vb_cli/src/commands_workflow/"
  look_for: "All usages; verify the mapping is correct"
- scenario: "When the label is empty"
  run: "rg 'node_kind_label' crates/vb_cli/src/commands_workflow/"
  look_for: "All usages; verify the function returns a non-empty string for every variant"

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT claim 4 baseline fields — there are 3.
- DO NOT add `action_id`, `mock_output`, or `suspension_reason` to `SimulationStep`.
- DO NOT claim master §75 specifies `SimulationStep` fields — master §75 specifies the wire-format `events` output.
- DO NOT add 4 new fields — the spec is to add ONLY 1 new field (`kind`) and rename 1 field.

VERIFY that:
- `SimulationStep` has 3 fields before this bead: `rg 'pub.*struct SimulationStep' crates/vb_cli/src/commands_workflow/mod.rs` (must show 3 fields before impl; 4 after).
- `node_kind_label` exists in `dot.rs`: `rg 'fn node_kind_label' crates/vb_cli/src/commands_workflow/dot.rs` (must return 1 match).
- No old field name `kind_label` remains: `rg 'kind_label[^_]' crates/vb_cli/src/commands_workflow/` (must return ZERO matches after impl).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    rg 'StepKind|kind_label_text' crates/vb_cli/src/commands_workflow/  # confirm the new field and rename are wired

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-5dgth/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-5dgth/progress.txt` and continue from "Current Task". The field count is FIXED at 4; do not add more fields.
Key invariants:
- Baseline is 3 fields; new total is 4 fields.
- The new field is `kind: StepKind` (enum).
- The rename is `kind_label` → `kind_label_text`.
- The `StepKind` enum has at least 19 variants (Nop, SetConst, Copy, EvalExpr, BuildObject, BuildList, Do, Choose, ForEach, Together, Collect, Reduce, Repeat, WaitUntil, WaitEvent, Ask, RetryCheck, ErrorHandler, Finish).
- This bead has NO dependencies.

## Section 8. Completion Checklist

- [ ] All 7 acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing with real CLI
- [ ] No mocks or fake data in any test
- [ ] moon run :ci passes
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] No production code touched outside `crates/vb_cli/src/commands_workflow/mod.rs`
- [ ] bd close with reason: "P1-12r complete: SimulationStep has 4 fields (index, kind_label_text, kind, description)"

## Section 9. Context

Related files:
- `crates/vb_cli/src/commands_workflow/mod.rs:17-21` — `SimulationStep` (3 fields, the struct to modify)
- `crates/vb_cli/src/commands_workflow/mod.rs:23-28` — `SimulationResult` (the wrapper struct)
- `crates/vb_cli/src/commands_workflow/mod.rs:30-60` — `simulate_workflow` (the function to modify)
- `crates/vb_cli/src/commands_workflow/dot.rs:11` — `node_kind_label` (the re-exported helper)
- master doc §75 (lines 4133-4170) — wire-format `events` output (not the struct)

Similar implementations:
- The existing 3-field `SimulationStep` shows the pattern of a small struct with primitive/String fields. Apply the same shape to the 4-field version with the new `StepKind` enum.

Codebase patterns:
- pattern: "Small struct with primitive/String fields"
  example_location: `crates/vb_cli/src/commands_workflow/mod.rs:17-21`
  how_to_apply: Add the new `kind: StepKind` field; rename `kind_label` to `kind_label_text`; preserve `index` and `description`.

## Section 10. AI Hints

### DO
- Read `crates/vb_cli/src/commands_workflow/mod.rs:17-60` BEFORE writing any code. The struct is 4 lines; the read is fast.
- Define `StepKind` as `pub(crate) enum StepKind` (visible within the crate).
- Derive `Debug, Clone, Copy, PartialEq, Eq` on `StepKind` so it can be matched without allocation.
- Use a `match` statement in `node_kind_to_step_kind` to map `CompiledNodeKind` to `StepKind`.
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT use `unwrap()` or `expect()`.
- Do NOT add 4 new fields (the spec is to add 1 and rename 1).
- Do NOT add `action_id`, `mock_output`, or `suspension_reason`.
- Do NOT claim master §75 specifies `SimulationStep` fields.
- Do NOT use `unsafe`.

### Code patterns
- name: "Enum-backed kind field with derive"
  use_when: "Adding a typed enum field to a struct that previously had only a string label"
  example: |
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum StepKind { Nop, SetConst, Do, Finish, /* ... */ }
    pub(crate) struct SimulationStep {
        pub index: usize,
        pub kind_label_text: String,
        pub kind: StepKind,
        pub description: String,
    }

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect().
- Test first: Tests MUST exist and FAIL before implementation.
- Read before write: ALWAYS read the source file before modifying it.
- Real data only: Use real `CompiledNodeKind` and `node_kind_label`; no fabricated placeholders.
- Minimal change: ONE struct to modify; do NOT refactor the simulate module.
