# Proof Coverage Matrix — Idempotency Hydration

## Bead: vb-8mdp.6

Maps contracts to proof obligations with exact lane coverage.

## Coverage Rows

### C1: GI3 — Idempotency Key Determinism

**Contract Clause**: `compute_action_idempotency_key(run, seq, action)` is deterministic: same inputs always produce same u128 output.

| Obligation ID | Proof Seed | Verifier | Artifact | Command | Evidence |
|---------------|------------|----------|----------|---------|----------|
| PO-VB-IDEM-001a | PS-VB-IDEM-001 | Kani | kani_recovery_hydrate.rs | `cargo kani -p vb_storage --harness kani_key_determinism` | No panic, deterministic output |
| PO-VB-IDEM-001b | PS-VB-IDEM-001 | TLA+ | IdempotencySafety.tla | `java -jar tla2tools.jar IdempotencySafety.tla -config IdempotencySafety.cfg` | KeyDeterminism invariant holds |
| PO-VB-IDEM-001c | PS-VB-IDEM-001 | Proptest | vb_core/action.rs test | `cargo test -p vb_core -- test_key_computation_deterministic` | 1000 iterations pass |

---

### C2: GI1 + GI2 — ActionTicket Identity / Tracker Key Independence

**Contract Clause**: ActionReplayTracker keys on `(ActionId, StepIdx)` only. Two tickets with same `(action, step)` but different evidence produce ReplayDivergence.

| Obligation ID | Proof Seed | Verifier | Artifact | Command | Evidence |
|---------------|------------|----------|----------|---------|----------|
| PO-VB-IDEM-002a | PS-VB-IDEM-002 | Kani | kani_recovery_hydrate.rs | `cargo kani -p vb_storage --harness kani_divergent_ticket_evidence` | ReplayDivergence for divergent tickets |
| PO-VB-IDEM-002b | PS-VB-IDEM-002 | Verus | vb_rpch_action_replay_tracker.rs | `cargo verus --verify vb_rpch_action_replay_tracker.rs` | is_resolved monotonicity proof |

---

### C3: AC3 + GI5 — Forbidden Unchecked Key Derivation / No Secret in Key

**Contract Clause**: `validate_idempotency_key_ingredients` MUST be called before issuing KeyRequired tickets. No secret/random/time-dependent slots in key.

| Obligation ID | Proof Seed | Verifier | Artifact | Command | Evidence |
|---------------|------------|----------|----------|---------|----------|
| PO-VB-IDEM-003a | PS-VB-IDEM-003 | Kani | kani_recovery_hydrate.rs | `cargo kani -p vb_storage --harness kani_validate_key_ingredients` | Correct Err variant per taint |
| PO-VB-IDEM-003b | PS-VB-IDEM-003 | TLA+ | IdempotencySafety.tla | `java -jar tla2tools.jar IdempotencySafety.tla -config IdempotencySafety.cfg` | NoSecretInKey invariant |
| PO-VB-IDEM-003c | PS-VB-IDEM-003 | Flux | **WAIVED** | N/A | Flux waived pending dedicated effort |
| PO-VB-IDEM-009a | PS-VB-IDEM-009 | Kani | kani_recovery_hydrate.rs | `cargo kani -p vb_storage --harness kani_validate_key_ingredients` | Correct error for all taint types |

---

### C4: GI6 — Hydration Atomicity

**Contract Clause**: `hydrate_run_frame` is atomic: either returns `Ok(RunFrame)` with all state correctly reconstructed OR returns typed `RecoveryError` with no partial state.

| Obligation ID | Proof Seed | Verifier | Artifact | Command | Evidence |
|---------------|------------|----------|----------|---------|----------|
| PO-VB-IDEM-004a | PS-VB-IDEM-004 | Kani | kani_recovery_hydrate.rs | `cargo kani -p vb_storage --harness kani_hydrate_run_frame_atomic` | All error paths return before state modification |
| PO-VB-IDEM-004b | PS-VB-IDEM-004 | TLA+ | RecoveryHydration.tla | `java -jar tla2tools.jar RecoveryHydration.tla -config RecoveryHydration.cfg` | Atomicity invariant holds |

---

### C5: GI7 — Value Digest Binding

**Contract Clause**: `ActionCompletedEnvelope.value_digest` is the BLAKE3 digest of value bytes. Any mismatch indicates divergence.

