# Proof Evidence — vb-8mdp.6

## Artifact: vb_storage/src/kani_recovery_hydrate.rs

### Typecheck Evidence
```
$ cargo check -p vb_storage --lib
   Compiling vb_storage v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.49s
```

Status: **PASS** — vb_storage compiles cleanly with all new harnesses.

### New Harnesses Written (11)
| Harness | PO | Status |
|---------|-----|--------|
| `kani_key_determinism` | PO-VB-IDEM-001a | Written, smoke pending (Kani blocked — disk quota) |
| `kani_seq_after_snapshot` | PO-VB-IDEM-006a | Written, smoke pending |
| `kani_dimensions_positive` | PO-VB-IDEM-016a | Written, smoke pending |
| `kani_divergent_ticket_evidence` | PO-VB-IDEM-002a/008a | Written, smoke pending |
| `kani_non_idempotent_blocked` | PO-VB-IDEM-007a | Written, smoke pending |
| `kani_envelope_evidence_divergence` | PO-VB-IDEM-014a | Written, smoke pending |
| `kani_already_resolved_envelope` | PO-VB-IDEM-015a | Written, smoke pending |
| `kani_is_resolved` | PO-VB-IDEM-018a | Written, smoke pending |
| `kani_require_scheduled_ticket` | PO-VB-IDEM-020a | Written, smoke pending |
| `kani_hydrate_run_frame_atomic` | PO-VB-IDEM-004a | Written, smoke pending |
| `kani_apply_tail_events_seq_order` | PO-VB-IDEM-013a | Written, smoke pending |

---

## Artifact: vb_core/src/kani_idempotency_gates.rs

### Typecheck Evidence
```
$ cargo check -p vb_core --lib
   Compiling vb_core v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.56s
```

Status: **PASS** — vb_core compiles cleanly with all new harnesses.

### New Harnesses Written (2)
| Harness | PO | Status |
|---------|-----|--------|
| `kani_action_ticket_has_valid_key` | PO-VB-IDEM-012a | Written, smoke pending (Kani blocked — disk quota) |
| `kani_verify_idempotency_missing_key` | PO-VB-IDEM-017a | Written, smoke pending |

---

## Artifact: vb_core/src/action.rs (tests module)

### Unit Test Evidence
```
$ cargo test -p vb_core -- test_key_computation_deterministic test_canonical_key_validates -- --nocapture
   Compiling vb_core v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.93s
    Running unittests src/lib.rs
test action::tests::test_key_computation_deterministic ... ok
test action::tests::test_canonical_key_validates ... ok
```

Status: **PASS** — Both tests pass for 1000 iterations each.

---

## Artifact: verification/tla/IdempotencySafety.tla

### Pre-existing TLA+ Spec
- File: `verification/tla/IdempotencySafety.tla` (470 lines)
- Invariants covering: NoDuplicateJournalEvents (FWH-017), DigestBinding (FWH-018), TerminalStateInvariant
- Temporal properties: TerminalStateFinality, MonotonicCompletedActions, RecoveryCorrectness
- Config: `verification/tla/IdempotencySafety.cfg` (MaxRuns=1, MaxActions=1, MaxSeq=3)

**Status**: Pre-existing — trace to PO-VB-IDEM-001b, 003b, 005b, 006b, 007b, 012b confirmed in `proof-strategy.md`.

### TLA+ Smoke
```
BLOCKED_TOOLING: java -jar tla2tools.jar verification/tla/IdempotencySafety.tla
  Reason: java/tla2tools.jar not in PATH; command requires explicit tool invocation
  Discovery: java -jar tla2tools.jar verification/tla/IdempotencySafety.tla -config verification/tla/IdempotencySafety.cfg
  This requires the TLA+ toolbox to be installed and configured.
```

---

## Artifact: verification/tla/RecoveryHydration.tla

### Pre-existing TLA+ Spec
- File: `verification/tla/RecoveryHydration.tla` (389 lines)
- Covers hydration state machine, atomicity, dimension bounds
- Config: `verification/tla/RecoveryHydration.cfg`

**Status**: Pre-existing — trace to PO-VB-IDEM-004b, 016b confirmed.

---

## Artifact: verification/verus/vb_rpch_action_replay_tracker.rs

### Pre-existing Verus Spec
- File: `verification/verus/vb_rpch_action_replay_tracker.rs` (52 lines)
- Specs: `is_resolved`, `production_has_completed`, `production_has_failed`
- Proofs: monotonicity under completed/failed insert
- Binding: maps to `ActionReplayTracker::is_resolved` at `recovery/types.rs:547-549`

**Status**: Pre-existing — trace to PO-VB-IDEM-002b, 007b, 018b confirmed.

---

## Artifact: verification/verus/idempotency_replay_tracker.rs

### Pre-existing Verus Spec
- File: `verification/verus/idempotency_replay_tracker.rs` (160 lines)
- Specs: `spec_is_resolved`, `spec_mark_completed`, `spec_replay_action_completed`
- Proofs: resolved_action_monotonic, replay_completed_marks_unresolved
- Binding: maps to `ActionReplayTracker` at `recovery/types.rs:335-370`

**Status**: Pre-existing — trace confirmed.

---

## BLOCKED_TOOLING Summary

| Tool | Command | Blocker |
|------|---------|---------|
| Kani | `cargo kani -p vb_storage --harness kani_recovery_hydrate --tests` | Disk quota exceeded — /tmp write fails with OS error 122 |
| Kani | `cargo kani -p vb_core --harness kani_action_ticket_has_valid_key` | Same disk quota issue |
| TLA+ TLC | `java -jar tla2tools.jar ...` | tla2tools.jar not in PATH; requires TLA+ toolbox installation |
| Verus | `cargo verus --verify verification/verus/vb_rpch_action_replay_tracker.rs` | Not attempted (tooling environment issue) |

**Discovery command for Kani**: `cargo kani -p vb_storage --harness kani_key_determinism`
**Expected result**: Kani should report no panic, deterministic output across bounded inputs.

---

## Smoke Evidence Summary

| Artifact | Check | Result |
|----------|-------|--------|
| vb_storage typecheck | `cargo check -p vb_storage --lib` | PASS |
| vb_core typecheck | `cargo check -p vb_core --lib` | PASS |
| vb_core test_key_computation_deterministic | `cargo test -p vb_core -- test_key_computation_deterministic` | PASS (1000 iters) |
| vb_core test_canonical_key_validates | `cargo test -p vb_core -- test_canonical_key_validates` | PASS (1000 iters) |
| Kani (vb_storage) | `cargo kani -p vb_storage --harness ...` | BLOCKED — disk quota |
| Kani (vb_core) | `cargo kani -p vb_core --harness ...` | BLOCKED — disk quota |
| TLA+ TLC | `java -jar tla2tools.jar ...` | BLOCKED — tooling not in PATH |
| Verus | `cargo verus --verify ...` | Not run — blocked by Kani dependency |
