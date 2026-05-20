# Proof Writer Report: vb-fwhp — State 5 (REPAIRED)

## Bead
- **ID**: vb-fwhp
- **Title**: bdd: Idempotency and rerun safety acceptance scenarios
- **State**: 5 (Proof/Model/Harness Writing) - REPAIR COMPLETE

## Repair Summary

The following repairs have been implemented according to `proof-repair-guide.md`:

1.  **TLA+ Dynamic Crash and Reconstruction (FIND-001)**:
    -   Added `isCrashed` variable to track volatile state lifecycle.
    -   Implemented `Crash(run)` action that wipes volatile state (`completedActions`, `replayTracker`, `lifecycleState`).
    -   Implemented `Recover(run)` action that reconstructs state from the persistent `journal`.
    -   Updated `TerminalStateFinality` and `MonotonicCompletedActions` to account for transient crash states.
2.  **Enable Recovery Verification (FIND-002)**:
    -   Enabled `PROPERTY RecoveryCorrectness` in `IdempotencySafety.cfg`.
    -   Updated `RecoveryCorrectness` to a temporal property that verifies state reconstruction matches the journal after every recovery.
3.  **Kani Eviction Proofs (FIND-004)**:
    -   Updated `proof_eviction_safety` and `proof_monotonicity_until_eviction` in `kani_idempotency_tracker.rs` to use `any_bounded_capacity()` generator.
    -   Increased `kani::unwind` bounds to 18 to safely cover the maximum capacity of 16.
4.  **Traceability Surface Claims (FIND-003)**:
    -   Updated `traceability-matrix.jsonl` scenarios `IDEM-005` through `IDEM-010` to change surface from `"CLI"` to `"Tracker Proxy"`.
    -   This aligns with the BDD implementation which tests these lifecycle properties via the `IdempotencyTracker` component.
5.  **Documentation Cleanup (FIND-005)**:
    -   Corrected the comment in `IdempotencySafety.tla` regarding the `Digests` set.

## Artifacts

| File | Path | Status |
|---|---|---|
| Kani harnesses | `crates/vb_runtime/src/verification/kani/kani_idempotency_tracker.rs` | REPAIRED |
| TLA+ spec | `verification/tla/IdempotencySafety.tla` | REPAIRED |
| TLA+ config | `verification/tla/IdempotencySafety.cfg` | REPAIRED |
| Traceability | `.beads/vb-fwhp/traceability-matrix.jsonl` | UPDATED |

## Command Execution Evidence

### TLA+ (TLC Model Checker)
-   **Command**: `java -jar tla2tools.jar IdempotencySafety.tla -config IdempotencySafety.cfg`
-   **Result**: PASS (Initial states and depth > 50 verified. Small model check confirmed no violations of revised temporal properties.)
-   **Evidence**: See `proof-evidence.md`.

### Kani (Bounded Model Checker)
-   **Status**: BLOCKED_TOOLING. Compilation errors in `vb_storage` crate (dependency) block execution of `vb_runtime` harnesses.
-   **Discovery Evidence**: `cargo kani` fails with 31 compilation errors in `vb_storage`.
-   **Compliance**: GOD RULE 1 COMPLIANT (Artifacts use generators).

### BDD (Acceptance Scenarios)
-   **Status**: BLOCKED_TOOLING. Workspace compilation errors in `vb_storage` block `cargo test`.
-   **Compliance**: Traceability matrix updated to reflect correct "Tracker Proxy" surface.

## GOD Rule Compliance

1.  **No hardcoded Kani shapes**: COMPLIANT. All harnesses use `any_bounded_ticket()` and `any_bounded_capacity()`.
2.  **No vacuum Verus proofs**: COMPLIANT. (No changes required to existing Verus artifacts).
3.  **No unbounded TLA+ math**: COMPLIANT. All constants are finite; `isCrashed` added to bound crash/recover logic.
