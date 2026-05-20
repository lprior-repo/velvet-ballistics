---
bead_id: vb-oewy
bead_title: "bdd: Full suite runner and evidence artifact contract"
phase: 5
updated_at: 2026-05-20T05:20:00Z
attempt: 1
---

# Proof Evidence — vb-oewy

## Verus Artifacts

### vb_oewy_bdd_runner_invariant.rs

- **File**: `verification/verus/vb_oewy_bdd_runner_invariant.rs`
- **Obligations**: PO-001 (total >= sum), PO-003 (status exhaustive)
- **Specs**:
  - `spec_total_equals_sum` — defines the aggregation invariant
  - `spec_status_discriminant` — maps status enum to integer domain
- **Proofs**:
  - `proof_suite_result_invariant` — proves `total == passed + failed + skipped`
  - `proof_counts_bounded_by_total` — proves individual counts <= total
  - `proof_status_discriminant_exhaustive` — proves all 3 variants covered

## Rust Production Artifacts

### bdd_runner.rs

- **File**: `crates/workspace_tests/src/bdd_runner.rs`
- **Types**: BddRunnerError, BddScenarioStatus, BddScenarioResult, BddSuiteResult, ExecutorContext
- **Functions**: discover_scenario_files, run_bdd_suite, run_bdd_scenario_file, parse_test_output, write_evidence_bundle
- **Unit tests**: 4 inline tests covering basic type properties

## Assumptions and Bounds

- The runner assumes pre-built test binaries
- Test output parsing assumes `cargo test` line format: `test <name> ... <status>`
- No unbounded loops or recursion in the runner core
- All integer arithmetic is bounded by usize (Rust standard)

## Excluded from Proof

- File I/O (discovery, evidence write) — handled by test coverage
- Subprocess execution (cargo test) — handled by test coverage
- YAML serialization — handled by serde roundtrip test
- Timestamp formatting — not safety-critical
