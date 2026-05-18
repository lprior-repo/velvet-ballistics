# Proof Writer Report: vb-core-ipc-sync-evidence

updated_at: 2026-05-15T22:39:09Z
state: 5
attempt: 3-of-7
skill: proof-writer v1.0.1
workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence`

## Scope

- Worked only in the isolated workspace.
- Repaired verification evidence artifacts only: `.beads/vb-core-ipc-sync-evidence/proof-writer-report.md`, `.beads/vb-core-ipc-sync-evidence/proof-evidence.md`, and `.beads/vb-core-ipc-sync-evidence/STATE.md`.
- Did not edit production source, tests, dependencies, CI, State 3/4 contract artifacts, or `/home/lewis/src/velvet-ballistics`.
- No proof/model source artifact was changed in attempt 3 because the State 6 rejection requires upstream/downstream owners for production refinement, Loom source/test repair, slow-client tests, static-scan classification, canonical obligation shape, and final CI.

## State 6 Rejection Triage

- `REFINE-IPC-001..005`: BLOCK_LOCAL, route to State 3/5/8. No executable production-linked refinement map or adapter exists. Proof-writer cannot fabricate production linkage from pure Verus witnesses and cannot edit production source in State 5.
- `LOOM-IPC-002..005`: BLOCK_LOCAL, route to State 8. Loom build still fails before interleavings execute because existing loom model files miss `Arc` imports.
- `PROP-IPC-006`: BLOCK_LOCAL, route to State 8. `slow_client` filter remains vacuous with zero selected tests.
- `SCAN-IPC-007` and `SCAN-IPC-008`: BLOCK_LOCAL, route to State 10 and State 8 for any defects. Current evidence is match counts only, not exhaustive per-match classification.
- `BLOCK-TLA-LIVENESS`: BLOCK_LOCAL, route to State 3/5. Existing configs intentionally prove bounded safety/enabledness only; adding true `PROPERTY`/fairness/deadlock proof would require a State 3 contract decision or new TLA+ design.
- `GATE-IPC-009`: DEFERRED_GLOBAL, route to State 11 after proof/test/source repairs. No `moon ci` pass is claimed in State 5.
- Canonical `proof-obligations.jsonl` status rule from `contract-verification-review.md`: BLOCK_LOCAL upstream route to State 3/4. Reviewer requires canonical rows to be `planned` only or blockers moved to a separate register; State 5 did not rewrite contract/planning artifacts.

## Verification Artifacts Touched In Attempt 3

- None. Existing TLA+ and Verus artifacts were rerun or attempted as evidence refresh only.

## Command Evidence Summary

- Workspace and JSONL guard: PASS. `TMPDIR=target/tmp test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence"` plus `jq -c .` for `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, and `traceability-matrix.jsonl`; exit 0.
- Canonical status discovery: PASS_AS_DISCOVERY. `proof-obligations.jsonl` has `15 planned`, `14 blocked`, and `1 waived`; this confirms the State 3/4 reviewer-shape blocker.
- TLC capacity 2: PASS after local tmp repair. `TMPDIR=target/tmp JAVA_TOOL_OPTIONS="-Djava.io.tmpdir=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence/target/tmp" tlc -metadir target/tmp/tlc-cap2-2240 -config verification/tla/IpcSyncEvidence.cfg verification/tla/IpcSyncEvidence.tla`; exit 0; `28060 states generated, 5136 distinct states found, 0 states left on queue`.
- TLC capacity 1: PASS after local tmp repair. `TMPDIR=target/tmp JAVA_TOOL_OPTIONS="-Djava.io.tmpdir=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence/target/tmp" tlc -metadir target/tmp/tlc-cap1-2240 -config verification/tla/IpcSyncEvidenceCap1.cfg verification/tla/IpcSyncEvidence.tla`; exit 0; `15781 states generated, 2997 distinct states found, 0 states left on queue`.
- TLC initial rerun note: BLOCK_LOCAL_ENV before local tmp repair. Plain `TMPDIR=target/tmp tlc ...` first failed with `java.io.IOException: Disk quota exceeded` while copying standard modules to `/tmp`; local `JAVA_TOOL_OPTIONS` repaired execution.
- Verus strict admission: PASS_PURE_ONLY. `TMPDIR=target/tmp verus verification/verus/ipc_strict_admission.rs`; exit 0; `verification results:: 5 verified, 0 errors`.
- Verus capacity bounds: PASS_PURE_ONLY. `TMPDIR=target/tmp verus verification/verus/ipc_capacity_bounds.rs`; exit 0; `verification results:: 6 verified, 0 errors`.
- Verus runtime transitions: PASS_PURE_ONLY. `TMPDIR=target/tmp verus verification/verus/ipc_runtime_transitions.rs`; exit 0; `verification results:: 7 verified, 0 errors`.
- Loom bounded queue: BLOCK_LOCAL. `TMPDIR=target/tmp RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime bounded_queue`; exit 101; missing `Arc` in `timer_fired_cancel.rs:18` and `shutdown_drain.rs:16`.
- Loom action completion cancel: BLOCK_LOCAL. `TMPDIR=target/tmp RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime action_completion_cancel`; exit 101; same missing `Arc` compile blocker.
- Loom timer fired cancel: BLOCK_LOCAL. `TMPDIR=target/tmp RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime timer_fired_cancel`; exit 101; same missing `Arc` compile blocker.
- Loom shutdown drain: BLOCK_LOCAL. `TMPDIR=target/tmp RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime shutdown_drain`; exit 101; same missing `Arc` compile blocker.
- Slow client: BLOCK_LOCAL. `TMPDIR=target/tmp rtk cargo test -p vb_ipc slow_client`; exit 0 but `0 passed, 407 filtered out`; no behavior exercised.
- Fanout/buffer scan count: BLOCK_LOCAL until State 10 classification. `TMPDIR=target/tmp rtk rg -n "tokio::spawn|std::thread::spawn|spawn\(|unbounded|Vec::new|Vec<|channel::unbounded" crates/vb_ipc/src crates/vb_runtime/src crates/vb_core/src crates/vb_storage/src | wc -l`; exit 0; `465`.
- YAML/JSON/HTTP scan count: BLOCK_LOCAL until State 10 classification. `TMPDIR=target/tmp rtk rg -n "serde_json|serde_yaml|yaml|json|http|hyper|reqwest|axum|tonic" Cargo.toml crates/vb_ipc crates/vb_runtime crates/vb_core crates/vb_storage | wc -l`; exit 0; `46`.
- Flux discovery: BLOCK_LOCAL_TOOLING for non-required row. `TMPDIR=target/tmp cargo flux --version`; exit 101; `error: no such command: flux`.

