STATUS: PASS

# Regression Diff — tier-a-0-002 State 12 Repair

No State 12 production-code changes were made by the formal verifier. Evidence-only artifacts were regenerated under `.beads/tier-a-0-002/`; reports, alignment, the verification ledger, and trusted-base wording were updated to match the current repaired scanner implementation.

Scoped regression result: PASS.

Global gate note: `timeout 120s moon run :check` is `FAIL_GLOBAL` due unrelated `check-removed-crate-residue` active `vb_codegen` residue at `crates/workspace_tests/tests/vb_y1zq_boundary_inventory_contract/discovery.rs:223`; local `forbid-runtime-fmt` passed before the global failure.
