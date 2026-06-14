# S-19r vb-benchmark-cleanup

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> `crates/vb_benchmark/tests/benchmark_tests.rs` (lines 1-609), `crates/vb_benchmark/src/` (directory listing).

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications (round-2 corrections applied):
- The 11 STUB: markers in `benchmark_tests.rs` use the format `// STUB: <function_name> <description>`, NOT `// STUB: This test will FAIL`.
- `aggregate_resource_budget` does NOT exist as a source file — it must be CREATED at `crates/vb_benchmark/src/aggregate_resource_budget.rs`.
- The Criterion bench harness does NOT exist — it must be CREATED at `crates/vb_benchmark/benches/aggregate_resource_budget.rs` (and the `benches/` directory added to `Cargo.toml`).

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL replace the 11 STUB: function bodies in `crates/vb_benchmark/tests/benchmark_tests.rs` with real assertions.
- THE SYSTEM SHALL create `crates/vb_benchmark/src/aggregate_resource_budget.rs` (NEW) that exports `pub fn aggregate_resource_budget(runs: &[RunMetrics]) -> ResourceBudgetReport`.
- THE SYSTEM SHALL create `crates/vb_benchmark/benches/aggregate_resource_budget.rs` (NEW) that is a Criterion bench harness.
- THE SYSTEM SHALL add a regression-shield test that asserts the STUB: count in `benchmark_tests.rs` is 0.

### Event-Driven
- WHEN `baseline_within_budget(actual, budget_us)` is called, THE SYSTEM SHALL return `actual.as_micros() <= budget_us` (real assertion, not `false`).
- WHEN `budget_utilization_percent(actual, budget_us)` is called, THE SYSTEM SHALL compute `(actual.as_micros() * 10_000) / budget_us` (basis points).
- WHEN `latency_within_budget(elapsed, budget_us)` is called, THE SYSTEM SHALL return `elapsed.as_micros() <= budget_us`.
- WHEN `result_exceeds_threshold(result, baseline, delta)` is called, THE SYSTEM SHALL return `result > baseline + delta`.
- WHEN `check_evidence_gate(evidence_path)` is called, THE SYSTEM SHALL verify the evidence file exists, contains a baseline, and the regression is within threshold.

### Unwanted
- THE SYSTEM SHALL NOT cite `crates/vb_benchmark/src/aggregate_resource_budget.rs` as existing — it does NOT. It must be CREATED.
- THE SYSTEM SHALL NOT cite `crates/vb_benchmark/benches/` as existing — it does NOT. The directory must be CREATED.
- THE SYSTEM SHALL NOT use `// STUB: This test will FAIL` as the pattern — the actual text is `// STUB: <function_name> <description>`.

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs:
  - field: `runs`
    type: `&[RunMetrics]`
    constraints: 10-100 runs with resource usage data.
    example_valid: 10 `RunMetrics { cpu_us: 1000, memory_bytes: 1024, ... }` entries.
    example_invalid: empty slice (degenerate; the function should handle gracefully).
  - field: `actual`
    type: `Duration`
    constraints: a measured duration.
    example_valid: `Duration::from_micros(500)`.
    example_invalid: N/A.
  - field: `budget_us`
    type: `u64`
    constraints: a budget in microseconds.
    example_valid: `1000`.
    example_invalid: `0` (the function should handle as "infinite budget").
- system_state:
  - `benchmark_tests.rs` has 11 STUB: markers at lines 10, 20, 31, 41, 51, 58, 68, 79, 90, 107, 127.
  - `crates/vb_benchmark/src/` contains only `error.rs` and `lib.rs`.
  - `crates/vb_benchmark/benches/` does NOT exist.

### Postconditions
- state_changes:
  - The 11 STUB: function bodies in `benchmark_tests.rs` are replaced with real assertions.
  - `crates/vb_benchmark/src/aggregate_resource_budget.rs` is created.
  - `crates/vb_benchmark/benches/aggregate_resource_budget.rs` is created.
  - `Cargo.toml` is updated to include the new bench target.
- return_guarantees:
  - field: `baseline_within_budget`
    guarantee: Returns `true` iff `actual.as_micros() <= budget_us`.
  - field: `budget_utilization_percent`
    guarantee: Returns `(actual.as_micros() * 10_000) / budget_us` (basis points).
