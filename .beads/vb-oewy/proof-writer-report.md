---
bead_id: vb-oewy
bead_title: "bdd: Full suite runner and evidence artifact contract"
phase: 5
updated_at: 2026-05-20T05:20:00Z
attempt: 1
---

# Proof Writer Report — vb-oewy

## Changed Verification Artifacts

| Artifact | Change |
|---|---|
| `crates/workspace_tests/src/bdd_runner.rs` | New module — BDD runner types, error enum, run_bdd_suite, run_bdd_scenario_file, write_evidence_bundle |
| `verification/verus/vb_oewy_bdd_runner_invariant.rs` | New Verus proof file — structural invariants for BddSuiteResult and BddScenarioStatus |

## Proof Obligations Covered

| Obligation | Status | Evidence |
|---|---|---|
| PO-001 (total >= sum invariant) | Planned | Verus specs in vb_oewy_bdd_runner_invariant.rs |
| PO-003 (status exhaustive) | Planned | Verus specs + proof in vb_oewy_bdd_runner_invariant.rs |
| PO-008 (duration monotonic) | Waived | Not applicable — LOW risk |

## Blocked Tooling

None. All required tools are available.

## Assumptions

- `cargo test` output format is parseable via the `test <name> ... <status>` line format
- The workspace has already been built (no compilation step in runner)
- `serde_yaml` is available in workspace_tests (needs dependency check)

## Open Questions

1. Does `serde_yaml` need to be added to workspace_tests Cargo.toml?
2. Should `cargo nextest` be supported instead of/in addition to `cargo test`?
3. Should the runner be exposed as a CLI subcommand?

## Commands Attempted

```bash
# Verus check (when run)
verus crates/workspace_tests/src/bdd_runner.rs
verus verification/verus/vb_oewy_bdd_runner_invariant.rs

# Test compile check
cargo check -p workspace_tests
```
