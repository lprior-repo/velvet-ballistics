# P2-14c batched-atomicity-bench

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> `crates/vb_runtime/src/shard/impl_parts/dispatch.rs` (lines 1-50), `crates/vb_runtime/src/shard/config.rs` (lines 20-62).

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications (round-2 corrections applied):
- This bead depends ONLY on P2-14b2 (vb-qpcer). It does NOT depend on P2-14a (vb-7e64r) — the storage batch is orthogonal.
- The A/B benchmark measures the COALESCING layer (P2-14b2's `coalesce_window_ticks`), NOT the storage batch (P2-14a's `append_sequenced_batch`).
- The Criterion bench file is NEW: `crates/vb_benchmark/benches/batched_atomicity.rs`. The `benches/` directory does NOT exist; this bead must create it.

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL create a new Criterion bench at `crates/vb_benchmark/benches/batched_atomicity.rs`.
- THE SYSTEM SHALL create the `crates/vb_benchmark/benches/` directory if it does not exist.
- THE SYSTEM SHALL wire the new bench in `crates/vb_benchmark/Cargo.toml`.
- THE SYSTEM SHALL measure the A/B throughput ratio: `coalesce_window_ticks=1` (baseline) vs `coalesce_window_ticks=10` (coalescing).
- THE SYSTEM SHALL close ONLY when the A/B ratio is recorded in `.evidence/batched_atomicity_bench.json` with `ratio >= 3.0`.

### Event-Driven
- WHEN the bench runs run A (`coalesce_window_ticks=1`), THE SYSTEM SHALL record the commit count (expected: ~100 for 100 commands).
- WHEN the bench runs run B (`coalesce_window_ticks=10`), THE SYSTEM SHALL record the commit count (expected: ~10 for 100 commands).
- WHEN the bench completes, THE SYSTEM SHALL compute `ratio = commits_a / commits_b` and assert `ratio >= 3.0`.

### Unwanted
- THE SYSTEM SHALL NOT depend on P2-14a — the benchmark measures coalescing, not storage batching.
- THE SYSTEM SHALL NOT cite `crates/vb_benchmark/benches/` as existing — it does NOT. This bead must create the directory.
- THE SYSTEM SHALL NOT use the round-2 P2-14b's `coalesce_window_us` — the new P2-14b2 uses `coalesce_window_ticks: u32`.

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs:
  - field: `commands`
    type: `Vec<SubmitCommand>`
    constraints: 100 commands per run; each is a `Submit` action.
    example_valid: 100 `Submit { workflow: ... }` commands.
    example_invalid: 0 commands (the bench would be trivial).
  - field: `coalesce_window_ticks`
    type: `u32`
    constraints: 1 (baseline) or 10 (coalescing).
    example_valid: `1`, `10`.
    example_invalid: `0` (degenerate; rejected by the validator).
- system_state:
  - The Criterion bench harness must be available (`criterion` crate in `Cargo.toml`).
  - The `crates/vb_benchmark/benches/` directory must exist (or be created).
  - P2-14b2 (vb-qpcer) must be merged (the bench depends on `coalesce_window_ticks`).

### Postconditions
- state_changes:
  - A new bench file is created at `crates/vb_benchmark/benches/batched_atomicity.rs`.
  - `Cargo.toml` is updated to register the new bench target.
  - The A/B ratio is recorded in `.evidence/batched_atomicity_bench.json`.
- return_guarantees:
  - field: `ratio`
    guarantee: `ratio >= 3.0` (the coalescing layer provides at least 3x throughput).
- side_effects: The bench creates a temp FjallJournal in `/tmp` and cleans it up.

### Invariants
- The A/B ratio is >= 3.0 for the bench to close.
- The bench measures COALESCING (P2-14b2), NOT storage batching (P2-14a).
- The default for `coalesce_window_ticks` remains `1` until the ratio is verified.

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: `crates/vb_runtime/src/shard/impl_parts/dispatch.rs:3-17`
  what_to_extract: The `Shard::tick` function. Confirm it is synchronous, one command per call.
  document_in: research_notes.md