- side_effects: None for the assertions; the bench file creates a temp database for measurement.

### Invariants
- After the cleanup, the STUB: count in `benchmark_tests.rs` is 0.
- The `aggregate_resource_budget` function aggregates resource usage across multiple runs.
- The bench harness is a real Criterion bench (not a stub).

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: `crates/vb_benchmark/tests/benchmark_tests.rs:1-609`
  what_to_extract: The 11 STUB: function bodies and the ACTUAL STUB: text format. Confirmed: `// STUB: <function_name> <description>`.
  document_in: research_notes.md
- path: `crates/vb_benchmark/src/` (directory listing)
  what_to_extract: Confirm the directory contains ONLY `error.rs` and `lib.rs`. NO `aggregate_resource_budget.rs`.
  document_in: research_notes.md
- path: `crates/vb_benchmark/benches/` (directory listing)
  what_to_extract: Confirm the directory does NOT exist.
  document_in: research_notes.md
- path: `crates/vb_benchmark/Cargo.toml`
  what_to_extract: Existing dependencies and bench targets.
  document_in: research_notes.md
- path: `crates/vb_benchmark/src/lib.rs`
  what_to_extract: The current public API. The new `aggregate_resource_budget` must be exported here.
  document_in: research_notes.md

Patterns to find:
- pattern: `STUB:`
  purpose: Locate all 11 STUB: markers in `benchmark_tests.rs`.
  expected_locations: `crates/vb_benchmark/tests/benchmark_tests.rs:10, 20, 31, 41, 51, 58, 68, 79, 90, 107, 127`.
- pattern: `aggregate_resource_budget`
  purpose: Verify the function does NOT exist; this bead creates it.
  expected_locations: NONE — does not exist.
- pattern: `crates/vb_benchmark/benches/`
  purpose: Verify the directory does NOT exist.
  expected_locations: NONE — does not exist.

Prior art:
- feature: existing 11 STUB: function bodies in `benchmark_tests.rs`
  location: `crates/vb_benchmark/tests/benchmark_tests.rs:1-609`
  what_to_learn: The pattern of stub functions. The cleanup replaces them with real assertions.

