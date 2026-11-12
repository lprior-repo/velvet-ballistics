STATUS: PASS

# State 10 Integration Hygiene Repair Evidence

## Files changed

- `xtask/tests/integration_gates.rs`
  - Renamed repaired-surface integration test functions from banned `test_*` names to behavior names.
  - Replaced `expected_gates` test-body loops with explicit gate assertions in helper functions outside test bodies.
  - Replaced directory-entry test-body loop with a deterministic helper that counts failed YAML evidence files lacking diagnostics.
- `.beads/vb-nf2u/state10-integration-hygiene-repair.md`
  - Added State 10 repair evidence.

## Static hygiene scans

- Command: `rtk grep -rn "fn test_\|fn it_works\|fn should_pass" xtask/tests/integration_gates.rs xtask/tests/ui_release_tooling_red_phase.rs crates/vb_ui_snapshot/tests/redaction_checks.rs crates/vb_ui_makepad/tests crates/vb_ui_snapshot/tests 2>&1`
  - Result: PASS — no output.
- Command: `rtk grep -rn "for .* in \|while " xtask/tests/integration_gates.rs xtask/tests/ui_release_tooling_red_phase.rs crates/vb_ui_snapshot/tests/redaction_checks.rs crates/vb_ui_makepad/tests crates/vb_ui_snapshot/tests 2>&1`
  - Result: PASS — no output.

## Execution gates

- Command: `cargo nextest run -p xtask`
  - Result: PASS — 91 tests run, 91 passed, 0 skipped.
- Command: `rtk cargo fmt --all --check`
  - Result: PASS — no output.

## Tooling red-phase external-machine-gate classification

The State 10 review identified residual risk that formal/tooling red-phase lanes from `.beads/vb-nf2u/test-plan.md` are classified as external machine gates rather than executed by the bead-local `xtask` suite. This repair does not claim bead-local execution for Kani, fuzz, Miri, mutants, coverage, or moon release lanes. The existing `xtask/tests/ui_release_tooling_red_phase.rs` coverage remains metadata/profile classification evidence only; actual execution remains an external machine-gate obligation for the orchestrator.