| Obligation ID | Proof Seed | Verifier | Artifact | Command | Evidence |
|---------------|------------|----------|----------|---------|----------|
| PO-VB-IDEM-005a | PS-VB-IDEM-005 | Kani | kani_recovery_hydrate.rs | `cargo kani -p vb_storage --harness kani_envelope_digest_mismatch` | ReplayDivergence on digest mismatch |
| PO-VB-IDEM-005b | PS-VB-IDEM-005 | TLA+ | IdempotencySafety.tla | `java -jar tla2tools.jar IdempotencySafety.tla -config IdempotencySafety.cfg` | DigestInvariant holds |
| PO-VB-IDEM-014a | PS-VB-IDEM-014 | Kani | kani_recovery_hydrate.rs | `cargo kani -p vb_storage --harness kani_envelope_evidence_divergence` | ReplayDivergence on evidence mismatch |

---

### C6: GI4 — Journal Sequence Strictness

**Contract Clause**: All JournalEvent sequence numbers must be strictly monotonically increasing within a run.

| Obligation ID | Proof Seed | Verifier | Artifact | Command | Evidence |
|---------------|------------|----------|----------|---------|----------|
| PO-VB-IDEM-006a | PS-VB-IDEM-006 | Kani | kani_recovery_hydrate.rs | `cargo kani -p vb_storage --harness kani_seq_after_snapshot` | False for stale events |
| PO-VB-IDEM-006b | PS-VB-IDEM-006 | TLA+ | IdempotencySafety.tla | `java -jar tla2tools.jar IdempotencySafety.tla -config IdempotencySafety.cfg` | StrictlyIncreasingSeq invariant |
| PO-VB-IDEM-013a | PS-VB-IDEM-013 | Kani | kani_recovery_hydrate.rs | `cargo kani -p vb_storage --harness kani_apply_tail_events_seq_order` | Events processed in seq order |

---

### C7: AC1 — Forbidden Non-Idempotent Replay

**Contract Clause**: ActionReplayTracker MUST block replay of RetrySafety::Unsafe actions.

| Obligation ID | Proof Seed | Verifier | Artifact | Command | Evidence |
|---------------|------------|----------|----------|---------|----------|
| PO-VB-IDEM-007a | PS-VB-IDEM-007 | Kani | kani_recovery_hydrate.rs | `cargo kani -p vb_storage --harness kani_non_idempotent_blocked` | NonIdempotentActionBlocked returned |
| PO-VB-IDEM-007b | PS-VB-IDEM-007 | Verus | vb_rpch_action_replay_tracker.rs | `cargo verus --verify vb_rpch_action_replay_tracker.rs` | is_resolved monotonicity |
| PO-VB-IDEM-018a | PS-VB-IDEM-018 | Kani | kani_recovery_hydrate.rs | `cargo kani -p vb_storage --harness kani_is_resolved` | Correct completed/failed lookup |

---

### C8: Post: hydrate_snapshot_tail_preconditions

**Contract Clause**: Returns true iff run matches, seq after snapshot, evidence present.

| Obligation ID | Proof Seed | Verifier | Artifact | Command | Evidence |
|---------------|------------|----------|----------|---------|----------|
| PO-VB-IDEM-010a | PS-VB-IDEM-010 | Kani | kani_recovery_hydrate.rs | `cargo kani -p vb_storage --harness kani_snapshot_tail_preconditions` | True only when all 3 checks pass |

---

### C9: Post: hydrate_events_preconditions

**Contract Clause**: Returns true iff !events.is_empty().

| Obligation ID | Proof Seed | Verifier | Artifact | Command | Evidence |
|---------------|------------|----------|----------|---------|----------|
| PO-VB-IDEM-011a | PS-VB-IDEM-011 | Kani | vb_core/action.rs kani | `cargo kani -p vb_core --harness kani_hydrate_events_preconditions` | False for empty, true for non-empty |

---

### C10: Post: action_ticket_has_valid_key

**Contract Clause**: Returns true iff ticket.idempotency_key equals canonical computed key.