External docs:
- url: Criterion docs (https://github.com/bheisler/criterion.rs)
  section: bench harness
  extract: the `criterion_group!` and `criterion_main!` macros.

Research questions (all answered):
- Q: What is the STUB: text format? A: `// STUB: <function_name> <description>` (not `// STUB: This test will FAIL`).
- Q: Does `aggregate_resource_budget.rs` exist? A: No, it must be CREATED.
- Q: Does `benches/` exist? A: No, the directory must be CREATED.

Research complete when:
- [x] All files_to_read opened.
- [x] All patterns_to_find searched.
- [x] All prior_art examined.
- [x] All research_questions have answers.

## Section 3. Inversions

### Security
- failure: A STUB: function (e.g., `baseline_within_budget`) returns `false` always, allowing an over-budget operation to pass CI.
  prevention: The STUB: bodies are replaced with REAL assertions. The regression-shield test asserts the STUB: count is 0.
  test_for_it: `test_no_stub_markers: rg 'STUB:' crates/vb_benchmark/tests/ | wc -l` returns 0.

### Usability
- failure: A developer reads the round-2 bead and tries to find `aggregate_resource_budget.rs` as an existing file, but it does not exist.
  prevention: The bead creates the file. The anti-hallucination guard explicitly notes the file must be created.
  test_for_it: `test_file_exists: rg 'pub fn aggregate_resource_budget' crates/vb_benchmark/src/` returns 1 match after impl.

### Data Integrity
- failure: The `aggregate_resource_budget` function returns the wrong aggregate (e.g., `sum` instead of `median`).
  prevention: The function is documented in the bead to return the correct aggregate. The test asserts the expected behavior.
  test_for_it: `test_aggregate_uses_correct_method: assert the function returns the median (or whatever the documented behavior is)`.

### Integration Failure
- failure: The new bench file is created but not registered in `Cargo.toml`, so `cargo bench` does not find it.
  prevention: The bead updates `Cargo.toml` to register the new bench target.
  test_for_it: `test_bench_registered: rg 'aggregate_resource_budget' crates/vb_benchmark/Cargo.toml` returns 1 match.

## Section 4. ATDD Tests

### Happy
- name: `test_baseline_within_budget_returns_true_when_actual_within_budget`
  given: `actual = Duration::from_micros(500)`, `budget_us = 1000`.
  when: `baseline_within_budget(actual, budget_us)` is called.
  then: Returns `true`.
  real_input: `(Duration::from_micros(500), 1000)`.
  expected_output: `true`.
- name: `test_baseline_within_budget_returns_false_when_actual_exceeds_budget`
  given: `actual = Duration::from_micros(1500)`, `budget_us = 1000`.
  when: Called.
  then: Returns `false`.
  real_input: `(Duration::from_micros(1500), 1000)`.
  expected_output: `false`.

### Error
- name: `test_aggregate_resource_budget_with_empty_runs`
  given: `runs = &[]`.
  when: `aggregate_resource_budget(runs)` is called.
  then: Returns `ResourceBudgetReport { total_cpu_us: 0, total_memory_bytes: 0, ... }`.
  real_input: empty slice.
  expected_output: zero-valued report.
- name: `test_aggregate_resource_budget_with_single_run`
  given: `runs = &[RunMetrics { cpu_us: 1000, memory_bytes: 2048, ... }]`.
  when: Called.
  then: Returns a report with `total_cpu_us == 1000` and `total_memory_bytes == 2048`.
  real_input: 1 run.
  expected_output: aggregated report.

### Edge
- name: `test_aggregate_resource_budget_with_100_runs`
  given: 100 runs.
  when: Called.
  then: Returns a report with `total_cpu_us == sum of all 100 runs`.
  real_input: 100 runs.
  expected_output: aggregated report.
- name: `test_aggregate_resource_budget_with_overflow_protection`
  given: Runs with extreme values (e.g., `cpu_us = u64::MAX / 2`).
  when: Called.
  then: Returns a report without overflow (uses `saturating_add`).
  real_input: extreme values.
  expected_output: aggregated report with no overflow.

### Contract
- name: `test_precondition_stub_count_is_zero`
  verifies: Precondition "STUB: count is 0 after cleanup".
  test: `rg 'STUB:' crates/vb_benchmark/tests/ | wc -l` returns 0.
- name: `test_postcondition_aggregate_resource_budget_is_exported`
  verifies: Postcondition "the function is exported from the lib".
  test: `rg 'pub fn aggregate_resource_budget' crates/vb_benchmark/src/lib.rs` returns 1 match.
- name: `test_invariant_bench_is_registered_in_cargo_toml`
  verifies: Invariant "the bench is registered".
  test: `rg 'aggregate_resource_budget' crates/vb_benchmark/Cargo.toml` returns 1 match.

## Section 5. E2E Tests

```
pipeline_test:
  name: test_benchmark_cleanup_e2e
  description: Real benchmark tests, real bench harness; verify no STUB: markers remain and the bench runs.
  setup:
    - confirm benchmark_tests.rs has 0 STUB: markers
    - confirm aggregate_resource_budget.rs exists
    - confirm benches/aggregate_resource_budget.rs exists
  execute:
    - run `cargo test -p vb_benchmark` (all tests should pass)
    - run `cargo bench -p vb_benchmark aggregate_resource_budget -- --nocapture` (bench should run)
  verify:
    - all tests pass
    - bench runs and prints the median
  cleanup:
    - none

e2e_scenarios:
  - name: e2e_no_stub_markers
    description: prove the regression shield works
    steps:
      - run `rg 'STUB:' crates/vb_benchmark/tests/`
      - count the lines
      - assert == 0
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] `benchmark_tests.rs:1-609` read (11 STUB: markers confirmed)"
    - "[x] `src/` directory listing confirmed (only error.rs and lib.rs)"
    - "[x] `benches/` directory does NOT exist"
    - "[x] Round-2 errors documented (wrong STUB: format, fabricated file paths)"
  evidence_required:
    - "Research notes file with line-numbered extracts"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All 8 acceptance tests written (2 happy, 2 error, 2 edge, 3 contract)"
    - "[ ] Tests fail with compile error (function/file does not exist yet)"
  evidence_required:
    - "Test file"
    - "Compile error output"

### Implementation
- name: "Implementation Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] All 8 tests pass"
    - "[ ] moon run :ci passes"
  evidence_required:
    - "Test output"
    - "CI green"

### Integration
- name: "Integration Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] E2E test passes (no STUB: markers, bench runs)"
    - "[ ] No regressions in benchmark tests"
  evidence_required:
    - "E2E output"

## Section 6. Implementation Tasks

### Phase 0: Research
- [ ] Read `benchmark_tests.rs:1-609` (parallel: research)
- [ ] List `crates/vb_benchmark/src/` (parallel: research)
- [ ] Confirm `crates/vb_benchmark/benches/` does NOT exist (parallel: research)
- [ ] Read `Cargo.toml` for `vb_benchmark` (parallel: research)
- [ ] Read `src/lib.rs` (parallel: research)
- [ ] Document the round-2 errors (parallel: research)

### Phase 1: Tests
- [ ] Write `test_baseline_within_budget_returns_true_when_actual_within_budget` (parallel: tests)
- [ ] Write `test_baseline_within_budget_returns_false_when_actual_exceeds_budget` (parallel: tests)
- [ ] Write `test_aggregate_resource_budget_with_empty_runs` (parallel: tests)
- [ ] Write `test_aggregate_resource_budget_with_single_run` (parallel: tests)
- [ ] Write `test_aggregate_resource_budget_with_100_runs` (parallel: tests)
- [ ] Write `test_aggregate_resource_budget_with_overflow_protection` (parallel: tests)
- [ ] Write 3 contract tests (parallel: tests)
- [ ] Write the regression-shield test `test_regression_shield_zero_stub_markers_in_benchmark_tests` (parallel: tests)
- [ ] Confirm all 8 tests fail (gate)

### Phase 2: Implementation
- [ ] Replace the 11 STUB: function bodies in `benchmark_tests.rs` (depends: tests; sequential)
- [ ] Create `crates/vb_benchmark/src/aggregate_resource_budget.rs` (depends: tests; sequential)
- [ ] Export the new function from `src/lib.rs` (depends: file; sequential)
- [ ] Create `crates/vb_benchmark/benches/` directory (depends: file; sequential)
- [ ] Create `crates/vb_benchmark/benches/aggregate_resource_budget.rs` (depends: dir; sequential)
- [ ] Update `Cargo.toml` to register the new bench target (depends: file; sequential)
- [ ] Confirm all 8 tests pass (gate: green)

### Phase 3: Integration
- [ ] Run the E2E test (depends: impl; sequential)
- [ ] Run `cargo test -p vb_benchmark` to confirm no regressions (sequential)
- [ ] Run `cargo bench -p vb_benchmark aggregate_resource_budget -- --nocapture` to confirm the bench works (sequential)

### Phase 4: Documentation
- [ ] Run `moon run :ci` (depends: all of the above; parallel)
- [ ] Close the bead (sequential)

## Section 7. Failure Modes

- symptom: "Test fails: STUB: count is 11, not 0"
  likely_cause: One or more STUB: markers were not replaced.
  where_to_look:
    - file: `crates/vb_benchmark/tests/benchmark_tests.rs`
    - what_to_check: "Are there any `// STUB:` lines remaining?"
  fix_pattern: Find each remaining `// STUB:` line and replace the function body.
- symptom: "Compile error: cannot find module `aggregate_resource_budget`"
  likely_cause: The new file was not created or not exported.
  where_to_look:
    - file: `crates/vb_benchmark/src/lib.rs`
    - what_to_check: "Is `pub mod aggregate_resource_budget;` present?"
  fix_pattern: Add the `pub mod` declaration.
- symptom: "Compile error: cannot find bench target `aggregate_resource_budget`"
  likely_cause: The new bench file is not registered in `Cargo.toml`.
  where_to_look:
    - file: `crates/vb_benchmark/Cargo.toml`
    - what_to_check: "Is `[[bench]] name = \"aggregate_resource_budget\"` present?"
  fix_pattern: Add the bench target.

debugging_commands:
- scenario: "When the STUB: count is wrong"
  run: "rg 'STUB:' crates/vb_benchmark/tests/"
  look_for: "All 11 STUB: lines; cross-check with the cleanup list"
- scenario: "When the bench is not found"
  run: "rg 'aggregate_resource_budget' crates/vb_benchmark/"
  look_for: "The file should be at `src/aggregate_resource_budget.rs` and `benches/aggregate_resource_budget.rs`"

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT cite `crates/vb_benchmark/src/aggregate_resource_budget.rs` as existing — it does NOT. It must be CREATED.
- DO NOT cite `crates/vb_benchmark/benches/` as existing — it does NOT. The directory must be CREATED.
- DO NOT use `// STUB: This test will FAIL` as the pattern — the actual text is `// STUB: <function_name> <description>`.

VERIFY that:
- `crates/vb_benchmark/src/aggregate_resource_budget.rs` does NOT exist before this bead: `rg 'pub fn aggregate_resource_budget' crates/vb_benchmark/src/` (must return ZERO matches before impl).
- `crates/vb_benchmark/benches/` does NOT exist: `ls crates/vb_benchmark/benches/` (must return "No such file or directory").
- The 11 STUB: markers exist: `rg 'STUB:' crates/vb_benchmark/tests/ | wc -l` (must return 11 before impl; 0 after).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    rg 'STUB:' crates/vb_benchmark/tests/ | wc -l  # must be 0
    rg 'aggregate_resource_budget' crates/vb_benchmark/Cargo.toml  # must be 1 match

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-8cdjz/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-8cdjz/progress.txt` and continue from "Current Task". The STUB: format is FIXED at `// STUB: <function_name> <description>`.
Key invariants:
- The 11 STUB: markers in `benchmark_tests.rs` use the format `// STUB: <function_name> <description>`.
- `aggregate_resource_budget.rs` does NOT exist; it must be CREATED at `crates/vb_benchmark/src/`.
- The `benches/` directory does NOT exist; it must be CREATED.
- `Cargo.toml` must be updated to register the new bench target.
- The regression-shield test asserts the STUB: count is 0.

## Section 8. Completion Checklist

- [ ] All 8 acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing (no STUB: markers, bench runs)
- [ ] No mocks or fake data in any test
- [ ] moon run :ci passes
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] No production code touched outside `crates/vb_benchmark/`
- [ ] Regression shield test in place
- [ ] bd close with reason: "S-19r complete: 11 STUB: replaced; aggregate_resource_budget created; bench wired"

