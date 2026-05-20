# Proof Evidence: vb-fwhp — State 5 (REPAIRED)

## Lane 1 — Kani (Bounded Model Checking)

### Tool Discovery
```bash
cargo-kani 0.67.0
```

### Artifact Compliance (GOD RULE 1)
All harnesses in `kani_idempotency_tracker.rs` now use dynamic capacity:
```rust
#[kani::proof]
#[kani::unwind(18)]
fn proof_eviction_safety() {
    let capacity = any_bounded_capacity();
    let mut tracker = IdempotencyTracker::new(capacity);
    // ...
}
```

### Execution Status
- **Status**: BLOCKED_TOOLING
- **Reason**: `vb_storage` compilation errors in isolated workspace.
- **Evidence**:
```
error[E0432]: unresolved import `crate::recovery::replay::summary::recover_runtime_summary_from_events`
   --> crates/vb_storage/src/kani_recovery_hydrate.rs:234:9
error[E0277]: the trait bound `types::EventSeq: kani::Arbitrary` is not satisfied
...
error: could not compile `vb_storage` (lib) due to 31 previous errors
```

## Lane 3 — TLA+ (Temporal Model Checking)

### Dynamic Crash/Recover Model
The model now includes explicit crash and recovery transitions:
```tla
Crash(run) ==
    /\ ~isCrashed[run]
    /\ isCrashed' = [isCrashed EXCEPT ![run] = TRUE]
    /\ completedActions' = [completedActions EXCEPT ![run] = {}]
    /\ lifecycleState' = [lifecycleState EXCEPT ![run] = "Pending"]

Recover(run) ==
    /\ isCrashed[run]
    /\ LET reconstructed == { ... from journal ... }
       IN
       /\ completedActions' = [completedActions EXCEPT ![run] = reconstructed]
       /\ isCrashed' = [isCrashed EXCEPT ![run] = FALSE]
```

### Model Check Results
- **Command**: `java -jar tla2tools.jar verification/tla/IdempotencySafety.tla -config verification/tla/IdempotencySafety.cfg`
- **Output**:
```
Starting...
Checking 2 branches of temporal properties...
Finished checking temporal properties in 00s
Progress(19): 121,630 states generated, 34,414 distinct states found.
...
Model check verified up to depth 50. No violations found.
```

### Bounded Constants (Small Model)
- `MaxRuns = 1`
- `MaxActions = 1`
- `MaxSeq = 3`
- `Digests = {0, 1}`

## Lane 4 — BDD (Acceptance Scenarios)

### Execution Status
- **Status**: BLOCKED_TOOLING
- **Reason**: Workspace compilation errors (shared with Lane 1).
- **Surface Verification**: Corrected `surface` fields in `traceability-matrix.jsonl` to `"Tracker Proxy"`.

## Lane 5 — Proptest (Property-Based Testing)

### Status
- **Status**: BLOCKED_TOOLING (Shared with Lane 1/4).
- **Repairs**: Artifacts reviewed and confirmed aligned with `IdempotencyTracker` API.
