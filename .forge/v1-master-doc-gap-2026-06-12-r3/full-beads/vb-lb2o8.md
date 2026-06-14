# P1-9r2 verify-15-gates

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> master doc §63 (lines 3053-3082 and 3148-3166), `crates/vb_cli/src/commands_verify.rs`.

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications:
- The 15 gate names are FIXED by master §63. The order is: profile, shape, names, references, expressions, CFG, bounded, budgets, contracts, taint, idempotency, durability, capabilities, results, evidence.
- No gate codes (hex) are specified in master §63. The 9 fabricated names from the rejected P1-9r are REMOVED entirely: `digest_stability`, `resource_contract_validation`, `error_handler_completeness`, `taint_boundary`, `input_purity`, `expression_complexity`, `cycle_detection`, `determinism_seed`, `replay_round_trip`.
- The output shape is `Vec<&'static str>` (existing VerifyOk.checks field at line 8-17). For unimplemented gates, push `"<gate_name>:deferred"`.

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL enumerate EXACTLY 15 named verification gates in the `VerifyOk.checks` field, matching master §63 verbatim and in order.
- THE SYSTEM SHALL preserve the existing `VerifyOk.checks: Vec<&'static str>` field type.
- THE SYSTEM SHALL NOT include any of the 9 fabricated gate names from the rejected P1-9r.

### Event-Driven
- WHEN `run_verification` is called with `VerifyProfile::Full` on a valid workflow YAML, THE SYSTEM SHALL return `VerifyOk` with `checks.len() == 15`.
- WHEN `run_verification` is called on a workflow with a missing `bounded` gate enforcement, THE SYSTEM SHALL push `"bounded:deferred"` for that gate.
- WHEN `run_verification` is called on a workflow with a missing `evidence` gate enforcement, THE SYSTEM SHALL push `"evidence:deferred"` for that gate.

### Unwanted
- THE SYSTEM SHALL NOT invent any new gate names (the 15 are FIXED by master §63).
- THE SYSTEM SHALL NOT include any hex codes (master §63 does not specify codes; the rejected bead fabricated `0x0E01..0x0F0F`).
- THE SYSTEM SHALL NOT include any of the 9 fabricated names: `digest_stability`, `resource_contract_validation`, `error_handler_completeness`, `taint_boundary`, `input_purity`, `expression_complexity`, `cycle_detection`, `determinism_seed`, `replay_round_trip`.

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs:
  - field: `VerifyProfile`
    type: `enum { Full, Minimal, ... }` (defined in `commands_verify.rs`)
    constraints: For `Full`, all 15 gates must appear in the output.
    example_valid: `VerifyProfile::Full`
    example_invalid: `VerifyProfile::Unknown` (does not exist; the enum is exhaustive)
- system_state:
  - `VerifyOk.checks: Vec<&'static str>` exists at `commands_verify.rs:8-17`.
  - Master §63 enumerates the 15 gates in order at lines 3053-3082.

### Postconditions
- state_changes:
  - The `checks.push(...)` calls in `commands_verify.rs:73-122` are replaced with the 15-gate enumeration.
  - The order of the 15 gates matches master §63 exactly.
- return_guarantees:
  - field: `VerifyOk.checks`
    guarantee: For `VerifyProfile::Full`, the Vec has exactly 15 elements.
    guarantee: The first element is `"profile"`, the second is `"shape"`, ..., the 15th is `"evidence"`.
- side_effects: None. `run_verification` is a pure function over the input workflow.

### Invariants
- The 15 gate names are in the EXACT order from master §63. No reordering.
- The 15 gate names are the ONLY names that appear in the output (no fabricated names).
- For `VerifyProfile::Full`, the output is deterministic: same input produces the same 15-element Vec.

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: `master doc §63` (lines 3053-3082)
  what_to_extract: The 15 named gates and their descriptions. The order is: profile, shape, names, references, expressions, CFG, bounded, budgets, contracts, taint, idempotency, durability, capabilities, results, evidence.
  document_in: research_notes.md
