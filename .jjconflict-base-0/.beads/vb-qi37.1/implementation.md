# Implementation Report: vb-qi37.1

STATUS: APPROVED

## Implementation Surface

- Runtime hydration boundary: `crates/vb_runtime/src/recovery.rs`.
- Storage recovery orchestration: `crates/vb_storage/src/recovery/recover.rs`.
- Frame-seed replay and recovery state: `crates/vb_storage/src/recovery/replay/summary.rs` and `crates/vb_storage/src/recovery/types.rs`.
- Formal artifacts: `verification/verus/recovery_verification.rs`, `verification/tla/RecoveryHydration.tla`, `verification/tla/RecoveryHydration.cfg`.

## Continuation Delta

- This continuation did not change production source. It consumed the current isolated-workspace implementation and repaired formal evidence.
- State 5 attempt 4 changed only `verification/verus/recovery_verification.rs` and evidence artifacts, adding direct `PO-003A` / `VERUS-PRE-004` proof coverage.

## Behavior Implemented

- Digest verification checks workflow-source for `WorkflowSourceOnly`, `WorkflowAndIr`, and `Full`, and compiled-IR for `WorkflowAndIr` and `Full`.
- Frame-seed recovery reconstructs summary, dimensions, pc, step states, slot values/taint, and unsupported-state flags.
- Runtime boundary applies recovered steps, slots, taint, and pc; unsupported or inconsistent seeds fail closed with typed runtime errors.
