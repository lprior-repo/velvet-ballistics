# S-20r expr-op-count

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> `crates/vb_core/src/workflow/types.rs:456-515` (ExprOp enum, 29 variants),
> `crates/vb_compile/src/expression_bytecode.rs:552-568` (helper_op function).

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications:
- The current `ExprOp` enum has EXACTLY 29 variants (verified by counting lines 458-514 in `types.rs`).
- The new variant is `Coalesce`, bringing the total to 30.
- The new variant must be added to:
  1. The `ExprOp` enum in `types.rs` (after `Unique` at line 514)
  2. The `helper_op` function in `expression_bytecode.rs:552-568` (mapping from `ExpressionHelper` to `ExprOp`)

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL add a new `Coalesce` variant to the `ExprOp` enum, making it the 30th variant.
- THE SYSTEM SHALL update the `helper_op` function to handle the new variant.
- THE SYSTEM SHALL preserve the existing 29 variants (regression-protected).

### Event-Driven
- WHEN `ExprOp` is enumerated (e.g., via `match`), THE SYSTEM SHALL handle the new `Coalesce` variant (the enum is `#[non_exhaustive]`, so adding a variant is non-breaking).
- WHEN `helper_op` is called with `ExpressionHelper::Coalesce`, THE SYSTEM SHALL return `ExprOp::Coalesce`.

### Unwanted
- THE SYSTEM SHALL NOT remove any of the existing 29 variants.
- THE SYSTEM SHALL NOT change the order of the existing variants.
- THE SYSTEM SHALL NOT remove the `#[non_exhaustive]` attribute (it allows non-breaking additions).
- THE SYSTEM SHALL NOT use a different name (e.g., `CoalesceOr`, `DefaultIfNull`); the master doc specifies `Coalesce`.

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs:
  - field: `op`
    type: `ExprOp`
    constraints: any of the 30 variants (29 existing + `Coalesce`).
    example_valid: `ExprOp::Coalesce`.
    example_invalid: N/A.
- system_state:
  - `ExprOp` has 29 variants at `types.rs:456-515`.
  - `helper_op` is at `expression_bytecode.rs:552-568`.
  - The enum is `#[non_exhaustive]`, so adding a variant is non-breaking.

### Postconditions
- state_changes:
  - The `ExprOp` enum has 30 variants after the change.
  - `helper_op` handles `ExpressionHelper::Coalesce -> ExprOp::Coalesce`.
- return_guarantees:
  - field: `ExprOp` (after the change)
    guarantee: Has exactly 30 variants.
- side_effects: None. The change is additive (a new variant).

### Invariants
- The existing 29 variants are preserved (regression).
- The order of the existing 29 variants is unchanged.
- The new `Coalesce` variant is added at the end (after `Unique`).
- The enum remains `#[non_exhaustive]`.

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: `crates/vb_core/src/workflow/types.rs:456-515`
  what_to_extract: The `ExprOp` enum. Confirm the 29 variants and the `#[non_exhaustive]` attribute.
  document_in: research_notes.md
- path: `crates/vb_compile/src/expression_bytecode.rs:552-568`
  what_to_extract: The `helper_op` function. Confirm the 14 helper variants (Contains, StartsWith, ..., Unique).
  document_in: research_notes.md
- path: `crates/vb_compile/src/expression_bytecode.rs` (other usages of `ExprOp`)
  what_to_extract: All `match` arms on `ExprOp` to ensure the new variant is handled.
  document_in: research_notes.md
- path: `crates/vb_core/src/workflow/types.rs:727-735`
  what_to_extract: The `validate_expr_op_count` function. Confirm the count check.
  document_in: research_notes.md

Patterns to find:
- pattern: `match.*ExprOp`
  purpose: Locate all match arms on `ExprOp` to ensure they handle the new variant.
  expected_locations: `crates/vb_compile/src/expression_bytecode.rs` and `crates/vb_core/src/workflow/types.rs`.
- pattern: `fn helper_op`
  purpose: Locate the helper mapping function.
  expected_locations: `crates/vb_compile/src/expression_bytecode.rs:552`.

Prior art:
- feature: existing 29-variant `ExprOp` enum
  location: `crates/vb_core/src/workflow/types.rs:456-515`
  what_to_learn: The pattern of variant ordering (loaders first, then comparison, then helpers). Apply the same pattern to the new variant.

External docs:
- url: master doc §75 (the master doc amendment specifying Coalesce)
  section: expression operators
  extract: confirm the new variant is named `Coalesce` (not `CoalesceOr` or similar).