- path: `master doc §63` (lines 3148-3166) — the gate status table
  what_to_extract: Confirm the 15-gate order from the table: profile, shape, names, references, expressions, CFG, boundedness, resource budget, action contract, secret/taint, idempotency, durability, capability, output/result, observability. Note: the table uses `boundedness` but the gate is `bounded` (master §63 line 3066).
  document_in: research_notes.md
- path: `crates/vb_cli/src/commands_verify.rs:8-17`
  what_to_extract: The `VerifyOk` struct and the `pub checks: Vec<&'static str>` field.
  document_in: research_notes.md
- path: `crates/vb_cli/src/commands_verify.rs:70-122`
  what_to_extract: The current `checks.push(...)` calls. The current implementation produces 5-6 names; this bead replaces them with 15.
  document_in: research_notes.md

Patterns to find:
- pattern: `checks.push`
  purpose: Locate all 5-6 existing push calls that need to be replaced.
  expected_locations: `crates/vb_cli/src/commands_verify.rs:73-122`.
- pattern: `VerifyProfile`
  purpose: Confirm the enum variants (`Full`, `Minimal`, etc.).
  expected_locations: `crates/vb_cli/src/commands_verify.rs` (likely near the top).

Prior art:
- feature: existing 5-6 gate names in `commands_verify.rs:73-122`
  location: `crates/vb_cli/src/commands_verify.rs:73-122`
  what_to_learn: The pattern of pushing `&'static str` to the `checks` Vec. Apply the same pattern to 15 gates.

External docs:
- url: master doc §63 (verified line range)
  section: verification gate pipeline
  extract: the 15 gate names and their descriptions.

Research questions (all answered):
- Q: Is the order "bounded" or "boundedness"? A: `bounded` (master §63 line 3066 uses `bounded`; the status table at 3148-3166 uses `boundedness` as a description but the gate name is `bounded`).
- Q: Should we use a structured enum or `&'static str`? A: `&'static str` (existing field type; minimal change).

Research complete when:
- [x] All files_to_read opened.
- [x] All patterns_to_find searched.
- [x] All prior_art examined.
- [x] All research_questions have answers.

## Section 3. Inversions

### Security
- failure: A future bead adds a 16th gate with a name that collides with one of the 9 fabricated names (e.g., `digest_stability` is added back), causing operator confusion.
  prevention: The 15 gate names are HARD-CODED from master §63. Adding a 16th gate requires a master-doc amendment, not a code change.
  test_for_it: `test_no_fabricated_gate_names: assert all 15 names are from the master §63 list; assert NONE of the 9 fabricated names appear in the output`.

### Usability
- failure: An operator runs `vb verify --profile=full` and sees 15 gate names but doesn't know what each one means.
  prevention: The 15 gate names match master §63 verbatim; the master doc is the source of truth for descriptions. Add a doc comment in the code referencing master §63.
  test_for_it: `test_gate_names_match_master_doc: assert each of the 15 names is documented in master §63 (check via grep on the master doc)`.

### Data Integrity
- failure: A bug in the implementation reorders the 15 gates (e.g., alphabetical sort), breaking downstream tools that rely on the master §63 order.
  prevention: The order is FIXED; the test asserts the exact order. Any reordering fails the test.
  test_for_it: `test_exact_gate_order: assert checks[0] == "profile" && checks[1] == "shape" && ... && checks[14] == "evidence"`.

### Integration Failure
- failure: A downstream tool parses the `checks` Vec and assumes exactly 6 elements (the old count). The change to 15 breaks the tool.
  prevention: The test asserts `checks.len() == 15`; downstream tools that depend on the old count are updated as part of the same change. Document the breaking change in the commit message.
  test_for_it: `test_count_is_15: assert checks.len() == 15; document breaking change in the bead close reason`.

## Section 4. ATDD Tests

### Happy
- name: `test_verify_produces_15_gates_for_full_profile`
  given: A valid workflow YAML at `/tmp/test-workflow.yaml`.
  when: `run_verification(workflow, VerifyProfile::Full)` is called.
  then: Returns `VerifyOk { checks: Vec<&str> }` with `checks.len() == 15` and the 15 names in master §63 order.
  real_input: a minimal valid workflow YAML with one Set step and one Do step.
  expected_output: `Vec` of length 15 with elements: `"profile"`, `"shape"`, `"names"`, `"references"`, `"expressions"`, `"CFG"`, `"bounded"`, `"budgets"`, `"contracts"`, `"taint"`, `"idempotency"`, `"durability"`, `"capabilities"`, `"results"`, `"evidence"`.
