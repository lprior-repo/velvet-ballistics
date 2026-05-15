# Verification Layers: vb-core-ipc-sync-evidence

## Boundary
- Verus-owned pure kernel: strict-admission witness, capacity arithmetic, terminal/timer/shutdown transition predicates.
- TLA+ model: bounded safety/enabledness for IPC submit, bounded queues, races, timers, shutdown drain, slow-client behavior, and bounded fanout abstraction.
- Runtime shell: socket I/O, crossbeam queues, channels, journal/storage, scheduler readiness, and client behavior.
- External systems excluded from formal proof: OS socket buffers, filesystem persistence internals, and wall-clock scheduling.

## Layer Assignment
- CON-IPC-001 -> TLA+ safety/enabledness + Verus pure strict-admission + production-refinement blocker + strict-admission tests.
- CON-IPC-002 -> TLA+ queue safety/enabledness + Verus capacity arithmetic + Loom blocker/lane + runtime queue tests.
- CON-IPC-003 -> TLA+ terminal safety + Verus pure terminal predicate + Loom blocker/lane + runtime race tests.
- CON-IPC-004 -> TLA+ timer safety + Verus pure timer predicate + Loom blocker/lane + timer tests.
- CON-IPC-005 -> TLA+ shutdown safety + Verus pure shutdown predicate + Loom blocker/lane + shutdown drain tests.
- CON-IPC-006 -> TLA+ bounded client-buffer safety/enabledness + slow-client executable-test blocker + fuzz/backpressure tests when created.
- CON-IPC-007 -> TLA+ bounded fanout abstraction + exhaustive static scan blocker/lane.
- CON-IPC-008 -> exhaustive static dependency/path scan; Verus and TLA+ are not applicable to source/dependency classification.

## Verus Scope
- Existing commands:
  - `verus verification/verus/ipc_strict_admission.rs`
  - `verus verification/verus/ipc_capacity_bounds.rs`
  - `verus verification/verus/ipc_runtime_transitions.rs`
- Targets: pure proof artifacts above, not production Rust functions directly.
- Required refinement blockers: `REFINE-IPC-001` through `REFINE-IPC-005` must map production constructors/functions/events to the pure witnesses before final closure.
- Trusted boundary: validated capacities/payloads/frames, accepted-artifact store abstraction, and event extraction from production runtime shell.
- Shell exclusions: socket polling, OS readiness, channel internals, crossbeam internals, storage I/O, journal I/O, wall-clock time.

## TLA+ Scope
- Module/model path: `verification/tla/IpcSyncEvidence.tla`.
- Configs: `verification/tla/IpcSyncEvidence.cfg` and `verification/tla/IpcSyncEvidenceCap1.cfg`.
- Variables/actions/invariants: see `tla-spec.md`.
- Evidence commands:
  - `tlc -config verification/tla/IpcSyncEvidence.cfg verification/tla/IpcSyncEvidence.tla`
  - `tlc -config verification/tla/IpcSyncEvidenceCap1.cfg verification/tla/IpcSyncEvidence.tla`
- Fairness/deadlock stance: not currently claimed; tracked by `BLOCK-TLA-LIVENESS`.

## Runtime/Test/Scan Evidence Scope for Later States
- Loom commands are executable but currently blocked by source/model compile errors:
  - `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime bounded_queue`
  - `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime action_completion_cancel`
  - `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime timer_fired_cancel`
  - `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime shutdown_drain`
- Slow-client command remains a blocker until real matching tests exist or an actual target is identified: `cargo test -p vb_ipc slow_client`.
- Static scan commands remain required and need exhaustive classification:
  - `rg -n "tokio::spawn|std::thread::spawn|spawn\(|unbounded|Vec::new|Vec<|channel::unbounded" crates/vb_ipc/src crates/vb_runtime/src crates/vb_core/src crates/vb_storage/src`
  - `rg -n "serde_json|serde_yaml|yaml|json|http|hyper|reqwest|axum|tonic" Cargo.toml crates/vb_ipc crates/vb_runtime crates/vb_core crates/vb_storage`
- Final gate is downstream-owned until code/proof/test repairs complete: `moon ci`.

## Waivers and Blockers
- `THM-WAIVE-001`: Lean/Aeneas/Hax waived; see `lean-contract.md`.
- `WAIVE-VERUS-008`: Verus not applicable to CON-IPC-008 source/dependency classification. Owner: State 3 rust-contract. Compensating evidence: `SCAN-IPC-008` exhaustive static scan.
- `BLOCK-TLA-LIVENESS`: current TLA+ evidence is safety/enabledness only.
- `REFINE-IPC-001..005`: pure Verus proofs require production-refinement map/adapters before final proof closure.
- `BLOCK-LOOM-002..005`, `BLOCK-PROP-006`, `BLOCK-SCAN-007`, `BLOCK-SCAN-008`, and `BLOCK-GATE-009` are explicit blocker obligations in `proof-obligations.jsonl`.
