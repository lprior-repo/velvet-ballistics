---
bead_id: vb-oewy
bead_title: "bdd: Full suite runner and evidence artifact contract"
phase: 3
updated_at: 2026-05-20T05:10:00Z
attempt: 1
---

# Verification Layers — vb-oewy

## Boundary

- **Verus-owned kernel**: `BddSuiteResult::total >= passed + failed + skipped` invariant,
  `duration_ms` monotonicity, `Scenario.id` matching
- **TLA+ temporal model**: None — not a temporal system
- **Theorem projection**: None — no algebraic kernels
- **Runtime shell**: File I/O (evidence write), subprocess execution (cargo test), YAML serialization
- **External systems excluded**: None

## Layer Assignment

| Contract Clause | Verification Layer | Tool/Command |
|---|---|---|
| PRE-001 | `test` | `cargo test bdd_runner_precondition` |
| PRE-002 | `test` | `cargo test bdd_runner_discovery` |
| PRE-003 | `test` | `cargo test bdd_runner_evidence_write` |
| POST-001 | `verus` | Verus proof of `total >= passed + failed + skipped` |
| POST-002 | `test` | `cargo test bdd_runner_catalog_coverage` |
| POST-003 | `verus` + `test` | Verus enum exhaustive match + variant tests |
| POST-004 | `test` | `cargo test bdd_runner_error_reporting` |
| POST-005 | `test` | `cargo test bdd_runner_evidence_bundle_yaml` |
| POST-006 | `test` | `cargo test bdd_runner_returns_err_infrastructure_only` |
| INV-001 | `test` | `cargo test bdd_runner_scenario_id_matching` |
| INV-002 | `verus` | Verus proof of `duration_ms` monotonicity |
| INV-003 | `test` | `cargo test bdd_runner_no_shared_state` |
| INV-004 | `test` | `cargo test bdd_runner_schema_versioning` |

## Verus Scope

- **Rust target**: `crates/workspace_tests/src/bdd_runner.rs`
- **Spec/proof function**:
  - `spec fn total_ge_sum(result: &BddSuiteResult) -> bool`
  - `proof fn total_ge_sum_proof(result: &BddSuiteResult)`
- **Invariants**: `total >= passed + failed + skipped`
- **Trusted boundary**: `BddSuiteResult` is constructed only via the `add_result` helper
- **Shell exclusions**: File I/O, subprocess execution, YAML serialization are excluded from Verus proof

## TLA+ Scope

None — not applicable.

## Theorem Scope

None — not applicable.

## Waivers

None. All obligations have clear verification paths.
