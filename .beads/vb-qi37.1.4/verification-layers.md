# Verification Layers — vb-qi37.1.4

## Boundary

- **Verus-owned kernel**: `vb_runtime::recovery::reject_unsupported_live_frame_state`, `vb_runtime::recovery::DurableFrameRecoveryBoundary::hydrate_run_frame`, `vb_storage::recovery::verify_digests`
- **TLA+ temporal model**: Event replay lifecycle, unsupported state propagation, fail-closed gating decision
- **Theorem projection**: None (Verus owns all Rust-local clauses)
- **Runtime shell**: Fjall journal I/O, snapshot decode, event sequence retrieval
- **External systems excluded from formal proof**: Other runtimes, network, wall-clock time

---

## Layer Assignment

| Clause | Primary | Secondary | Tertiary |
|---|---|---|---|
| INV-RC-001 | `verus` | `proptest` | `integration-test` |
| INV-RC-002 | `verus` | `proptest` | `integration-test` |
| INV-RC-003 | `verus` | `proptest` | `integration-test` |
| INV-RC-004 | `verus` | `proptest` | `integration-test` |
| INV-RC-005 | `verus` | `integration-test` | — |
| INV-RC-006 | `verus` | `integration-test` | — |
| INV-RC-007 | `tla-plus` | `integration-test` | — |
| INV-RC-008 | `verus` | `integration-test` | — |
| INV-RC-009 | `verus` | `integration-test` | — |
| PRE-RC-001 | `verus` | `integration-test` | — |
| PRE-RC-002 | `verus` | `integration-test` | — |
| POST-RC-001 | `verus` | `tla-plus` | `integration-test` |
| POST-RC-002 | `verus` | `integration-test` | — |
| POST-RC-003 | `tla-plus` | `integration-test` | — |
| POST-RC-004 | `verus` | `proptest` | `integration-test` |

---

## Verus Scope

### Rust target
`verification/verus/recovery_verification.rs` — standalone Verus model for `reject_unsupported_live_frame_state`, `DurableFrameRecoveryBoundary::hydrate_run_frame`, and `verify_digests` obligations. Production crates remain normal Rust and carry no Cargo dependency on Verus.

### Spec/proof function
```verus
// Verus spec for the fail-closed gate
spec fn spec_reject_unsupported_live_frame_state(seed: &RecoveryFrameSeed) -> bool {
    seed.unsupported.slot_values
        || seed.unsupported.slot_taint
        || seed.unsupported.action_payloads  // MISSING IN SOURCE — adding this closes the gap
        || (!seed.pending_actions.is_empty() && seed.unsupported.pending_actions)
}

// Verus proof: when spec returns true, function returns Err(InvalidRecoveryHydration)
proof fn proof_reject_unsupported_returns_error(seed: &RecoveryFrameSeed)
  requires spec_reject_unsupported_live_frame_state(seed)
  ensures reject_unsupported_live_frame_state(seed) == Err(InvalidRecoveryHydration)
```

### Invariants
- INV-RC-001 through INV-RC-004: each unsupported flag independently causes rejection
- INV-RC-005: action results unreadable when `action_payloads: true`
- INV-RC-003: `action_payloads` flag checked in same conditional as `slot_values` and `slot_taint`

### Trusted boundary
- `RecoveryFrameSeed` constructor is trusted (produced by storage replay)
- `UnsupportedRecoveryState::union()` is trusted helper
- `RunFrame` construction via `RunFrame::new()` is trusted (validated by `vb_core`)

### Shell exclusions
- Fjall journal I/O (storage layer)
- Snapshot decode (storage layer)
- Wall-clock time
- Network

### Evidence command
```bash
verus verification/verus/recovery_verification.rs
```
Expected: Verus verifies all proof obligations with 0 errors.

---

## TLA+ Scope

### Module/model path
`specs/RecoveryReplay.tla` (written in `tla-spec.md`)

### Variables
`seed`, `replay_buf`, `hydration_ok`

### Actions
`SetSlotValuesUnsupported`, `SetActionPayloadsUnsupported`, `RejectUnsupportedState`, `AcceptSupportedState`, `ReplayLifecycleEvent`

### Safety invariants
- `SafeHydration`: hydration succeeds only when all 4 unsupported flags are false (or pending_actions empty)
- `LifecycleEventsNotDropped`: RunResumed/RunRetried/RunAnswered appear in replay buffer

### Temporal properties
- `EventuallyHydratedOrRejected`: recovery always terminates
- `NoSpuriousActionPayloads`: `action_payloads: true` forces hydration failure

### Fairness/deadlock stance
- Weak fairness on unsupported state setters
- No deadlock possible — model is finite state machine with clear terminal states

### Refinement boundary
- `seed.unsupported` ↔ `RecoveryFrameSeed::unsupported` (4 flags)
- `seed.pending_actions` ↔ `RecoveryFrameSeed::pending_actions`
- `RejectUnsupportedState` ↔ `reject_unsupported_live_frame_state()`

### Evidence command
```bash
tlc -config RecoveryReplay.cfg RecoveryReplay.tla
```
Expected: TLC reports no invariant violations for model bounds (16 UnsupportedState combos × 10 pending_actions × 20 replay_buf).

---

## Integration Test Scope

### Test targets
- `crates/vb_storage/tests/recovery_integration.rs`
- `crates/vb_runtime/src/recovery.rs` (in-module tests)
- `crates/workspace_tests/tests/vb_qi37_1_1_red_recovery_contract_test.rs`

### Coverage required
1. `action_payloads: true` + `hydrate_run_frame` → `Err(InvalidRecoveryHydration)`
2. `action_payloads: true` + full recovery → action results unreadable from frame
3. `DigestCheck::Full` + mismatched action ABI → `Err(ActionAbiMismatch)`
4. `DigestCheck::Full` + mismatched policy digest → `Err(PolicyDigestMismatch)`
5. `RunResumed/RunRetried/RunAnswered` present in `replay_events` output
6. All 4 unsupported flags independently trigger rejection
7. Empty `pending_actions` with `pending_actions: true` flag → hydration succeeds

### Evidence command
```bash
cargo test --test recovery_integration -- --nocapture
cargo test -p vb_runtime -- recovery --nocapture
```
Expected: all tests pass with no `InvalidRecoveryHydration` false negatives.

---

## Kani Scope

### Target
`crates/vb_storage/src/kani_codec.rs` — snapshot and event codec roundtrip

### Evidence command
```bash
cargo kani --workspace
```
Expected: Kani reports no failures for codec roundtrip harness.

---

## Waivers

| Clause | Owner | Reason | Compensating Evidence |
|---|---|---|---|
| Fjall journal durability | Storage layer | Out of scope | Kani codec harness + Miri on decode path |
| Snapshot post-card decode | Storage layer | Covered by Kani | `vb_storage/src/kani_codec.rs` |
| Concurrent HashSet in ActionReplayTracker | Concurrent tests | Non-critical for fail-closed | Loom test suite |
| Action retry backoff policy | Out of scope | Not recovery safety | Manual QA |