Research questions (all answered):
- Q: How many variants does `ExprOp` have? A: 29 (verified by counting lines 458-514).
- Q: What is the new variant name? A: `Coalesce` (per the master doc amendment).
- Q: Where is the new variant added? A: At the end of the enum (after `Unique`).

Research complete when:
- [x] All files_to_read opened.
- [x] All patterns_to_find searched.
- [x] All prior_art examined.
- [x] All research_questions have answers.

## Section 3. Inversions

### Security
- failure: A future bead removes the `#[non_exhaustive]` attribute, breaking downstream crates that match on `ExprOp` exhaustively.
  prevention: The `#[non_exhaustive]` attribute is preserved. The new variant is added WITHOUT removing the attribute.
  test_for_it: `test_non_exhaustive_preserved: rg 'non_exhaustive' crates/vb_core/src/workflow/types.rs` returns 1 match.

### Usability
- failure: A developer adds the new variant at the wrong position (e.g., in the middle of the existing variants), changing the enum's internal ordering and breaking serialization.
  prevention: The new variant is added at the END of the enum (after `Unique`). All existing variants keep their position.
  test_for_it: `test_variant_order_preserved: assert the first 29 variants are unchanged; the 30th is `Coalesce``.

### Data Integrity
- failure: The `helper_op` function is not updated, so the new variant cannot be reached from a `ExpressionHelper`.
  prevention: The `helper_op` function is updated to map `ExpressionHelper::Coalesce -> ExprOp::Coalesce`.
  test_for_it: `test_helper_op_handles_coalesce: helper_op(ExpressionHelper::Coalesce) == ExprOp::Coalesce`.

### Integration Failure
- failure: A `match` arm on `ExprOp` is exhaustive WITHOUT the new variant, causing a compile error in downstream crates.
  prevention: The enum is `#[non_exhaustive]`, so exhaustive matches are already not allowed. Adding a new variant is non-breaking.
  test_for_it: `test_non_exhaustive_allows_addition: a downstream crate using `match op { ... }` without `Coalesce` still compiles (because of #[non_exhaustive])`.

## Section 4. ATDD Tests

### Happy
- name: `test_expr_op_enum_has_30_variants`
  given: The `ExprOp` enum after the change.
  when: Enumerated (e.g., via a `match` on all variants).
  then: Has exactly 30 variants.
  real_input: the updated enum.
  expected_output: 30 variants; the 30th is `Coalesce`.
- name: `test_helper_op_maps_coalesce_helper_to_coalesce_op`
  given: The `helper_op` function.
  when: Called with `ExpressionHelper::Coalesce`.
  then: Returns `ExprOp::Coalesce`.
  real_input: `ExpressionHelper::Coalesce`.
  expected_output: `ExprOp::Coalesce`.

### Error
- name: `test_helper_op_returns_error_for_unknown_helper`
  given: A `match` in `helper_op` that does NOT handle a hypothetical new helper.
  when: Called with the unknown helper.
  then: Returns `Err(CompileError::UnknownExpressionHelper)`.
  real_input: a synthetic unknown helper.
  expected_error: `Err(CompileError)`.
- name: `test_expr_op_serialization_round_trip`
  given: An `ExprOp::Coalesce` value.
  when: Serialized to bytes and deserialized.
  then: The result equals the original.
  real_input: `ExprOp::Coalesce`.
  expected_output: `ExprOp::Coalesce`.

### Edge
- name: `test_existing_29_variants_preserved`
  given: The 29 existing variants.
  when: Enumerated.
  then: All 29 are present in the same order.
  real_input: the existing variants.
  expected_output: 29 unchanged variants.
- name: `test_validate_expr_op_count_includes_coalesce`
  given: A program with 30 `ExprOp` values (one of each).
  when: `validate_expr_op_count` is called.
  then: Returns `Ok(())` (count is within `MAX_EXPRESSION_OPS`).
  real_input: 30 ops.
  expected_output: `Ok(())`.

### Contract
- name: `test_precondition_non_exhaustive_attribute_present`
  verifies: Precondition "the enum is `#[non_exhaustive]`".
  test: `rg 'non_exhaustive' crates/vb_core/src/workflow/types.rs` returns 1 match.
- name: `test_postcondition_helper_op_handles_coalesce`
  verifies: Postcondition "helper_op handles Coalesce".
  test: `helper_op(ExpressionHelper::Coalesce) == ExprOp::Coalesce`.
- name: `test_invariant_variant_count_is_30`
  verifies: Invariant "the enum has 30 variants".
  test: count the variants in the enum definition; assert == 30.

## Section 5. E2E Tests

