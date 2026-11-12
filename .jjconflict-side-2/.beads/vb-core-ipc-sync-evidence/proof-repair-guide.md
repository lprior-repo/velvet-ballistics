# Proof Repair Guide: vb-core-ipc-sync-evidence

reviewed_at: 2026-05-15T21:49:00Z
state: 6
attempt: 3-of-7
result: route back to repair

## Repair Targets

- State 5: add reviewed production-refinement maps or executable adapters for `REFINE-IPC-001..005`, or narrow those claims out of the required proof gate with explicit valid waivers.
- State 8: repair loom cfg compilation for `LOOM-IPC-002..005`, then rerun `bounded_queue`, `action_completion_cancel`, `timer_fired_cancel`, and `shutdown_drain` under `RUSTFLAGS="--cfg loom"` with passing raw evidence.
- State 8: add or identify non-vacuous slow-client tests/properties for `PROP-IPC-006`; `0 passed, 407 filtered out` is not evidence.
- State 10: create exhaustive per-match classifications for `SCAN-IPC-007` and `SCAN-IPC-008`; raw grep counts do not discharge source-policy obligations.
- State 5 or State 3: resolve `BLOCK-TLA-LIVENESS` by either adding real TLA+ `PROPERTY`/fairness/deadlock evidence or keeping all claims explicitly bounded safety/enabledness and removing liveness as a required approval blocker.
- State 11: execute or formally classify `GATE-IPC-009` through `moon ci` after proof/test/source repairs.

## Rerun Commands

```bash
pwd -P
jq -c . .beads/vb-core-ipc-sync-evidence/proof-obligations.jsonl >/dev/null
jq -c . .beads/vb-core-ipc-sync-evidence/proof-obligations.planned.jsonl >/dev/null
jq -c . .beads/vb-core-ipc-sync-evidence/traceability-matrix.jsonl >/dev/null
tlc -config verification/tla/IpcSyncEvidence.cfg verification/tla/IpcSyncEvidence.tla
tlc -config verification/tla/IpcSyncEvidenceCap1.cfg verification/tla/IpcSyncEvidence.tla
verus verification/verus/ipc_strict_admission.rs
verus verification/verus/ipc_capacity_bounds.rs
verus verification/verus/ipc_runtime_transitions.rs
RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime bounded_queue
RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime action_completion_cancel
RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime timer_fired_cancel
RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime shutdown_drain
rtk cargo test -p vb_ipc slow_client
```

## Acceptance Bar For Next State 6

- Every required obligation in `proof-obligations.jsonl` is either executable and passed with raw evidence, or carries a valid waiver/blocker that is not being used to approve the same required claim.
- `proof-evidence.md` names exact commands, exits, and key raw output for each required proof lane.
- Pure Verus witnesses are not represented as production proofs unless refinement maps/adapters are reviewed.
- TLA+ liveness/fairness/deadlock claims are either executable or absent from required approval language.
- Static scan rows include complete classifications, not only match counts.
- `proof-findings.jsonl` from the next review remains valid JSONL and non-empty.
