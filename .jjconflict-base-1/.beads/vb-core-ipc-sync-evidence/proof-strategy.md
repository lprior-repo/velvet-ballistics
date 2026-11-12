# Proof Strategy: vb-core-ipc-sync-evidence

updated_at: 2026-05-15T20:48:34Z
state: 4
attempt: 3-of-7
skill: proof-planner v1.0.1
workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence`

## Scope

- Planning only. No production source, tests, proof/model/harness/spec, dependency, or config files were edited.
- Replanned from repaired State 3 artifacts after State 6 rejection.
- Source checkout `/home/lewis/src/velvet-ballistics` was not written.

## Inputs Read

- `.beads/vb-core-ipc-sync-evidence/contract.md`
- `.beads/vb-core-ipc-sync-evidence/verification-layers.md`
- `.beads/vb-core-ipc-sync-evidence/proof-obligations.jsonl`
- `.beads/vb-core-ipc-sync-evidence/traceability-matrix.jsonl`
- `.beads/vb-core-ipc-sync-evidence/delivery-scope.jsonl`
- `.beads/vb-core-ipc-sync-evidence/proof-review.md`
- `.beads/vb-core-ipc-sync-evidence/proof-findings.jsonl`
- `.beads/vb-core-ipc-sync-evidence/proof-repair-guide.md`
- `.beads/vb-core-ipc-sync-evidence/contract-verification-review.md`
- Prior evidence files `.beads/vb-core-ipc-sync-evidence/proof-evidence.md` and `.beads/vb-core-ipc-sync-evidence/proof-writer-report.md` as context only.

## Discovery Commands

- `pwd -P` -> `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence`.
- `test -s ".beads/vb-core-ipc-sync-evidence/contract.md"` -> exit 0.
- `test -s ".beads/vb-core-ipc-sync-evidence/traceability-matrix.jsonl"` -> exit 0.
- `test -s ".beads/vb-core-ipc-sync-evidence/delivery-scope.jsonl"` -> exit 0.
- `rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" crates/vb_ipc/src crates/vb_runtime/src crates/vb_core/src crates/vb_storage/src` -> exit 0; `8366 matches in 240 files`.
- `rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" crates/vb_ipc/src crates/vb_runtime/src crates/vb_core/src crates/vb_storage/src verification` -> exit 0; `988 matches in 188 files`.

## Blocked Discovery

- None.

## Verifier Selection

- TLA+ is required for CON-IPC-001 through CON-IPC-007 bounded protocol safety/enabledness. Existing commands are executable, but true temporal liveness/fairness remains a planned blocker until the TLA+ model/configs add `PROPERTY`, fairness, and a deadlock stance.
- Verus is required for CON-IPC-001 through CON-IPC-005 pure Rust-local predicates. Existing commands are executable, but production-refinement rows remain required because prior evidence proved pure models only.
- Loom is required for CON-IPC-002 through CON-IPC-005 concurrency/interleaving risk. Commands are exact, but current evidence says they compile-fail until source/model repair.
- Slow-client boundedness for CON-IPC-006 needs an executable test/property lane. The current exact command is retained as a blocker because it selected zero tests.
- Static scans are required for CON-IPC-007 and CON-IPC-008, with exhaustive per-match classification required before closure.
- `moon ci` is downstream final-gate evidence and remains planned for State 11.
- Kani, Miri, Flux, and theorem-kernel lanes are explicitly non-required or blocked as rows in `proof-obligations.planned.jsonl`; no pass result is claimed.

## Output

- Machine-readable planned obligations: `.beads/vb-core-ipc-sync-evidence/proof-obligations.planned.jsonl`.
- Review handoff: `.beads/vb-core-ipc-sync-evidence/proof-plan-review-input.md`.
