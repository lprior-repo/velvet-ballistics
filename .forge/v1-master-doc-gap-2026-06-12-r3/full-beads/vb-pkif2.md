# P1-7r compiler-aliases

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> `crates/vb_compile/src/mod_compile_lowering/part_02.rs:28-101` (lower_canonical_step function),
> `crates/vb_compile/src/mod_compile_lowering/part_05.rs:374-381` (existing Save arm in digest).

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications:
- `Save` IS a `StepPrimitive` variant (verified in `crates/vb_compile/src/ast/types.rs:103-106` and `139`).
- `lower_canonical_step` at `part_02.rs:28-137` has match arms for `Set, Finish, ForEach, Together, Collect, Reduce, Repeat, Wait, Ask, Choose, Do`, but NO `Save` arm. The `Save` case falls through to the `other =>` catch-all and returns `Err(CompileError::UnsupportedStepPrimitive)`.
- `part_05.rs:374-381` already has an explicit `Save` arm in the digest function. The fix is to add a parallel arm in `part_02.rs:lower_canonical_step` to compile `Save` steps.
- The bead is titled "Save + Set" because both must be digest-identical (Save is the alias for Set, and both must produce the same hash).

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL add an explicit match arm for `vb_yaml::ast::StepPrimitive::Save { value }` in `lower_canonical_step` at `crates/vb_compile/src/mod_compile_lowering/part_02.rs:28`.
- THE SYSTEM SHALL preserve the existing `Set` arm (which currently exists at line 29-32).
- THE SYSTEM SHALL produce digest-identical output for `Save` and `Set` (master §25 aliasing invariant).

### Event-Driven
- WHEN `lower_canonical_step` is called with `step.primitive = StepPrimitive::Save { value }`, THE SYSTEM SHALL compile the step to a node (NOT fall through to the `other =>` catch-all).
- WHEN `lower_canonical_step` is called with `step.primitive = StepPrimitive::Set { output, value }`, THE SYSTEM SHALL continue to compile the step as before (regression-protected).
- WHEN the digest is computed for a `Save` step, THE SYSTEM SHALL produce the SAME hash as a `Set` step with the same value (aliasing invariant).

### Unwanted
- THE SYSTEM SHALL NOT remove the existing `Set` arm.
- THE SYSTEM SHALL NOT add a new file (the fix is in `part_02.rs:lower_canonical_step`).
- THE SYSTEM SHALL NOT add a new trait or method (the fix is a direct match arm).
- THE SYSTEM SHALL NOT panic on any `Save` input.

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs:
  - field: `step`
    type: `&vb_yaml::ast::StepAst`
    constraints: must have a `primitive` field.
    example_valid: `StepAst { primitive: StepPrimitive::Save { value: ScalarValue::String("hello") }, id: "step1", .. }`
    example_invalid: a malformed AST (the compiler catches it earlier).
  - field: `value`
    type: `&vb_yaml::ast::ScalarValue` (String or Integer)
    constraints: must be a valid scalar.
    example_valid: `ScalarValue::String("hello")` or `ScalarValue::Integer(42)`.
    example_invalid: `ScalarValue::Unsupported` (not yet supported).
- system_state:
  - `lower_canonical_step` is at `part_02.rs:28-137`.
  - The `Save` arm does NOT exist in `lower_canonical_step` (this bead adds it).
  - The `Save` arm DOES exist in `part_05.rs:374-381` (the digest function).

### Postconditions
- state_changes:
  - A new match arm `vb_yaml::ast::StepPrimitive::Save { value }` is added to `lower_canonical_step`.
  - The `Save` arm produces a node equivalent to the `Set` arm (digest identity).
- return_guarantees:
  - field: `Result<(), CompileErrors>`
    guarantee: `Ok(())` for a valid `Save` step (no longer falls through to `Err(UnsupportedStepPrimitive)`).
- side_effects: None. `lower_canonical_step` is a pure function (modulo the `builder` argument).

### Invariants
- For any `Save { value: v }` step, the digest matches `Set { value: v }` (aliasing invariant).
- The `Save` arm is added BEFORE the `other =>` catch-all to prevent fallthrough.
- The `Set` arm is preserved (regression).

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: `crates/vb_compile/src/mod_compile_lowering/part_02.rs:28-137`
  what_to_extract: The full `lower_canonical_step` function. Confirm the existing match arms and the `other =>` catch-all at line 131-136.
  document_in: research_notes.md