```
pipeline_test:
  name: test_expr_op_coalesce_e2e
  description: Real CLI invocation, real workflow with a Coalesce expression.
  setup:
    - create /tmp/test-workflow.yaml with a Coalesce expression
  execute:
    command: "moon run -- vb compile /tmp/test-workflow.yaml"
    timeout_ms: 5000
  verify:
    - exit_code: 0
    - compiled expression bytecode contains ExprOp::Coalesce
  cleanup:
    - delete /tmp/test-workflow.yaml

e2e_scenarios:
  - name: e2e_coalesce_expression_compiles
    description: prove the new Coalesce variant is reachable
    steps:
      - submit a workflow with a Coalesce expression
      - compile it
      - assert the bytecode contains Coalesce
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] `types.rs:456-515` read (29 variants confirmed)"
    - "[x] `expression_bytecode.rs:552-568` read (helper_op confirmed)"
    - "[x] Master doc amendment for Coalesce read (name confirmed: Coalesce)"
  evidence_required:
    - "Research notes file with line-numbered extracts"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All 7 acceptance tests written (2 happy, 2 error, 2 edge, 3 contract)"
    - "[ ] Tests fail with compile error (Coalesce variant does not exist yet)"
  evidence_required:
    - "Test file in `crates/vb_core/src/workflow/tests.rs`"
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
    - "[ ] No regressions in compiler/runtime tests"
  evidence_required:
    - "E2E output"

## Section 6. Implementation Tasks

### Phase 0: Research
- [ ] Read `types.rs:456-515` (parallel: research)
- [ ] Read `expression_bytecode.rs:552-568` (parallel: research)
- [ ] Read master doc amendment for Coalesce (parallel: research)
- [ ] Count the existing 29 variants (parallel: research)

### Phase 1: Tests
- [ ] Write `test_expr_op_enum_has_30_variants` (parallel: tests)
- [ ] Write `test_helper_op_maps_coalesce_helper_to_coalesce_op` (parallel: tests)
- [ ] Write `test_helper_op_returns_error_for_unknown_helper` (parallel: tests)
- [ ] Write `test_expr_op_serialization_round_trip` (parallel: tests)
- [ ] Write `test_existing_29_variants_preserved` (parallel: tests)
- [ ] Write `test_validate_expr_op_count_includes_coalesce` (parallel: tests)
- [ ] Write 3 contract tests (parallel: tests)
- [ ] Confirm all 7 tests fail (gate)

### Phase 2: Implementation
- [ ] Add `Coalesce` variant to `ExprOp` enum in `types.rs` (after `Unique` at line 514) (depends: tests; sequential)
- [ ] Update `helper_op` in `expression_bytecode.rs:552-568` to handle `ExpressionHelper::Coalesce -> ExprOp::Coalesce` (depends: enum; sequential)
- [ ] Update any other `match` arms on `ExprOp` to handle the new variant (depends: helper; sequential)
- [ ] Confirm all 7 tests pass (gate: green)

### Phase 3: Integration
- [ ] Write the E2E test (depends: impl; sequential)
- [ ] Run the E2E test (sequential)
- [ ] Run `cargo test -p vb_core -p vb_compile` to confirm no regressions (sequential)

### Phase 4: Documentation
- [ ] Run `moon run :ci` (depends: all of the above; parallel)
- [ ] Close the bead (sequential)

## Section 7. Failure Modes

- symptom: "Compile error: cannot find variant `ExprOp::Coalesce`"
  likely_cause: The new variant was not added.
  where_to_look:
    - file: `crates/vb_core/src/workflow/types.rs:456-515`
    - what_to_check: "Is the `Coalesce` variant declared?"
  fix_pattern: Add the variant after `Unique`.
- symptom: "Test fails: helper_op returns CompileError for ExpressionHelper::Coalesce"
  likely_cause: The `helper_op` function was not updated.
  where_to_look:
    - file: `crates/vb_compile/src/expression_bytecode.rs:552-568`
    - what_to_check: "Is there an `ExpressionHelper::Coalesce => ExprOp::Coalesce` arm?"
  fix_pattern: Add the arm.
- symptom: "Test fails: serialization round-trip fails for ExprOp::Coalesce"
  likely_cause: The `Serialize`/`Deserialize` impls are not auto-derived for the new variant.
  where_to_look:
    - file: `crates/vb_core/src/workflow/types.rs:454`
    - what_to_check: "Is `Serialize, Deserialize` still derived?"
  fix_pattern: Confirm the derives are present.

debugging_commands:
- scenario: "When the variant is missing"
  run: "rg 'Coalesce' crates/vb_core/src/workflow/types.rs crates/vb_compile/src/expression_bytecode.rs"
  look_for: "The variant should be present in both files"
- scenario: "When the variant count is wrong"
  run: "rg 'pub enum ExprOp' crates/vb_core/src/workflow/types.rs -A 100 | rg '^\s+[A-Z][a-zA-Z]+' | wc -l"
  look_for: "Should be 30 (29 existing + Coalesce)"

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT remove any of the existing 29 variants.
- DO NOT change the order of the existing variants.
- DO NOT remove the `#[non_exhaustive]` attribute.
- DO NOT use a different name (e.g., `CoalesceOr`, `DefaultIfNull`).

