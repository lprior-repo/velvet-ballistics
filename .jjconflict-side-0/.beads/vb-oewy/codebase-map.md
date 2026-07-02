---
bead_id: vb-oewy
bead_title: "bdd: Full suite runner and evidence artifact contract"
phase: 2
updated_at: 2026-05-20T05:05:00Z
attempt: 1
---

# Codebase Map — vb-oewy

## Bead: Full Suite Runner and Evidence Artifact Contract

vb-oewy delivers a **full BDD suite runner** that discovers, executes, and collects evidence
for all BDD scenarios in the acceptance catalog, plus an **evidence artifact contract** that
defines the structure of the output bundle.

## Existing BDD Infrastructure

### 1. Acceptance Catalog (`crates/workspace_tests/src/acceptance_catalog.rs`)

- `Scenario` struct with fields: id, master_behavior, given, when, then, public_surface,
  fixture, expected_outcome, expected_error, durability_profile, related_bead,
  executable_evidence_target, deferred_follow_up_bead
- `catalog()` → `&'static [Scenario]` — 10 rows (VB-BDD-CATALOG-001 through -010)
- `validate_catalog()` — validates scenario structure
- 4 scenarios deferred to follow-up beads (VB-BDD-CATALOG-005,006,007,008)
- 6 scenarios have executable evidence targets

### 2. Existing BDD Test Files

| File | Scenario Count | Subject |
|------|--------------|---------|
| `crates/workspace_tests/tests/bdd_validation_tests.rs` | 62 | Validation pipeline gates (B1-B62) |
| `crates/vb_cli/tests/cli_vb_m214_bdd_scenarios.rs` | ~40 | CLI operator workflow scenarios |
| `crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs` | 5 | Catalog validation tests |
| `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs` | ~20 | Direct runtime API |
| `crates/workspace_tests/tests/vb_core_yaml_e2e_chain_contract.rs` | ~10 | YAML-to-engine E2E chain |
| `crates/workspace_tests/tests/vb_qi37_2_4_integration_budget_errors.rs` | ~8 | Budget error integration |
| `crates/workspace_tests/tests/vb_37lc_canonical_spelling_red.rs` | ~5 | Canonical spelling |
| `crates/workspace_tests/tests/vb_5xs4_test_loop_inventory_red.rs` | ~5 | Test loop inventory |

### 3. Evidence Artifact Infrastructure (`xtask/src/evidence/`)

- `bundle.rs` — `EvidenceBundle` struct with schema_version, executor_context, linked_bead_id,
  gates (Vec<GateEvidence>), source_test_mappings, release_artifacts
- `profile_runner.rs` — `run_profile()`, `run_gate()` — runs gates and serializes evidence
- `artifact_facts.rs` — artifact fact types
- `release_contract.rs`, `release_model.rs`, `release_validation.rs` — release evidence types

### 4. Runner Gap

There is **NO** full BDD suite runner that:
- Discovers all BDD scenario test files
- Executes them with Given/When/Then structured output
- Aggregates results into an EvidenceBundle
- Produces per-scenario pass/fail evidence with scenario ID tracing

## What vb-oewy Must Build

### Full Suite Runner

A new crate or module (`vb_bdd_runner` or `workspace_tests::bdd_runner`) that:
1. Discovers all BDD test files under `crates/workspace_tests/tests/` and `crates/vb_cli/tests/`
2. Runs each scenario file via `cargo test` with structured output
3. Collects scenario-level pass/fail results with scenario IDs
4. Aggregates into a `BddSuiteResult` struct

### Evidence Artifact Contract

Extends the existing `EvidenceBundle` with BDD-specific fields:
- `bdd_scenarios` — Vec<BddScenarioResult> with id, status, duration_ms, error
- Maps to existing `Scenario` catalog rows
- `executor_context` links to bead vb-oewy

## Touched Crates

- `crates/workspace_tests/` — new `bdd_runner` module
- `xtask/src/evidence/` — extend `bundle.rs` with BDD scenario evidence types

## Public APIs to Create

```rust
// crates/workspace_tests/src/bdd_runner.rs (new)
pub struct BddSuiteResult {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub scenarios: Vec<BddScenarioResult>,
    pub executor_context: ExecutorContext,
    pub linked_bead_id: String,
}

pub struct BddScenarioResult {
    pub scenario_id: String,
    pub test_name: String,
    pub status: BddScenarioStatus, // Passed, Failed, Skipped
    pub duration_ms: u64,
    pub error: Option<String>,
}

pub fn run_bdd_suite() -> Result<BddSuiteResult>
pub fn run_bdd_scenario_file(path: &Path) -> Result<Vec<BddScenarioResult>>
```

## Risk Tags

- `persistence` — evidence bundle must be durable
- `public_api` — runner is a public-facing tool/CLI
- `integration` — cross-crate discovery and execution
- `evidence_contract` — schema must be stable and versioned

## Open Questions

1. Should the runner be a CLI subcommand (`velvet-ballistics bdd-suite`) or a library?
2. Should it integrate with the existing `profile_runner.rs` or be separate?
3. Does evidence go into the same `EvidenceBundle` format or a new BDD-specific bundle?

## Recommended Downstream Owners

- `rust-contract` — for evidence artifact contract schema
- `holzman-rust` — for runner implementation
- `bdd-enforcer` — for scenario assertion strength validation