- path: `crates/vb_compile/src/mod_compile_lowering/part_05.rs:374-381`
  what_to_extract: The existing `Save` arm in the digest function. Confirm: `vb_yaml::ast::StepPrimitive::Save { value } => { hasher.update(b"save"); match value { String(v) => hasher.update(v.as_bytes()), Integer(v) => hasher.update(&v.to_le_bytes()), _ => hasher.update(b"unsupported") } }`.
  document_in: research_notes.md
- path: `crates/vb_compile/src/ast/types.rs:103-106, 139`
  what_to_extract: The `Save` variant of `StepPrimitive` (confirm it has a `value` field).
  document_in: research_notes.md
- path: `crates/vb_compile/src/tests/save_digest_unit_tests.rs`
  what_to_extract: The mutation-resistant tests for the `Save` arm. The tests are at the digest level (part_05.rs), but they imply the Save arm should produce a specific hash.
  document_in: research_notes.md
- path: `crates/vb_compile/src/mod_compile_lowering/part_02.rs:152-160` (and following)
  what_to_extract: The `lower_canonical_set` function. Confirm the signature and behavior.
  document_in: research_notes.md

Patterns to find:
- pattern: `StepPrimitive::Save`
  purpose: Locate all uses of the Save variant.
  expected_locations: `crates/vb_compile/src/ast/types.rs:103-106, 139`; `crates/vb_compile/src/mod_compile_lowering/part_05.rs:374-381`.
- pattern: `lower_canonical_set`
  purpose: Locate the Set lowering helper that the Save arm should delegate to (or replicate).
  expected_locations: `crates/vb_compile/src/mod_compile_lowering/part_02.rs:152`.

Prior art:
- feature: existing `Set` arm in `lower_canonical_step`
  location: `crates/vb_compile/src/mod_compile_lowering/part_02.rs:29-32`
  what_to_learn: The pattern of `match` + helper call. Apply the same shape to the new `Save` arm.
- feature: existing `Save` arm in the digest function
  location: `crates/vb_compile/src/mod_compile_lowering/part_05.rs:374-381`
  what_to_learn: The pattern of hashing `b"save" + value_bytes`. The lowering arm should produce a node that hashes the same way.

External docs:
- url: master doc §25 (aliasing invariant)
  section: compilation semantics
  extract: confirm "Save is an alias for Set; they must produce the same digest".

Research questions (all answered):
- Q: Does `Save` exist as a `StepPrimitive`? A: Yes, verified at `ast/types.rs:103-106`.
- Q: Is there a helper `lower_canonical_save`? A: No. The fix is to add a `Save` arm to `lower_canonical_step` that calls `lower_canonical_set` (or replicates the logic).
- Q: What is the digest for `Save`? A: `b"save" + value_bytes`, matching the existing `Save` arm in `part_05.rs:374-381`.

Research complete when:
- [x] All files_to_read opened.
- [x] All patterns_to_find searched.
- [x] All prior_art examined.
- [x] All research_questions have answers.

## Section 3. Inversions

### Security
- failure: An attacker uses `Save` (which is currently an alias error) to bypass the compiler's `Set` validation, smuggling a malicious value into the workflow.
  prevention: The new `Save` arm performs the same validation as `Set` (via the shared `lower_canonical_set` helper or equivalent logic).
  test_for_it: `test_save_with_invalid_value_rejected: Save { value: ScalarValue::Unsupported } -> Err(CompileError)`.

### Usability
- failure: A developer writes a `Save` step in YAML and gets `Err(UnsupportedStepPrimitive)`, even though master §25 says `Save` is supported.
  prevention: The new `Save` arm compiles the step successfully, matching master §25.
  test_for_it: `test_save_step_compiles: Save { value: "hello" } -> Ok(()) and produces a node`.

### Data Integrity
- failure: `Save` and `Set` produce different digests, violating the aliasing invariant and breaking deduplication.
  prevention: The `Save` arm produces a node that, when digested, produces the same hash as the corresponding `Set` step.
  test_for_it: `test_save_and_set_have_same_digest: digest(Save { value: "x" }) == digest(Set { value: "x" })`.

### Integration Failure
- failure: A downstream tool (e.g., a workflow visualizer) inspects the compiled node and expects `Set` nodes but receives `Save` nodes (different discriminators).
  prevention: The `Save` arm produces the SAME node type as the `Set` arm (e.g., `CompiledNodeKind::Set { ... }`); only the YAML-level aliasing differs.
  test_for_it: `test_save_produces_set_node: compile Save { value: "x" } -> produces a node of kind CompiledNodeKind::Set`.

