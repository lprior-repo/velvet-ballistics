# Machine Gate Report: vb-iucs

STATUS: APPROVED

## Gates

| Gate | Result | Evidence |
|------|--------|----------|
| Source checkout issue lookup | PASS | `bd show vb-iucs --json` from `/home/lewis/src/velvet-ballistics` |
| Isolated jj workspace | PASS | `jj workspace add --revision main@origin --sparse-patterns full ...` |
| Workspace path guard | PASS | `pwd -P` returned isolated path |
| Artifact search | PASS | `.beads/vb-qi37.8` evidence found for Gate 8, StepState, and BudgetArithmetic |
| Source binding inspection | PASS | `frame.rs`, `kani_step_state_transition.rs`, `kani_gate_08_accessor.rs`, Verus and TLA files read |
| JSONL validation | PASS | `jq -c . .beads/vb-iucs/delivery-scope.jsonl .beads/vb-iucs/proof-obligations.jsonl .beads/vb-iucs/traceability-matrix.jsonl .beads/vb-iucs/proof-obligations.planned.jsonl .beads/vb-iucs/proof-findings.jsonl .beads/vb-iucs/verification-ledger.jsonl >/dev/null` |
| Cargo metadata | PASS | `cargo metadata --no-deps >/dev/null` |
| Moon CI | NOT_RERUN | Issue notes report `vb-l1yf` resolved and moon ci passed after combined repairs; no new production code changed here |

## Classification

No local blocker found for scoped State 13 approval. Production verification reruns are inherited from recovered raw evidence and current issue notes; this recovery adds evidence artifacts only.