- name: `test_verify_minimal_profile_produces_subset`
  given: A valid workflow YAML.
  when: `run_verification(workflow, VerifyProfile::Minimal)` is called.
  then: Returns a `Vec` of length 5 (the 5 most-critical gates: profile, shape, bounded, contracts, evidence).
  real_input: same workflow as above.
  expected_output: `Vec` of length 5 with the documented minimal subset.

### Error
- name: `test_verify_empty_workflow_returns_15_gates_with_all_deferred`
  given: An empty workflow YAML.
  when: `run_verification(workflow, VerifyProfile::Full)` is called.
  then: Returns a `Vec` of length 15 where every element is `"<gate_name>:deferred"`.
  real_input: empty YAML.
  expected_output: `Vec` of 15 `"<name>:deferred"` strings.
- name: `test_verify_invalid_workflow_yaml_returns_typed_error`
  given: An invalid YAML file (syntax error).
  when: `run_verification(workflow, VerifyProfile::Full)` is called.
  then: Returns `Err(YamlParseError)`.
  real_input: `"this: is: invalid: yaml: ["`.
  expected_error: `Err(VerifyError::YamlParse)`.

### Edge
- name: `test_verify_15_gates_in_exact_master_section_63_order`
  given: A valid workflow.
  when: `run_verification(workflow, VerifyProfile::Full)` is called.
  then: The order of the 15 gates is EXACTLY: profile, shape, names, references, expressions, CFG, bounded, budgets, contracts, taint, idempotency, durability, capabilities, results, evidence.
  real_input: a minimal workflow.
  expected_output: exact order asserted.
- name: `test_verify_no_fabricated_gate_names_appear_in_output`
  given: A valid workflow.
  when: `run_verification(workflow, VerifyProfile::Full)` is called.
  then: NONE of the 9 fabricated names appear in the output: `digest_stability`, `resource_contract_validation`, `error_handler_completeness`, `taint_boundary`, `input_purity`, `expression_complexity`, `cycle_detection`, `determinism_seed`, `replay_round_trip`.
  real_input: a minimal workflow.
  expected: a Vec containing only the 15 master §63 names.

### Contract
- name: `test_precondition_full_profile_must_yield_15_gates`
  verifies: Precondition "for Full, the output has 15 elements".
  test: assert `checks.len() == 15` for `VerifyProfile::Full`.
- name: `test_postcondition_gate_order_matches_master_section_63`
  verifies: Postcondition "the order is the master §63 order".
  test: assert each `checks[i] == MASTER_GATES[i]` for `i in 0..15`.
- name: `test_invariant_only_master_section_63_names_appear`
  verifies: Invariant "no fabricated names".
  test: assert all 15 elements are in the master §63 allow-list.

## Section 5. E2E Tests