## Section 4. ATDD Tests

### Happy
- name: `test_lower_canonical_step_with_save_compiles`
  given: A `StepAst` with `primitive = StepPrimitive::Save { value: ScalarValue::String("hello") }`.
  when: `lower_canonical_step` is called.
  then: Returns `Ok(())`; produces a `Set` node in the builder.
  real_input: `StepAst { primitive: Save { value: String("hello") }, id: "step1" }`.
  expected_output: `Ok(())`; 1 node added to the builder.
- name: `test_lower_canonical_step_with_set_still_compiles`
  given: A `StepAst` with `primitive = StepPrimitive::Set { output: "x", value: "hello" }`.
  when: `lower_canonical_step` is called.
  then: Returns `Ok(())`; produces a `Set` node (regression).
  real_input: `StepAst { primitive: Set { output: "x", value: "hello" }, id: "step1" }`.
  expected_output: `Ok(())`; 1 node added.

### Error
- name: `test_lower_canonical_step_with_save_unsupported_value_rejected`
  given: A `Save` step with an unsupported value type (e.g., `ScalarValue::Unsupported`).
  when: `lower_canonical_step` is called.
  then: Returns `Err(CompileError)` (no panic).
  real_input: `Save { value: ScalarValue::Unsupported }`.
  expected_error: `Err(CompileError::UnsupportedValueType)`.
- name: `test_lower_canonical_step_with_unknown_primitive_still_rejected`
  given: A `StepAst` with an unknown primitive (regression for the `other =>` catch-all).
  when: `lower_canonical_step` is called.
  then: Returns `Err(CompileError::UnsupportedStepPrimitive)`.
  real_input: a synthetic unknown primitive.
  expected_error: `Err(CompileError::UnsupportedStepPrimitive)`.

### Edge
- name: `test_save_and_set_have_same_digest`
  given: A `Save { value: "x" }` step and a `Set { value: "x" }` step.
  when: Both are compiled and digested.
  then: The digests are equal (aliasing invariant).
  real_input: both steps with the same value.
  expected: `digest_save == digest_set`.
- name: `test_save_with_integer_value`
  given: A `Save { value: ScalarValue::Integer(42) }` step.
  when: `lower_canonical_step` is called.
  then: Returns `Ok(())`; produces a `Set` node with the integer value.
  real_input: `Save { value: Integer(42) }`.
  expected_output: `Ok(())`.

### Contract
- name: `test_precondition_save_arm_exists_in_lower_canonical_step`
  verifies: Precondition "Save arm is added".
  test: `rg 'StepPrimitive::Save' crates/vb_compile/src/mod_compile_lowering/part_02.rs` returns 1 match after impl.
- name: `test_postcondition_save_produces_set_node`
  verifies: Postcondition "Save produces a Set node".
  test: assert the produced node's kind is `CompiledNodeKind::Set`.
- name: `test_invariant_save_and_set_digests_match`
  verifies: Invariant "Save and Set digests match".
  test: proptest with 100 random value pairs; assert `digest_save == digest_set`.

## Section 5. E2E Tests