VERIFY that:
- `ExprOp` has 29 variants before this bead: `rg 'pub enum ExprOp' crates/vb_core/src/workflow/types.rs -A 60 | rg '^\s+[A-Z]' | wc -l` (must return 29 before impl; 30 after).
- `helper_op` has 14 arms before this bead: `rg 'pub.*fn helper_op' crates/vb_compile/src/expression_bytecode.rs -A 20 | rg '=>' | wc -l` (must return 14 before impl; 15 after).
- The new variant name is exactly `Coalesce`: `rg 'Coalesce' crates/vb_core/src/workflow/types.rs` (must return 1 match after impl).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    rg 'Coalesce' crates/vb_core/src/workflow/types.rs crates/vb_compile/src/expression_bytecode.rs  # confirm the new variant is wired

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-rce3k/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-rce3k/progress.txt` and continue from "Current Task". The variant name is FIXED at `Coalesce`.
Key invariants:
- The new variant is `ExprOp::Coalesce` (NOT `CoalesceOr` or similar).
- The enum is `#[non_exhaustive]` (preserved).
- The existing 29 variants are UNCHANGED in name and order.
- The new variant is added at the END (after `Unique`).
- Total after: 30 variants.

## Section 8. Completion Checklist

- [ ] All 7 acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing with real CLI
- [ ] No mocks or fake data in any test
- [ ] moon run :ci passes
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] No production code touched outside `crates/vb_core/src/workflow/types.rs` and `crates/vb_compile/src/expression_bytecode.rs`
- [ ] bd remember note: "Round 3 black-hat APPROVED. 16-section content generated from source read."
- [ ] bd close with reason: "S-20r complete: ExprOp::Coalesce added; 30th variant; #[non_exhaustive] preserved"

## Section 9. Context

Related files:
- `crates/vb_core/src/workflow/types.rs:456-515` — `ExprOp` enum (29 variants; the enum to extend)
- `crates/vb_compile/src/expression_bytecode.rs:552-568` — `helper_op` function (the helper mapping to update)
- master doc §75 — the master doc amendment specifying `Coalesce`

Similar implementations:
- The existing 29 variants in `ExprOp` show the pattern of variant ordering. Apply the same pattern: add the new variant at the END.

Codebase patterns:
- pattern: "Enum with `#[non_exhaustive]` and derived Serialize/Deserialize"
  example_location: `crates/vb_core/src/workflow/types.rs:454-456`
  how_to_apply: Add a new variant to the enum; the `#[non_exhaustive]` attribute makes it non-breaking. Confirm the Serialize/Deserialize impls are auto-derived.

## Section 10. AI Hints

### DO
- Read `crates/vb_core/src/workflow/types.rs:456-515` BEFORE writing any code. The enum is 60 lines; the read is fast.
- Add the new `Coalesce` variant at the END of the enum (after `Unique`).
- Update `helper_op` to map `ExpressionHelper::Coalesce -> ExprOp::Coalesce`.
- Update any other `match` arms on `ExprOp` (e.g., in `apply_expr_stack_effect`).
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT use `unwrap()` or `expect()`.
- Do NOT remove any existing variant.
- Do NOT change the order of existing variants.
- Do NOT use a different name.
- Do NOT remove the `#[non_exhaustive]` attribute.
- Do NOT use `unsafe`.

### Code patterns
- name: "Add a new variant to a `#[non_exhaustive]` enum"
  use_when: "Non-breaking enum extension"
  example: |
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[non_exhaustive]
    pub enum ExprOp {
        // ... existing 29 variants ...
        /// `coalesce` helper.
        Coalesce,
    }

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect().
- Test first: Tests MUST exist and FAIL before implementation.
- Read before write: ALWAYS read the source file before modifying it.
- Real data only: Use real `ExpressionHelper` and `ExprOp`; no fabricated placeholders.
- Minimal change: ONE new variant + ONE new match arm; do NOT refactor the type system.
