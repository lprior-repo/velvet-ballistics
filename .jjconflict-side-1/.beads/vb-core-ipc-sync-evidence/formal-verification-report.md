# Formal Verification Report: vb-core-ipc-sync-evidence

STATUS: APPROVED

updated_at: 2026-05-17T03:47:00Z
state: 11

## TLA+

- `tlc -metadir /tmp/opencode/vb-ipc-main-tlc-final -config verification/tla/IpcSyncEvidence.cfg verification/tla/IpcSyncEvidence.tla`; exit 0.
- Result: `28060 states generated, 5136 distinct states found, 0 states left on queue`.
- Temporal checking: `Checking 5 branches of temporal properties`; no error found.
- `tlc -metadir /tmp/opencode/vb-ipc-cap1-tlc-final -config verification/tla/IpcSyncEvidenceCap1.cfg verification/tla/IpcSyncEvidence.tla`; exit 0.
- Result: `15781 states generated, 2997 distinct states found, 0 states left on queue`.
- Temporal checking: `Checking 5 branches of temporal properties`; no error found.

## Verus

- `verus verification/verus/ipc_strict_admission.rs`; exit 0; `verification results:: 5 verified, 0 errors`.
- `verus verification/verus/ipc_capacity_bounds.rs`; exit 0; `verification results:: 6 verified, 0 errors`.
- `verus verification/verus/ipc_runtime_transitions.rs`; exit 0; `verification results:: 7 verified, 0 errors`.

## Rust/Loom

- `rtk cargo test -p vb_runtime ipc_refinement`; exit 0; `5 passed, 1460 filtered out`.
- `RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime bounded_queue`; exit 0; `2 passed, 1472 filtered out`.
- `RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime action_completion_cancel`; exit 0; `2 passed, 1472 filtered out`.
- `RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime timer_fired_cancel`; exit 0; `1 passed, 1473 filtered out`.
- `RUSTFLAGS="--cfg loom" rtk cargo test -p vb_runtime shutdown_drain`; exit 0; `3 passed, 1471 filtered out`.

## Decision

STATUS: APPROVED