## Status By Required Planned Obligation

- `TLA-IPC-001`, `TLA-IPC-001-CAP1`, `TLA-IPC-002`, `TLA-IPC-002-CAP1`, `TLA-IPC-003`, `TLA-IPC-004`, `TLA-IPC-005`, `TLA-IPC-006`, `TLA-IPC-006-CAP1`, `TLA-IPC-007`: PASS_SAFETY_ONLY. TLC passes bounded invariant/enabledness checks only. No temporal liveness/fairness/deadlock pass claimed.
- `VERUS-IPC-001`: PASS_PURE_ONLY. Production linkage remains `REFINE-IPC-001`.
- `VERUS-IPC-002`: PASS_PURE_ONLY. Production linkage remains `REFINE-IPC-002`.
- `VERUS-IPC-003..005`: PASS_PURE_ONLY. Production linkage remains `REFINE-IPC-003..005`.
- `REFINE-IPC-001..005`: BLOCK_LOCAL. Exact upstream route: State 3/5 must define reviewed refinement maps/adapters; State 8 must implement any needed production-linked adapter/source repair.
- `LOOM-IPC-002..005`: BLOCK_LOCAL. Exact upstream route: State 8 source/model repair, then rerun all four loom commands.
- `PROP-IPC-006`: BLOCK_LOCAL. Exact upstream route: State 8 add or identify non-vacuous slow-client tests/properties.
- `SCAN-IPC-007` and `SCAN-IPC-008`: BLOCK_LOCAL. Exact upstream route: State 10 exhaustive per-match classification; State 8 for defects.
- `WAIVE-VERUS-008`: WAIVED by State 3, but contract-verification reviewer rejects this shape if retained in canonical `proof-obligations.jsonl`.
- `BLOCK-TLA-LIVENESS`: BLOCK_LOCAL. Exact upstream route: State 3 narrows temporal claims out of required approval, or State 5 adds real TLA+ `PROPERTY`/fairness/deadlock design and reruns TLC.
- `GATE-IPC-009`: DEFERRED_GLOBAL to State 11. No final CI pass claimed.

## Reviewer Guidance

- State 5 attempt 3 should not be treated as approval. It preserves positive bounded TLA+ and pure Verus evidence while refusing to fake proof success for required blockers.
- The next valid route is State 3/4 for canonical obligation shape and temporal/refinement planning, State 8 for Loom and slow-client repairs, State 10 for exhaustive static-scan classification, then State 11 for `moon ci`.

---

## State 5 Attempt 4 Restructuring (2026-05-16)

### Repair Action

- Acted as proof-writer v1.0.1 in isolated workspace.
- Verified path: `pwd -P` returns isolated workspace.
- Restructured canonical `proof-obligations.jsonl` to satisfy contract-verification-reviewer LETHAL rule.
- Created separate blocker register `proof-obligations.blocked.jsonl` preserving blocker metadata.

### Files Changed

- `proof-obligations.jsonl`: restructured to 15 planned rows only.
- `proof-obligations.blocked.jsonl`: new file with 14 blocked + 1 waived rows.
- `proof-evidence.md`: appended restructuring evidence.
- `proof-writer-report.md`: this append.
- `STATE.md`: appended State 5 attempt 4 transition.

### Scope Limit Honored

- No production source, tests, dependencies, CI, or source checkout edits.
- No verification artifact source code edited (TLA+, Verus).
- Downstream blockers routed to States 3/5, 8, 10, and 11 owners.

### Acceptance Gate

- `proof-obligations.jsonl` contains only `planned` rows: PASS (15 planned).
- `proof-obligations.blocked.jsonl` contains blocked/waived rows with metadata: PASS (14 blocked, 1 waived).
- Both JSONL files parse with `jq -c .`: PASS.
