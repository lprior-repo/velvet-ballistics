# Theorem Kernel Projection — vb-qi37.1.4

## Boundary

- **TLA+-owned temporal model**: Recovery event replay lifecycle (RunResumed/RunRetried/RunAnswered), unsupported state flag propagation, fail-closed gating decisions. Covered by TLA+ model `RecoveryReplay.tla`.
- **Verus-owned Rust core**: `DurableFrameRecoveryBoundary::hydrate_run_frame`, `reject_unsupported_live_frame_state`, `UnsupportedRecoveryState::union`, `verify_digests`. This is the primary proof surface.
- **Theorem-owned kernel**: None. All critical clauses are expressible in Verus.
- **Rust/runtime shell**: Fjall journal I/O, snapshot decode, event sequence retrieval. Excluded from formal proof; covered by integration tests and Miri.
- **External systems excluded**: Network, wall-clock time, other runtimes.

## Verus-Owned Clauses (Rust Core Proof Obligations)

All critical fail-closed invariants are proven in Verus. No separate Lean theorem projection is required.

### INV-RC-003 (action_payloads fail-closed gate)

**Contract clause**: INV-RC-003  
**Rust target**: `vb_runtime::recovery::reject_unsupported_live_frame_state`  
**Verus spec**: `spec_reject_unsupported_live_frame_state`  
**Invariant shape**: When `seed.unsupported.action_payloads` is `true`, the function returns `Err(RuntimeError::InvalidRecoveryHydration)`.

```verus
// Pseudocode — actual spec in crates/vb_runtime/src/recovery_verus.rs
spec fn reject_unsupported_live_frame_state(seed: &RecoveryFrameSeed) -> bool {
    seed.unsupported.slot_values
        || seed.unsupported.slot_taint
        || seed.unsupported.action_payloads  // <-- THIS IS THE GAP
        || (!seed.pending_actions.is_empty() && seed.unsupported.pending_actions)
}
```

### INV-RC-005 (action_payloads not consumed)

**Contract clause**: INV-RC-005  
**Rust target**: `RuntimeRecoveryBoundary::hydrate_run_frame` (trait spec)  
**Invariant**: When `UnsupportedRecoveryState::action_payloads` is `true`, no action result may be read from the recovered `RunFrame`.

### INV-RC-008 (action ABI digest check)

**Contract clause**: INV-RC-008  
**Rust target**: `vb_storage::recovery::verify_digests`  
**Invariant**: When `level == DigestCheck::Full`, the function returns `Ok(())` iff all action ABI digests match stored records.

## Theorem Obligations

### None — Verus owns all Rust-local proof obligations

**Rationale**: The fail-closed recovery invariants are pure state predicates on `RecoveryFrameSeed`, `UnsupportedRecoveryState`, and `DigestCheck`. These are directly expressible in Verus:

- `UnsupportedRecoveryState` is a `bool`-flag struct — Verus handles this natively.
- `reject_unsupported_live_frame_state` is a pure boolean function — trivially Verus-expressible.
- `verify_digests` is a deterministic function over journal contents — expressible as a Verus spec with iteration over action records.
- The `action_payloads` gap (INV-RC-003) is a missing boolean branch in an existing pure function — Verus will prove the correct branch is needed once the contract is written.

**No Lean/Aeneas/Hax required**: The algebraic structure of `UnsupportedRecoveryState` is a 4-field boolean record with `OR` composition — no complex state transitions, lattices, or protocol refinement that would benefit from a proof assistant.

## Waivers

| Clause | Owner | Reason | Limitation | Compensating Evidence |
|---|---|---|---|---|
| Fjall journal durability | Storage layer | Out of scope for runtime boundary | N/A | Kani codec harness |
| Snapshot post-card decode | Storage layer | Covered by Kani harness | N/A | `vb_storage/src/kani_codec.rs` |
| Concurrent `ActionReplayTracker` HashSet | Concurrent test | Non-critical for fail-closed proof | Loom test | Integration tests |
| Action retry backoff | Out of scope | Not recovery safety | N/A | N/A |