- path: `crates/vb_runtime/src/shard/config.rs:27-38, 52-62`
  what_to_extract: The `ShardConfig` struct and default. Confirm the 5 existing fields and the default values.
  document_in: research_notes.md
- path: `crates/vb_benchmark/Cargo.toml`
  what_to_extract: Existing bench targets and Criterion configuration.
  document_in: research_notes.md
- path: `crates/vb_benchmark/benches/`
  what_to_extract: Confirm the directory does NOT exist.
  document_in: research_notes.md

Patterns to find:
- pattern: `criterion::criterion_group`
  purpose: Locate the Criterion setup pattern.
  expected_locations: `crates/vb_benchmark/Cargo.toml` or existing bench files.
- pattern: `coalesce_window_ticks`
  purpose: Confirm the field is wired in `ShardConfig` (P2-14b2 dependency).
  expected_locations: `crates/vb_runtime/src/shard/config.rs` (after P2-14b2 is merged).

Prior art:
- feature: existing `Shard::tick` (synchronous, one command per call)
  location: `crates/vb_runtime/src/shard/impl_parts/dispatch.rs:3-17`
  what_to_learn: The pattern of a synchronous tick function. The bench calls it directly.

External docs:
- url: Criterion docs (https://github.com/bheisler/criterion.rs)
  section: bench harness
  extract: the `criterion_group!` and `criterion_main!` macros.

Research questions (all answered):
- Q: Does the bench depend on P2-14a? A: No (round-2 had it; black-hat removed it).
- Q: What is the unit? A: `coalesce_window_ticks: u32` (NOT `coalesce_window_us`).
- Q: Where does the bench file live? A: `crates/vb_benchmark/benches/batched_atomicity.rs` (NEW; the directory does not exist).

Research complete when:
- [x] All files_to_read opened.
- [x] All patterns_to_find searched.
- [x] All prior_art examined.
- [x] All research_questions have answers.

## Section 3. Inversions

### Security
- failure: The bench accidentally measures a different code path (e.g., the storage batch) and reports a misleading ratio.
  prevention: The bench explicitly constructs two `Shard` instances with `coalesce_window_ticks=1` and `coalesce_window_ticks=10`. The only difference is the coalescing layer.
  test_for_it: `test_bench_isolates_coalescing: assert run A and run B differ only in coalesce_window_ticks; no other config changes`.

### Usability
- failure: A developer runs the bench and sees "ratio = 1.0" (no improvement), and concludes the coalescing layer is useless.
  prevention: The bench prints a clear message: "ratio X means coalescing_window_ticks=10 is X times faster than baseline". The expected ratio is ~10x (one commit per 10 commands).
  test_for_it: `test_bench_prints_ratio: the bench output includes the ratio in the format "ratio: X.XX"`.

### Data Integrity
- failure: The bench reports a ratio > 10.0 (suspicious; suggests the implementation is doing extra work).
  prevention: The bench caps the ratio at a reasonable bound (e.g., assert `ratio <= 100.0`).
  test_for_it: `test_bench_ratio_bounded: assert 3.0 <= ratio <= 100.0`.

### Integration Failure
- failure: The bench file is created but not registered in `Cargo.toml`, so `cargo bench` does not find it.
  prevention: This bead updates `Cargo.toml` to register the new bench target.
  test_for_it: `test_bench_registered: rg 'batched_atomicity' crates/vb_benchmark/Cargo.toml` returns 1 match.

## Section 4. ATDD Tests

### Happy
- name: `test_bench_a_b_ratio_at_least_3x`
  given: A `Shard` with `coalesce_window_ticks=1` and 100 commands; another `Shard` with `coalesce_window_ticks=10` and 100 commands.
  when: The bench runs both and computes the ratio.
  then: `ratio >= 3.0`.
  real_input: 100 `Submit` commands per run.
  expected_output: `ratio = 10.0` (one commit per 10 commands, vs one per command).
- name: `test_bench_records_evidence_json`
  given: A successful bench run.
  when: The bench completes.
  then: `.evidence/batched_atomicity_bench.json` is created with the ratio.
  real_input: any successful run.
  expected_output: a JSON file with `{"ratio": 10.0, "commits_a": 100, "commits_b": 10, ...}`.

### Error
- name: `test_bench_fails_if_ratio_below_3x`
  given: A `Shard` with `coalesce_window_ticks=1` (no coalescing).
  when: The bench runs.
  then: Returns `Err(BenchError::RatioBelowThreshold)` if `ratio < 3.0`.
  real_input: a configuration that produces ratio < 3.0.
  expected_error: `Err(BenchError)`.
- name: `test_bench_fails_if_both_runs_have_same_commits`
  given: Both runs have `coalesce_window_ticks=1`.
  when: The bench runs.
  then: Returns `Err(BenchError::RatioEqualsOne)` (no coalescing effect).
  real_input: misconfigured bench.
  expected_error: `Err(BenchError)`.

### Edge
- name: `test_bench_with_zero_commands`
  given: 0 commands in the queue.
  when: The bench runs.
  then: Both runs produce 0 commits; ratio is undefined (NaN); bench logs a warning and exits 0.
  real_input: empty queue.
  expected: bench exits 0 with a warning.
- name: `test_bench_with_max_commands_1000`
  given: 1000 commands in the queue.
  when: The bench runs with `coalesce_window_ticks=10`.
  then: Produces ~100 commits; ratio is ~10x.
  real_input: 1000 commands.
  expected: `ratio ≈ 10.0`.

### Contract
- name: `test_precondition_p2_14b2_must_be_merged`
  verifies: Precondition "P2-14b2 is merged".
  test: `rg 'coalesce_window_ticks' crates/vb_runtime/src/shard/config.rs` returns 1 match.
- name: `test_postcondition_evidence_json_exists`
  verifies: Postcondition "evidence JSON is recorded".
  test: assert `.evidence/batched_atomicity_bench.json` exists.
- name: `test_invariant_bench_does_not_depend_on_p2_14a`
  verifies: Invariant "no dependency on P2-14a".
  test: `rg 'append_sequenced_batch' crates/vb_benchmark/benches/batched_atomicity.rs` returns ZERO matches.

## Section 5. E2E Tests

```
pipeline_test:
  name: test_bench_e2e
  description: Real Shard, real FjallJournal, real commands; measure A/B throughput.
  setup:
    - open a real FjallJournal
    - build two Shard instances: coalesce_window_ticks=1 and coalesce_window_ticks=10
    - push 100 Submit commands to each
  execute:
    - call Shard::tick() 100 times on each
    - count journal commits
    - compute ratio
  verify:
    - ratio >= 3.0
    - evidence JSON is written
  cleanup:
    - close FjallJournal
    - delete evidence JSON (or keep for audit)

e2e_scenarios:
  - name: e2e_bench_a_b_throughput_ratio
    description: prove the 3x throughput claim
    steps:
      - run A (coalesce=1)
      - run B (coalesce=10)
      - compute ratio
      - assert ratio >= 3.0
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] `dispatch.rs:3-17` read (synchronous tick confirmed)"
    - "[x] `config.rs:27-38, 52-62` read (5 fields confirmed; new field in P2-14b2)"
    - "[x] `Cargo.toml` for `vb_benchmark` read"
    - "[x] Confirmed: `benches/` directory does NOT exist"
    - "[x] Round-2 errors documented (P2-14a dep, wrong unit)"
  evidence_required:
    - "Research notes file with line-numbered extracts"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All 7 acceptance tests written (2 happy, 2 error, 2 edge, 3 contract)"
    - "[ ] Tests fail with compile error (bench file does not exist yet)"
  evidence_required:
    - "Test file"
    - "Compile error output"

### Implementation
- name: "Implementation Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] All 7 tests pass"
    - "[ ] moon run :ci passes"
  evidence_required:
    - "Test output"
    - "CI green"

### Integration
- name: "Integration Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] E2E test passes with real Shard + real FjallJournal"
    - "[ ] No regressions in benchmark tests"
  evidence_required:
    - "E2E output"
    - "Evidence JSON in `.evidence/`"

## Section 6. Implementation Tasks

### Phase 0: Research
- [ ] Read `dispatch.rs:3-17` (parallel: research)
- [ ] Read `config.rs:27-38, 52-62` (parallel: research)
- [ ] Read `Cargo.toml` for `vb_benchmark` (parallel: research)
- [ ] Confirm `benches/` does NOT exist (parallel: research)
- [ ] Document the round-2 errors (parallel: research)

### Phase 1: Tests
- [ ] Write `test_bench_a_b_ratio_at_least_3x` (parallel: tests)
- [ ] Write `test_bench_records_evidence_json` (parallel: tests)
- [ ] Write `test_bench_fails_if_ratio_below_3x` (parallel: tests)
- [ ] Write `test_bench_fails_if_both_runs_have_same_commits` (parallel: tests)
- [ ] Write `test_bench_with_zero_commands` (parallel: tests)
- [ ] Write `test_bench_with_max_commands_1000` (parallel: tests)
- [ ] Write 3 contract tests (parallel: tests)
- [ ] Confirm all 7 tests fail (gate)

### Phase 2: Implementation
- [ ] Create `crates/vb_benchmark/benches/` directory (depends: tests; sequential)
- [ ] Create `crates/vb_benchmark/benches/batched_atomicity.rs` (depends: dir; sequential)
- [ ] Update `Cargo.toml` to register the new bench target (depends: file; sequential)
- [ ] Implement the A/B benchmark (depends: registration; sequential)
- [ ] Write evidence JSON in `.evidence/batched_atomicity_bench.json` (depends: bench; sequential)
- [ ] Confirm all 7 tests pass (gate: green)

### Phase 3: Integration
- [ ] Run the E2E test (depends: impl; sequential)
- [ ] Confirm the ratio is >= 3.0 (sequential)
- [ ] Run `cargo test -p vb_benchmark` to confirm no regressions (sequential)

### Phase 4: Documentation
- [ ] Run `moon run :ci` (depends: all of the above; parallel)
- [ ] Close the bead (sequential)

## Section 7. Failure Modes

- symptom: "Compile error: cannot find `coalesce_window_ticks` field"
  likely_cause: P2-14b2 is not merged yet.
  where_to_look:
    - file: `crates/vb_runtime/src/shard/config.rs`
    - what_to_check: "Is the `coalesce_window_ticks` field present?"
  fix_pattern: Wait for P2-14b2 to merge, or merge it first.
- symptom: "Test fails: ratio is below 3.0"
  likely_cause: The coalescing layer is not actually reducing the commit count.
  where_to_look:
    - file: `crates/vb_runtime/src/shard/impl_parts/dispatch.rs`
    - function: `Shard::tick`
    - what_to_check: "Is the window counter actually being used to skip dispatches?"
  fix_pattern: Verify the window counter is decremented correctly.
- symptom: "Test fails: bench file is not found by `cargo bench`"
  likely_cause: The new bench target is not registered in `Cargo.toml`.
  where_to_look:
    - file: `crates/vb_benchmark/Cargo.toml`
    - what_to_check: "Is `[[bench]] name = \"batched_atomicity\"` present?"
  fix_pattern: Add the bench target to `Cargo.toml`.

debugging_commands:
- scenario: "When the ratio is wrong"
  run: "RUST_LOG=vb_runtime=trace cargo bench -p vb_benchmark batched_atomicity -- --nocapture"
  look_for: "Trace log showing the commit count for each run"
- scenario: "When the bench file is not found"
  run: "rg 'batched_atomicity' crates/vb_benchmark/"
  look_for: "Confirm the file is at `benches/batched_atomicity.rs` and registered in `Cargo.toml`"

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT depend on P2-14a — the benchmark measures coalescing, not storage batching.
- DO NOT cite `crates/vb_benchmark/benches/` as existing — it does NOT. This bead must create the directory.
- DO NOT use the round-2 P2-14b's `coalesce_window_us` — the new P2-14b2 uses `coalesce_window_ticks: u32`.

VERIFY that:
- P2-14b2 is merged: `rg 'coalesce_window_ticks' crates/vb_runtime/src/shard/config.rs` (must return 1 match).
- The bench file is created: `rg 'fn bench_batched_atomicity' crates/vb_benchmark/benches/batched_atomicity.rs` (must return 1 match).
- `Cargo.toml` is updated: `rg 'batched_atomicity' crates/vb_benchmark/Cargo.toml` (must return 1 match).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    cargo bench -p vb_benchmark batched_atomicity -- --nocapture

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-n7yyz/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-n7yyz/progress.txt` and continue from "Current Task". The dependency is ONLY on P2-14b2; do NOT add P2-14a.
Key invariants:
- Dependency: ONLY on P2-14b2 (vb-qpcer). NOT on P2-14a (vb-7e64r).
- The unit is `coalesce_window_ticks: u32`, NOT `coalesce_window_us`.
- The bench file is NEW at `crates/vb_benchmark/benches/batched_atomicity.rs`.
- The `benches/` directory does NOT exist; this bead creates it.
- The ratio threshold is 3.0x.

## Section 8. Completion Checklist

- [ ] All 7 acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing with real Shard + real FjallJournal
- [ ] No mocks or fake data in any test
- [ ] moon run :ci passes
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] No production code touched outside `crates/vb_benchmark/`
- [ ] Evidence JSON recorded at `.evidence/batched_atomicity_bench.json` with ratio >= 3.0
- [ ] bd close with reason: "P2-14c complete: A/B bench shows >= 3x throughput with coalesce_window_ticks=10"

## Section 9. Context

Related files:
- `crates/vb_runtime/src/shard/impl_parts/dispatch.rs:3-17` — `Shard::tick` (the function being benchmarked)
- `crates/vb_runtime/src/shard/config.rs:27-38, 52-62` — `ShardConfig` (the config being varied)
- `crates/vb_benchmark/Cargo.toml` — bench registration
- `crates/vb_benchmark/benches/batched_atomicity.rs` — NEW bench file
- `.evidence/batched_atomicity_bench.json` — NEW evidence file

Similar implementations:
- Other Criterion benches in the workspace (if any) show the pattern of `criterion_group!` and `criterion_main!`. Apply the same pattern.

Codebase patterns:
- pattern: "Criterion bench with A/B comparison"
  example_location: (none in current codebase; this is a NEW pattern)
  how_to_apply: Use `criterion::Criterion` to define a bench that runs two configurations and compares them.

## Section 10. AI Hints

### DO
- Read `crates/vb_runtime/src/shard/impl_parts/dispatch.rs:3-17` BEFORE writing any code.
- Use `criterion::criterion_group!` and `criterion::criterion_main!` macros.
- Create the `benches/` directory if it does not exist.
- Update `Cargo.toml` to register the new bench target.
- Record the evidence JSON in `.evidence/`.
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT use `unwrap()` or `expect()`.
- Do NOT depend on P2-14a.
- Do NOT use `coalesce_window_us` (the unit is `ticks`).
- Do NOT use `unsafe`.

### Code patterns
- name: "Criterion A/B bench"
  use_when: "Comparing two configurations of the same code"
  example: |
    use criterion::{criterion_group, criterion_main, Criterion};
    fn bench_batched_atomicity(c: &mut Criterion) {
        c.bench_function("coalesce_window_1", |b| b.iter(|| { /* run A */ }));
        c.bench_function("coalesce_window_10", |b| b.iter(|| { /* run B */ }));
    }
    criterion_group!(benches, bench_batched_atomicity);
    criterion_main!(benches);

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect().
- Test first: Tests MUST exist and FAIL before implementation.
- Read before write: ALWAYS read the source file before modifying it.
- Real data only: Use real `Shard` and `FjallJournal`; no fabricated placeholders.
- Minimal change: ONE new bench file; do NOT refactor the benchmark crate.
