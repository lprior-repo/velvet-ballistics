---
bead_id: vb-oewy
bead_title: "bdd: Full suite runner and evidence artifact contract"
phase: 7
updated_at: 2026-05-20T05:30:00Z
attempt: 1
---

# Proof-to-Rust Map — vb-oewy

## Overview

This bridge maps Verus proof obligations to Rust source targets and test evidence paths.

## PO-001: BddSuiteResult Aggregation Invariant

### Proof Variable → Rust Target Mapping

| Proof Concept | Rust Target | Verification Lane |
|---|---|---|
| `spec_total_equals_sum` | `crates/workspace_tests/src/bdd_runner.rs::BddSuiteResult` | Verus |
| `total == passed + failed + skipped` | Struct field initialization in `run_bdd_suite()` | Verus + test |
| `passed <= total` | Derived from sum | Verus lemma |

### Rust Evidence

- **Source**: `crates/workspace_tests/src/bdd_runner.rs` lines 60-70
- **Verus module**: `verification/verus/vb_oewy_bdd_runner_invariant.rs`
- **Test**: `bdd_runner_tests.rs::test_suite_result_total_invariant`
- **Harness**: N/A (Verus pure proof, no harness needed)

### Implementation Boundary

The invariant holds at construction time. The fields are set simultaneously in `run_bdd_suite()`. No post-construction mutation of `total`/`passed`/`failed`/`skipped` fields after construction.

## PO-003: BddScenarioStatus Exhaustiveness

### Proof Variable → Rust Target Mapping

| Proof Concept | Rust Target | Verification Lane |
|---|---|---|
| `spec_status_discriminant` | `crates/workspace_tests/src/bdd_runner.rs::BddScenarioStatus` | Verus |
| 3-variant exhaustive match | Match expression in `parse_test_line()` | Verus + test |
| Error field correlation | `error.is_some() == (status == Failed)` | Test |

### Rust Evidence

- **Source**: `crates/workspace_tests/src/bdd_runner.rs::BddScenarioStatus` (enum definition)
- **Verus module**: `verification/verus/vb_oewy_bdd_runner_invariant.rs`
- **Test**: `bdd_runner_tests.rs::test_status_exhaustive_match`
- **Harness**: N/A (Verus pure proof)

## PO-002: Catalog Coverage

### Requirement → Test Mapping

| Requirement | Test Evidence | Harness |
|---|---|---|
| Every catalog scenario has a result | `bdd_runner_tests.rs::test_all_catalog_scenarios_have_results` | Test iterates catalog and checks results |

## PO-004: Error Field for Failed Scenarios

### Requirement → Test Mapping

| Requirement | Test Evidence | Harness |
|---|---|---|
| Failed scenarios carry error | `bdd_runner_tests.rs::test_failed_scenario_carry_error` | Synthetic failing test checks error field |

## Intentionally Outside Rust Boundary

- `cargo test` subprocess output format — tested via integration test
- File system discovery — tested via unit test with temp dir
- YAML serialization roundtrip — tested via serde roundtrip test

## Waivers

- **INV-002 (duration monotonicity)**: WAIVED as LOW risk. Sequential execution makes this trivially safe. Test evidence: sequential execution in `run_bdd_suite()` is single-threaded.