```
pipeline_test:
  name: test_verify_cli_15_gates_e2e
  description: Real CLI invocation, real workflow, real output.
  setup:
    - create /tmp/test-workflow.yaml with a minimal valid workflow
  execute:
    command: "moon run -- vb verify --profile=full /tmp/test-workflow.yaml"
    timeout_ms: 5000
  verify:
    - exit_code: 0
    - stdout_contains: "15"  (the count)
    - stdout_contains: "profile" "shape" "names" "references" "expressions" "CFG" "bounded" "budgets" "contracts" "taint" "idempotency" "durability" "capabilities" "results" "evidence"
  cleanup:
    - delete /tmp/test-workflow.yaml

e2e_scenarios:
  - name: e2e_cli_verify_full_profile_count
    description: prove the count is 15 via the real CLI
    steps:
      - run `vb verify --profile=full <yaml>`
      - parse the output
      - assert exactly 15 gate names appear
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] Master §63 (lines 3053-3082 and 3148-3166) read and parsed"
    - "[x] Existing `commands_verify.rs:8-17` and `70-122` read"
    - "[x] All 9 fabricated names from rejected P1-9r documented as REMOVED"
  evidence_required:
    - "Research notes file with line-numbered extracts"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All 8 acceptance tests written (2 happy, 2 error, 2 edge, 3 contract)"
    - "[ ] Tests fail with the expected output (current 5-6 gates, not 15)"
  evidence_required:
    - "Test file in `crates/vb_cli/src/commands_verify.rs` test module"
    - "Test output showing length mismatch (5-6 != 15)"

### Implementation
- name: "Implementation Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] All 8 tests pass"
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
- [ ] Read master doc §63 (lines 3053-3082) (parallel: research)
- [ ] Read master doc §63 (lines 3148-3166) (parallel: research)
- [ ] Read `commands_verify.rs:8-17` (parallel: research)
- [ ] Read `commands_verify.rs:70-122` (parallel: research)
- [ ] Document the 9 fabricated names to REMOVE (parallel: research)

### Phase 1: Tests
- [ ] Write `test_verify_produces_15_gates_for_full_profile` (parallel: tests)
- [ ] Write `test_verify_minimal_profile_produces_subset` (parallel: tests)
- [ ] Write `test_verify_empty_workflow_returns_15_gates_with_all_deferred` (parallel: tests)
- [ ] Write `test_verify_invalid_workflow_yaml_returns_typed_error` (parallel: tests)
- [ ] Write `test_verify_15_gates_in_exact_master_section_63_order` (parallel: tests)
- [ ] Write `test_verify_no_fabricated_gate_names_appear_in_output` (parallel: tests)
- [ ] Write 3 contract tests (parallel: tests)
- [ ] Confirm all 8 tests fail (gate)

### Phase 2: Implementation
- [ ] Replace the `checks.push(...)` calls in `commands_verify.rs:73-122` with the 15-gate enumeration (depends: tests; sequential)
- [ ] Use `let gates: [&str; 15] = ["profile", "shape", "names", "references", "expressions", "CFG", "bounded", "budgets", "contracts", "taint", "idempotency", "durability", "capabilities", "results", "evidence"];` and `checks.extend_from_slice(&gates);` (depends: replace; sequential)
- [ ] For unimplemented gates, append `":deferred"` to the name (depends: enumeration; sequential)
- [ ] Confirm all 8 tests pass (gate: green)

### Phase 3: Integration
- [ ] Write the E2E test (depends: impl; sequential)
- [ ] Run the E2E test (sequential)
- [ ] Run `cargo test -p vb_cli` to confirm no regressions (sequential)

### Phase 4: Documentation
- [ ] Run `moon run :ci` (depends: all of the above; parallel)
- [ ] Close the bead (sequential)

## Section 7. Failure Modes

- symptom: "Test fails: checks.len() == 5, not 15"
  likely_cause: The 15-gate enumeration was not applied. The old 5-6 gates are still being pushed.
  where_to_look:
    - file: `crates/vb_cli/src/commands_verify.rs:73-122`
    - function: `run_verification`
    - what_to_check: "Is the new `extend_from_slice(&gates)` call present?"
  fix_pattern: Replace the 5-6 `checks.push(...)` calls with a single `extend_from_slice(&gates)`.
- symptom: "Test fails: gate order is wrong (e.g., 'bounded' appears before 'CFG')"
  likely_cause: The 15-element array was sorted alphabetically instead of using the master §63 order.
  where_to_look:
    - file: `crates/vb_cli/src/commands_verify.rs`
    - function: the array literal
    - what_to_check: "Is the array in the order from master §63?"
  fix_pattern: Reorder the array literal to match master §63 exactly.
- symptom: "Test fails: a fabricated name (e.g., `digest_stability`) appears in the output"
  likely_cause: A copy-paste from the rejected P1-9r bead leaked a fabricated name.
  where_to_look:
    - file: `crates/vb_cli/src/commands_verify.rs`
    - function: the array literal
    - what_to_check: "Are all 15 names in the master §63 allow-list?"
  fix_pattern: Remove the fabricated name from the array.

debugging_commands:
- scenario: "When the count is wrong"
  run: "rg 'checks.push' crates/vb_cli/src/commands_verify.rs"
  look_for: "Count the number of `checks.push` calls; should be 0 (replaced by extend_from_slice) or 15 (one per gate)"

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT invent any new gate names. The 15 are FIXED by master §63.
- DO NOT invent hex codes (master §63 does not specify codes).
- DO NOT include any of the 9 fabricated names: `digest_stability`, `resource_contract_validation`, `error_handler_completeness`, `taint_boundary`, `input_purity`, `expression_complexity`, `cycle_detection`, `determinism_seed`, `replay_round_trip`.

VERIFY that:
- The 15 names match master §63: `rg '"profile"' master doc` (must find it in the verification gate pipeline section).
- No fabricated names: `rg 'digest_stability|resource_contract_validation|error_handler_completeness|taint_boundary|input_purity|expression_complexity|cycle_detection|determinism_seed|replay_round_trip' crates/` (must return ZERO matches in `commands_verify.rs`).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    rg 'checks\.push|extend_from_slice' crates/vb_cli/src/commands_verify.rs  # confirm the 15-gate enumeration is wired

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-lb2o8/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-lb2o8/progress.txt` and continue from "Current Task". The 15-gate order is CRITICAL; do not change it.
Key invariants:
- The 15 gate names are HARD-CODED from master §63.
- The order is FIXED: profile, shape, names, references, expressions, CFG, bounded, budgets, contracts, taint, idempotency, durability, capabilities, results, evidence.
- For unimplemented gates, append `":deferred"` to the name.

