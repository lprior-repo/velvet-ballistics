# Baseline Report: vb-iucs

## Commands

| Command | Workdir | Result |
|---------|---------|--------|
| `bd show vb-iucs --json` | `/home/lewis/src/velvet-ballistics` | PASS; issue loaded with title `P0 repair proof integration after verifier rejection` |
| `bd show vb-iucs` | `/home/lewis/src/velvet-ballistics` | PASS; notes identify prior proof repair workspace and evidence |
| `jj workspace add --revision main@origin --sparse-patterns full /home/lewis/src/vb-go-skill/p0-wave-20260515/vb-iucs-recover2` | `/home/lewis/src/velvet-ballistics` | PASS; created full-source isolated workspace |
| `pwd -P` | isolated workspace | PASS; returned isolated path |
| `jj status` | isolated workspace | PASS; initially no changes |
| `bd show vb-iucs --json` | isolated workspace | FAIL as expected; workspace bead DB lacks `issues` table |

## Issue Context From Source Checkout

`vb-iucs` description: Formal-verifier and contract-verification-reviewer rejected current proof/code delta. Repair Rust build integration first, then tighten proof artifacts around production behavior.

Issue notes identify completed scoped proof repair evidence:

- StepState runtime delegates to `vb_proof_kernels` and Kani parity passed.
- Gate 8 Kani harnesses passed.
- Verus `step_state_machine` passed 6 verified, 0 errors.
- BudgetArithmetic TLC passed with 166 states generated and 84 distinct states.
- Dependency `vb-core-proof-15-gate` is closed.
- Follow-up CI blocker `vb-l1yf` is closed and moon ci passed after combined repairs.

## Baseline Decision

Target found. Continue States 2-13 against recovered scoped proof integration. Do not invent a different proof target.
