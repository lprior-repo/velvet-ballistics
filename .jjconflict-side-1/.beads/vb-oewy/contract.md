---
bead_id: vb-oewy
bead_title: "bdd: Full suite runner and evidence artifact contract"
phase: 3
updated_at: 2026-05-20T05:10:00Z
attempt: 1
---

# Contract Specification — vb-oewy

## Context

- **Feature**: Full BDD suite runner and evidence artifact contract
- **Domain terms**:
  - `Scenario` — a Given/When/Then structured test row from the acceptance catalog
  - `BddScenarioResult` — pass/fail/skip result for a single scenario
  - `BddSuiteResult` — aggregated results from running all BDD scenarios
  - `EvidenceBundle` — existing top-level evidence container from xtask
- **Assumptions**:
  - Runner operates on an already-built workspace (no compilation step)
  - All scenario test files follow `bdd_` or `scenario_` naming convention
  - Scenario execution is deterministic and isolated per test
- **Open questions**:
  - Should the runner be a library function or a CLI subcommand?
  - Does evidence output to a dedicated `.evidence/bdd/` directory or inline in `EvidenceBundle`?

## Preconditions

- PRE-001: The workspace is in a valid pre-execution state with all test binaries built
- PRE-002: The scenario discovery path points to a directory with at least one scenario file
- PRE-003: The output evidence path is writable

## Postconditions

- POST-001: `run_bdd_suite()` returns a `BddSuiteResult` with `total >= passed + failed + skipped`
- POST-002: Every scenario in the acceptance catalog has a corresponding entry in `BddSuiteResult.scenarios`
- POST-003: `BddScenarioResult.status` is exactly one of `Passed`, `Failed`, `Skipped`
- POST-004: Failed scenarios include an `error` field with the exact panic message or assertion failure
- POST-005: The evidence bundle is written to the specified output path as valid YAML
- POST-006: The runner returns `Err` only for infrastructure failures (not test failures)

## Invariants

- INV-001: Scenario IDs in results match exactly the `Scenario.id` from the acceptance catalog
- INV-002: `duration_ms` is monotonically increasing across scenarios (wall-clock)
- INV-003: No scenario modifies shared global state that affects other scenarios
- INV-004: The evidence bundle schema version is incremented when new fields are added

## Error Taxonomy

- `BddRunnerError::DiscoveryFailed` — no scenario files found in the discovery path
- `BddRunnerError::ExecutionFailed` — cargo test invocation failed (non-zero exit)
- `BddRunnerError::ParseFailed` — test output could not be parsed into scenario results
- `BddRunnerError::EvidenceWriteFailed` — could not write evidence bundle to output path
- `BddRunnerError::NoTestBinary` — the test binary does not exist (must be built first)

## Contract Signatures

```rust
// crates/workspace_tests/src/bdd_runner.rs

/// Runs all BDD scenarios and returns aggregated results.
pub fn run_bdd_suite() -> Result<BddSuiteResult, BddRunnerError>;

/// Runs a single scenario file and returns per-test results.
pub fn run_bdd_scenario_file(
    path: &Path,
) -> Result<Vec<BddScenarioResult>, BddRunnerError>;

/// Writes a BDD suite result to an evidence bundle file.
pub fn write_evidence_bundle(
    result: &BddSuiteResult,
    output_path: &Path,
) -> Result<(), BddRunnerError>;
```

## Verus-Owned Clauses

- INV-001: Scenario ID matching — verifiable via exact equality check on static catalog IDs
- INV-002: `duration_ms` monotonicity — provable via u64::try_from for duration conversion

## TLA+-Owned Clauses

None. This is a deterministic test runner, not a temporal workflow.

## Non-goals

- Compilation step (assumes workspace is pre-built)
- Parallel scenario execution (sequential for determinism)
- Dynamic scenario discovery at runtime (static catalog only)