## Section 8. Completion Checklist

- [ ] All 8 acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing with real CLI
- [ ] No mocks or fake data in any test
- [ ] moon run :ci passes
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] No production code touched outside `crates/vb_cli/src/commands_verify.rs`
- [ ] bd close with reason: "P1-9r2 complete: 15-gate enumeration matches master §63"

## Section 9. Context

Related files:
- `crates/vb_cli/src/commands_verify.rs:8-17` — `VerifyOk` struct (output shape)
- `crates/vb_cli/src/commands_verify.rs:70-122` — `run_verification` (the function to modify)
- master doc §63 (lines 3053-3082) — the 15 gate names
- master doc §63 (lines 3148-3166) — the gate status table (confirms order)

Similar implementations:
- The existing 5-6 `checks.push` calls show the pattern of pushing `&'static str`. Replace with a single `extend_from_slice` of a 15-element array.

Codebase patterns:
- pattern: "Static array of gate names"
  example_location: `crates/vb_cli/src/commands_verify.rs:73-122` (current 5-6 push calls)
  how_to_apply: Replace the 5-6 calls with a single `extend_from_slice(&[...15 names...])`.

## Section 10. AI Hints

### DO
- Read master doc §63 (lines 3053-3082) BEFORE writing any code. The 15 names are FIXED; the read is critical.
- Use a static array `const GATES: [&str; 15] = [...];` to make the order explicit and verifiable.
- Append `":deferred"` to gate names that are not yet enforced (this is the documented pattern).
- Reference master §63 in the code's doc comment.
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT invent new gate names.
- Do NOT invent hex codes.
- Do NOT include any of the 9 fabricated names.
- Do NOT use `unwrap()` or `expect()`.
- Do NOT reorder the gates.
- Do NOT use `unsafe`.

### Code patterns
- name: "Static array of gate names with extend_from_slice"
  use_when: "Enumerating a fixed list of items in a specific order"
  example: |
    const VERIFICATION_GATES: [&str; 15] = [
        "profile", "shape", "names", "references", "expressions",
        "CFG", "bounded", "budgets", "contracts", "taint",
        "idempotency", "durability", "capabilities", "results", "evidence",
    ];
    checks.extend_from_slice(&VERIFICATION_GATES);

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect().
- Test first: Tests MUST exist and FAIL before implementation.
- Read before write: ALWAYS read master §63 BEFORE writing any code.
- Real data only: The 15 gate names are real master §63 names; no fabricated placeholders.
- Minimal change: ONE function to modify; do NOT refactor the CLI.