## Section 9. Context

Related files:
- `crates/vb_benchmark/tests/benchmark_tests.rs:1-609` — 11 STUB: markers (the cleanup target)
- `crates/vb_benchmark/src/lib.rs` — must export the new function
- `crates/vb_benchmark/src/aggregate_resource_budget.rs` — NEW file
- `crates/vb_benchmark/benches/aggregate_resource_budget.rs` — NEW bench file
- `crates/vb_benchmark/Cargo.toml` — must register the new bench target

Similar implementations:
- (none in current codebase; this is a NEW pattern for the `benches/` directory)

Codebase patterns:
- pattern: "Stub function replacement"
  example_location: `crates/vb_benchmark/tests/benchmark_tests.rs:10, 20, 31, ...`
  how_to_apply: Replace each `// STUB: <name> <description>` with a real function body that performs the documented behavior.

## Section 10. AI Hints

### DO
- Read `crates/vb_benchmark/tests/benchmark_tests.rs:1-609` BEFORE writing any code. The 11 STUB: markers are at known lines.
- Use the EXACT STUB: format: `// STUB: <function_name> <description>`.
- Create `crates/vb_benchmark/src/aggregate_resource_budget.rs` (NEW).
- Create `crates/vb_benchmark/benches/aggregate_resource_budget.rs` (NEW).
- Update `Cargo.toml` to register the new bench target.
- Add a regression-shield test that asserts the STUB: count is 0.
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT use `unwrap()` or `expect()`.
- Do NOT use `// STUB: This test will FAIL` as the pattern.
- Do NOT cite `aggregate_resource_budget.rs` or `benches/` as existing.
- Do NOT use `unsafe`.

### Code patterns
- name: "Real assertion replacing a stub"
  use_when: "Replacing a `// STUB: <name> <description>` with a real function body"
  example: |
    // Before: // STUB: baseline_within_budget always returns false
    pub const fn baseline_within_budget(actual: Duration, budget_us: u64) -> bool {
        actual.as_micros() <= budget_us as u128
    }

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect().
- Test first: Tests MUST exist and FAIL before implementation.
- Read before write: ALWAYS read the source file before modifying it.
- Real data only: Use real `Duration`, `RunMetrics`, etc.; no fabricated placeholders.
- Minimal change: Replace 11 stubs + create 2 new files; do NOT refactor the benchmark crate.