```
pipeline_test:
  name: test_compile_save_alias_e2e
  description: Real CLI invocation, real workflow YAML with a Save step.
  setup:
    - create /tmp/test-workflow.yaml with a Save step
  execute:
    command: "moon run -- vb compile /tmp/test-workflow.yaml"
    timeout_ms: 5000
  verify:
    - exit_code: 0
    - compiled workflow has a Set node
  cleanup:
    - delete /tmp/test-workflow.yaml

e2e_scenarios:
  - name: e2e_save_and_set_produce_same_digest
    description: prove the aliasing invariant
    steps:
      - compile workflow A (uses Set)
      - compile workflow B (uses Save, equivalent values)
      - assert digest(A) == digest(B)
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] `part_02.rs:28-137` read (bug confirmed: no Save arm)"
    - "[x] `part_05.rs:374-381` read (existing Save arm in digest function)"
    - "[x] `ast/types.rs:103-106, 139` read (Save variant exists)"
    - "[x] `tests/save_digest_unit_tests.rs` read (digest tests are green)"
    - "[x] `part_02.rs:152-` read (lower_canonical_set helper exists)"
  evidence_required:
    - "Research notes file with line-numbered extracts"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All 7 acceptance tests written (2 happy, 2 error, 2 edge, 3 contract)"
    - "[ ] Tests fail with `Err(UnsupportedStepPrimitive)` for Save (the bug)"
  evidence_required:
    - "Test file in `crates/vb_compile/src/mod_compile_lowering/tests.rs`"
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
    - "[ ] No regressions in compiler tests"
  evidence_required:
    - "E2E output"
    - "Manual verification"

## Section 6. Implementation Tasks

### Phase 0: Research
- [ ] Read `part_02.rs:28-137` (parallel: research)
- [ ] Read `part_05.rs:374-381` (parallel: research)
- [ ] Read `ast/types.rs:103-106, 139` (parallel: research)
- [ ] Read `tests/save_digest_unit_tests.rs` (parallel: research)
- [ ] Read `part_02.rs:152-` for `lower_canonical_set` (parallel: research)

### Phase 1: Tests
- [ ] Write `test_lower_canonical_step_with_save_compiles` (parallel: tests)
- [ ] Write `test_lower_canonical_step_with_set_still_compiles` (parallel: tests)
- [ ] Write `test_lower_canonical_step_with_save_unsupported_value_rejected` (parallel: tests)
- [ ] Write `test_lower_canonical_step_with_unknown_primitive_still_rejected` (parallel: tests)
- [ ] Write `test_save_and_set_have_same_digest` (parallel: tests)
- [ ] Write `test_save_with_integer_value` (parallel: tests)
- [ ] Write 3 contract tests (parallel: tests)
- [ ] Confirm all 7 tests fail (gate)

### Phase 2: Implementation
- [ ] In `part_02.rs:28`, add a new match arm: `vb_yaml::ast::StepPrimitive::Save { value } => { /* delegate to lower_canonical_set or replicate logic */ }` (depends: tests; sequential)
- [ ] The arm must produce a `Set` node with the same value (aliasing invariant) (depends: arm; sequential)
- [ ] Add the arm BEFORE the `other =>` catch-all to prevent fallthrough (depends: arm; sequential)
- [ ] Confirm all 7 tests pass (gate: green)

### Phase 3: Integration
- [ ] Write the E2E test (depends: impl; sequential)
- [ ] Run the E2E test (sequential)
- [ ] Run `cargo test -p vb_compile` to confirm no regressions (sequential)

### Phase 4: Documentation
- [ ] Run `moon run :ci` (depends: all of the above; parallel)
- [ ] Close the bead (sequential)

## Section 7. Failure Modes

- symptom: "Test fails: Save step still returns Err(UnsupportedStepPrimitive)"
  likely_cause: The new Save arm was not added to `lower_canonical_step`.
  where_to_look:
    - file: `crates/vb_compile/src/mod_compile_lowering/part_02.rs:28-137`
    - what_to_check: "Is there a `StepPrimitive::Save { value }` arm?"
  fix_pattern: Add the arm BEFORE the `other =>` catch-all.
- symptom: "Test fails: Save and Set digests differ"
  likely_cause: The Save arm produces a different node type than Set (e.g., a new `Save` kind instead of reusing `Set`).
  where_to_look:
    - file: `crates/vb_compile/src/mod_compile_lowering/part_02.rs:28-137`
    - what_to_check: "Does the Save arm produce a `CompiledNodeKind::Set` node (or delegate to `lower_canonical_set`)?"
  fix_pattern: Make Save produce the same node as Set (delegate to `lower_canonical_set`).
- symptom: "Test fails: Set step now returns Err (regression)"
  likely_cause: The new Save arm accidentally catches Set steps.
  where_to_look:
    - file: `crates/vb_compile/src/mod_compile_lowering/part_02.rs:28-137`
    - what_to_check: "Is the Save arm only for `StepPrimitive::Save` (not `Set`)?"
  fix_pattern: Confirm the arm pattern is `StepPrimitive::Save { value }` only.

debugging_commands:
- scenario: "When Save still fails to compile"
  run: "rg 'StepPrimitive::Save' crates/vb_compile/src/mod_compile_lowering/part_02.rs"
  look_for: "The new arm should be present at the top of the match"
- scenario: "When digests differ"
  run: "rg 'StepPrimitive::Save' crates/vb_compile/src/mod_compile_lowering/part_05.rs"
  look_for: "Confirm the digest function hashes b\"save\" + value_bytes"

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT remove the existing `Set` arm.
- DO NOT add a new file (the fix is in `part_02.rs:lower_canonical_step`).
- DO NOT add a new trait or method.
- DO NOT use `unwrap()` or `expect()` in new code.

VERIFY that:
- `StepPrimitive::Save` exists: `rg 'Save,' crates/vb_compile/src/ast/types.rs` (must return 1 match).
- The Save arm in `part_05.rs` exists: `rg 'StepPrimitive::Save' crates/vb_compile/src/mod_compile_lowering/part_05.rs` (must return 1 match).
- The `Save` arm does NOT exist in `part_02.rs` before this bead: `rg 'StepPrimitive::Save' crates/vb_compile/src/mod_compile_lowering/part_02.rs` (must return ZERO matches before impl; 1 after).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    rg 'StepPrimitive::Save' crates/vb_compile/src/mod_compile_lowering/part_02.rs  # confirm the new arm is wired

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-pkif2/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-pkif2/progress.txt` and continue from "Current Task". The fix is a single match arm in `part_02.rs:lower_canonical_step`.
Key invariants:
- The new `Save` arm produces a `Set` node (aliasing invariant: Save and Set have the same digest).
- The `Set` arm is UNCHANGED.
- The new arm is BEFORE the `other =>` catch-all.
- The digest for Save is `b"save" + value_bytes` (matching `part_05.rs:374-381`).