| Obligation ID | Proof Seed | Verifier | Artifact | Command | Evidence |
|---------------|------------|----------|----------|---------|----------|
| PO-VB-IDEM-012a | PS-VB-IDEM-012 | Kani | vb_core/action.rs kani | `cargo kani -p vb_core --harness kani_action_ticket_has_valid_key` | True when key matches canonical |
| PO-VB-IDEM-012b | PS-VB-IDEM-012 | Proptest | vb_core/action.rs test | `cargo test -p vb_core -- test_canonical_key_validates` | Canonical key validates, non-canonical rejected |

---

### C11: Post: mark_completed_envelope_effect already-resolved

**Contract Clause**: Returns Ok(Duplicate) when completed/failed contains key, even without envelope entry.

| Obligation ID | Proof Seed | Verifier | Artifact | Command | Evidence |
|---------------|------------|----------|----------|---------|----------|
| PO-VB-IDEM-015a | PS-VB-IDEM-015 | Kani | kani_recovery_hydrate.rs | `cargo kani -p vb_storage --harness kani_already_resolved_envelope` | Duplicate returned for already-resolved |

---

### C12: GI9 — Frame Dimension Bounds

**Contract Clause**: `hydrate_dimensions_positive` returns false if step_count == 0 or slot_count == 0.

| Obligation ID | Proof Seed | Verifier | Artifact | Command | Evidence |
|---------------|------------|----------|----------|---------|----------|
| PO-VB-IDEM-016a | PS-VB-IDEM-016 | Kani | kani_recovery_hydrate.rs | `cargo kani -p vb_storage --harness kani_dimensions_positive` | False for zero dimensions |
| PO-VB-IDEM-016b | PS-VB-IDEM-016 | TLA+ | RecoveryHydration.tla | `java -jar tla2tools.jar RecoveryHydration.tla -config RecoveryHydration.cfg` | Dimension bounds invariant |

---

### C13: Post: verify_idempotency MissingKey

**Contract Clause**: Returns Err(MissingKey) for KeyRequired with empty key_slots or Unsafe retry.

| Obligation ID | Proof Seed | Verifier | Artifact | Command | Evidence |
|---------------|------------|----------|----------|---------|----------|
| PO-VB-IDEM-017a | PS-VB-IDEM-017 | Kani | vb_core/action.rs kani | `cargo kani -p vb_core --harness kani_verify_idempotency_missing_key` | Err(MissingKey) for correct conditions |

---

### C14: GI8 — Boundary Independence

**Contract Clause**: vb_core has no dependencies on vb_storage.

| Obligation ID | Proof Seed | Verifier | Artifact | Command | Evidence |
|---------------|------------|----------|----------|---------|----------|
| PO-VB-IDEM-019a | PS-VB-IDEM-019 | Cargo | vb_core/Cargo.toml | `cargo check -p vb_core 2>&1 \| grep -i vb_storage` | No vb_storage references |

---

### C15: Post: require_scheduled_ticket

**Contract Clause**: Returns Ok only if schedule evidence matches ticket and output exactly.

| Obligation ID | Proof Seed | Verifier | Artifact | Command | Evidence |
|---------------|------------|----------|----------|---------|----------|
| PO-VB-IDEM-020a | PS-VB-IDEM-020 | Kani | kani_recovery_hydrate.rs | `cargo kani -p vb_storage --harness kani_require_scheduled_ticket` | Ok for exact match, ReplayDivergence for mismatch |

---

## Summary

| Contract Group | Obligations | Required Lanes |
|----------------|------------|----------------|
| GI3 Key Determinism | 3 | Kani, TLA+, Proptest |
| GI1/GI2 Tracker Identity | 2 | Kani, Verus |
| AC3/GI5 Key Validation | 4 | Kani, TLA+, Flux (waived) |
| GI6 Hydration Atomicity | 2 | Kani, TLA+ |
| GI7 Digest Binding | 3 | Kani, TLA+ |
| GI4 Sequence Strictness | 3 | Kani, TLA+ |
| AC1 Non-Idempotent Blocking | 3 | Kani, Verus |
| Hydration Preconditions | 2 | Kani |
| Boundary Independence | 1 | Cargo |
| Other Postconditions | 5 | Kani, Proptest |

**Total Obligations**: 28
**Required Lanes**: Kani (18), TLA+ (7), Verus (3), Proptest (2), Cargo (1)
**Waived**: Flux (PS-VB-IDEM-003, PS-VB-IDEM-009)
**Not Applicable**: Miri (3), Loom (2), Cargo-fuzz (1)
