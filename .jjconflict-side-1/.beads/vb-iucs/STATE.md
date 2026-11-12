# STATE.md: vb-iucs

**bead_id**: vb-iucs
**title**: P0 repair proof integration after verifier rejection
**source_checkout**: `/home/lewis/src/velvet-ballistics`
**isolated_workspace**: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-iucs-recover2`
**current_state**: State 13 APPROVED, landing-ready, stopped before main merge
**updated**: 2026-05-17

## Isolation Proof

- Source checkout is read-only reference only: `/home/lewis/src/velvet-ballistics`.
- Recovery workspace is a separate jj workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-iucs-recover2`.
- `pwd -P` in workspace returned `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-iucs-recover2`.
- `jj workspace list` shows workspace `vb-iucs-recover2` at working copy `mwytwlkm bcf2f8b6` on parent `main` commit `356edd43`.
- `bd show vb-iucs --json` succeeds from source checkout and fails in isolated workspace with missing `issues` table, so issue metadata evidence is sourced from `/home/lewis/src/velvet-ballistics` per recovery instruction.

## Target Recovery

- Recovered target: existing scoped proof repair artifacts under `.beads/vb-qi37.8`.
- Proof integration area: Gate 8 Kani harness integration, StepState runtime-to-proof-kernel parity, StepState Verus mirror, BudgetArithmetic TLC boundary arithmetic.
- Raw artifact search evidence: `rg` over `.beads/**/*.md` found `.beads/vb-qi37.8/proof-evidence.md`, `.beads/vb-qi37.8/formal-verification-report.md`, `.beads/vb-qi37.8/black-hat-review.md`, `.beads/vb-qi37.8/truth-serum-report.md`, and `.beads/vb-qi37.8/final-evidence-decision.md` with the exact Gate 8, StepState, and BudgetArithmetic claims.
- Git/jj history evidence: `jj log -r 'all()'` output stored at `/home/lewis/.local/share/opencode/tool-output/tool_e3551a45e001e3Jgip0j7IWB1F`; history includes plan verifier Gate 8 integration and current main ancestry.

## State Progress

| State | Status | Artifact |
|-------|--------|----------|
| 1 | COMPLETE | `STATE.md`, `baseline-report.md` |
| 2 | COMPLETE | `codebase-map.md`, `delivery-scope.jsonl` |
| 3 | COMPLETE | `contract.md`, `domain-model-review.md`, `tla-spec.md`, `lean-contract.md`, `verification-layers.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl` |
| 4 | COMPLETE | `proof-strategy.md`, `proof-plan-review-input.md`, `proof-obligations.planned.jsonl` |
| 5 | COMPLETE | `proof-writer-report.md`, `proof-evidence.md` |
| 6 | APPROVED | `proof-review.md`, `proof-findings.jsonl`, `contract-verification-review.md` |
| 7 | COMPLETE | `test-plan.md` |
| 8 | COMPLETE | `test-writer-report.md` |
| 9 | APPROVED | `test-plan-review.md`, `test-suite-review.md` |
| 10 | COMPLETE | `implementation.md` |
| 11 | APPROVED | `formal-verification-report.md`, `verification-ledger.jsonl`, `machine-gate-report.md`, `regression-diff.md` |
| 12 | APPROVED | `black-hat-review.md` |
| 13 | APPROVED | `assurance-bundle.md`, `truth-serum-report.md`, `final-evidence-decision.md` |

## Deferred Non-Claims

- `PO-030` full validation pipeline composition remains `DEFERRED_GLOBAL`.
- Gate 8 Verus proof remains `DEFERRED_GLOBAL`.
- Gate 8 Miri remains `DEFERRED_GLOBAL`.
- This recovery does not invent a broader proof target beyond recovered scoped proof integration.