## Section 8. Completion Checklist

- [ ] All 7 acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing with real CLI
- [ ] No mocks or fake data in any test
- [ ] moon run :ci passes
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] No production code touched outside `crates/vb_compile/src/mod_compile_lowering/part_02.rs`
- [ ] bd remember note: "Round 3 black-hat APPROVED. 16-section content generated from source read."
- [ ] bd close with reason: "P1-7r complete: Save arm added to lower_canonical_step; digest identity preserved"

## Section 9. Context

Related files:
- `crates/vb_compile/src/mod_compile_lowering/part_02.rs:28-137` — `lower_canonical_step` (the function to fix)
- `crates/vb_compile/src/mod_compile_lowering/part_02.rs:152-` — `lower_canonical_set` (the helper to delegate to)
- `crates/vb_compile/src/mod_compile_lowering/part_05.rs:374-381` — existing `Save` arm in digest function
- `crates/vb_compile/src/ast/types.rs:103-106, 139` — `StepPrimitive::Save` variant
- `crates/vb_compile/src/tests/save_digest_unit_tests.rs` — digest tests (currently green)
- master doc §25 — aliasing invariant

Similar implementations:
- The existing `Set` arm at `part_02.rs:29-32` shows the pattern of `match` + helper call. Apply the same shape to the new `Save` arm.
- The existing `Save` arm in `part_05.rs:374-381` shows the digest pattern: `b"save" + value_bytes`. The lowering arm should produce a node that hashes the same way.

Codebase patterns:
- pattern: "Match arm + helper call"
  example_location: `crates/vb_compile/src/mod_compile_lowering/part_02.rs:29-32`
  how_to_apply: Add a new arm BEFORE the `other =>` catch-all; delegate to a helper or replicate the logic.

## Section 10. AI Hints

### DO
- Read `crates/vb_compile/src/mod_compile_lowering/part_02.rs:28-137` BEFORE writing any code. The function is 110 lines; the read is fast.
- Add the new `Save` arm BEFORE the `other =>` catch-all.
- Make the new arm produce a `Set` node (delegate to `lower_canonical_set`).
- Verify the digest invariant: Save and Set must produce the same hash.
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT use `unwrap()` or `expect()`.
- Do NOT remove the existing `Set` arm.
- Do NOT add a new file.
- Do NOT add a new trait.
- Do NOT use `unsafe`.

### Code patterns
- name: "Alias match arm"
  use_when: "Adding a new alias for an existing primitive"
  example: |
    // In lower_canonical_step, before the `other =>` catch-all:
    vb_yaml::ast::StepPrimitive::Save { value } => {
        // Save is an alias for Set (master §25); produce a Set node.
        let slot = slot_idx_for_step(index).map_err(|e| CompileErrors(vec![e]))?;
        lower_canonical_set(id, slot, /* output */ "save_alias", value, next, outputs, builder)
    }

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect().
- Test first: Tests MUST exist and FAIL before implementation.
- Read before write: ALWAYS read the source file before modifying it.
- Real data only: Use real StepPrimitive variants; no fabricated placeholders.
- Minimal change: ONE match arm to add; do NOT refactor the compiler.
