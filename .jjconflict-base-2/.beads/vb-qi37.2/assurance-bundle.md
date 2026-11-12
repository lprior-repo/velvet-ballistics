# Assurance Bundle: vb-qi37.2 State 13

STATUS: APPROVED

State 13 evidence packaging maps all vb-qi37.2 acceptance obligations to raw evidence.

## Evidence Map

- `PO-010`: aggregate add overflow/sum Kani, PASS, `.beads/vb-qi37.2/kani-aggregate-add.raw.log`.
- `PO-011`: aggregate capacity Kani, PASS, `.beads/vb-qi37.2/kani-aggregate-capacity.raw.log`.
- `PO-012`: value-store cap Kani, PASS, `.beads/vb-qi37.2/kani-value-store.raw.log`.
- `PO-013`: focused budget regression tests, PASS, `.beads/vb-qi37.2/cargo-test-budget.raw.log`.
- `PO-014`: `budget_compute` fuzz, PASS, `.beads/vb-qi37.2/fuzz-budget-compute-gnu-final.raw.log`.
- `PO-015`: `aggregate_workflow_budget` fuzz, PASS, `.beads/vb-qi37.2/fuzz-aggregate-workflow-budget-gnu-final.raw.log`.
- `PO-016`: `step_budget_new` fuzz, PASS, `.beads/vb-qi37.2/fuzz-step-budget-new-gnu-final.raw.log`.
- `PO-017`: scoped ValueStore Miri, PASS with reported skips, `.beads/vb-qi37.2/miri-value-store-final.raw.log`.
- `PO-018`: canonical `moon ci`, PASS, `.beads/vb-qi37.2/moon-ci-final.raw.log`.
- `PO-019`: ResourceContract parity tests/source review, PASS, `.beads/vb-qi37.2/resource-contract-vb-core.raw.log` and `.beads/vb-qi37.2/resource-contract-workspace.raw.log`.

## Gate Reviews

- Proof review: `STATUS: APPROVED`, `.beads/vb-qi37.2/proof-review.md`.
- Contract verification review: `STATUS: APPROVED`, `.beads/vb-qi37.2/contract-verification-review.md`.
- Formal verification: `STATUS: APPROVED`, `.beads/vb-qi37.2/formal-verification-report.md`.
- Black-hat review: `STATUS: APPROVED`, `.beads/vb-qi37.2/black-hat-review.md`.

## Waivers

- None. The musl fuzz path failed, but the required fuzz behaviors were executed and passed on the supported GNU sanitizer target.
