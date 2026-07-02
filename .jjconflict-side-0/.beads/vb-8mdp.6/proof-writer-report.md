# Proof Writer Report — vb-8mdp.6

## Bead
- ID: vb-8mdp.6
- Title: Add deterministic idempotency hydration tests for ActionTicket

## Summary of Artifacts Written

### 1. Kani Harnesses — vb_storage/src/kani_recovery_hydrate.rs

Appended 9 new proof harnesses targeting idempotency hydration obligations:

| Harness | Obligation | Function Under Test |
|---------|-----------|-------------------|
| `kani_key_determinism` | PO-VB-IDEM-001a | `compute_action_idempotency_key` — deterministic wrapping arithmetic |
| `kani_seq_after_snapshot` | PO-VB-IDEM-006a | `hydrate_snapshot_tail_preconditions` — seq ordering |
| `kani_dimensions_positive` | PO-VB-IDEM-016a | `hydrate_dimensions_positive` — zero-dimension rejection |
| `kani_divergent_ticket_evidence` | PO-VB-IDEM-002a/008a | `mark_scheduled_ticket_effect` — divergence detection |
| `kani_non_idempotent_blocked` | PO-VB-IDEM-007a | `is_resolved` blocks before duplicate detection |
| `kani_envelope_evidence_divergence` | PO-VB-IDEM-014a | `mark_completed_envelope_effect` — digest mismatch |
| `kani_already_resolved_envelope` | PO-VB-IDEM-015a | `mark_completed_envelope_effect` — already-resolved |
| `kani_is_resolved` | PO-VB-IDEM-018a | `ActionReplayTracker::is_resolved` |
| `kani_require_scheduled_ticket` | PO-VB-IDEM-020a | `require_scheduled_ticket` — exact match |
| `kani_hydrate_run_frame_atomic` | PO-VB-IDEM-004a | `hydrate_run_frame` — atomic error paths |
| `kani_apply_tail_events_seq_order` | PO-VB-IDEM-013a | `apply_tail_events` — seq ordering |

### 2. Kani Harnesses — vb_core/src/kani_idempotency_gates.rs

Appended 2 new proof harnesses:

| Harness | Obligation | Function Under Test |
|---------|-----------|-------------------|
| `kani_action_ticket_has_valid_key` | PO-VB-IDEM-012a | `action_ticket_has_valid_key` — canonical key validation |
| `kani_verify_idempotency_missing_key` | PO-VB-IDEM-017a | `verify_idempotency` — MissingKey conditions |

### 3. Proptest/Unit Tests — vb_core/src/action.rs

Added to existing `#[cfg(test)]` module:

| Test | Obligation | Property |
|------|-----------|---------|
| `test_key_computation_deterministic` | PO-VB-IDEM-001c | f(run,seq,action) == f(run,seq,action) for 1000 iterations |
| `test_canonical_key_validates` | PO-VB-IDEM-012b | canonical key validates; wrong key rejects |

Note: vb_core has no proptest dependency. Tests use deterministic LCG PRNG for 1000-iteration coverage.

### 4. Pre-existing Artifacts Used (Not Modified)

- **TLA+**: `verification/tla/IdempotencySafety.tla` + `IdempotencySafety.cfg` — covers key determinism (GI3), tracker independence (GI2), digest binding (GI7), seq strictness (GI4), no-secret-in-key (GI5) invariants
- **TLA+**: `verification/tla/RecoveryHydration.tla` + `RecoveryHydration.cfg` — covers atomicity (GI6) and dimension bounds (GI9)
- **Verus**: `verification/verus/vb_rpch_action_replay_tracker.rs` — is_resolved monotonicity and refinement
- **Verus**: `verification/verus/idempotency_replay_tracker.rs` — replay tracker state machine refinement

## Obligations Touched

All 32 proof obligations from `proof-obligations.planned.jsonl` were reviewed:

| Mode | Count | Obligations |
|------|-------|-------------|
| verify-proof | 27 | PO-VB-IDEM-001a/001b/001c, 002a/002b, 003a/003b, 004a/004b, 005a/005b, 006a/006b, 007a/007b, 008a, 009a, 010a, 011a, 012a/012b, 013a, 014a, 015a, 016a/016b, 017a, 018a/018b, 019a, 020a |
| waived | 1 | PO-VB-IDEM-003c (Flux — Kani+TLA+ sufficient) |
| not-applicable | 4 | Miri (no unsafe), Loom (single-threaded), Cargo-fuzz (Kani exhaustive), Flux VLD-019 |

## Verification Lane Status

| Lane | Artifact | Status |
|------|----------|--------|
| Kani | vb_storage: kani_recovery_hydrate.rs | Written — smoke pending |
| Kani | vb_core: kani_idempotency_gates.rs | Written — smoke pending |
| TLA+ | IdempotencySafety.tla | Pre-existing — trace to obligations confirmed |
| TLA+ | RecoveryHydration.tla | Pre-existing — trace to obligations confirmed |
| Verus | vb_rpch_action_replay_tracker.rs | Pre-existing — trace to obligations confirmed |
| Verus | idempotency_replay_tracker.rs | Pre-existing — trace to obligations confirmed |
| Proptest | vb_core/src/action.rs tests | Written — typecheck pending |

## Trusted Boundaries

- `wrapping_mul`/`wrapping_add` are intentional defined behavior (not overflow UB)
- `HashSet`/`HashMap` operations assumed deterministic for equality
- BLAKE3 digest modeled as `[u8; 32]` equality
- TLA+ constants: `MaxRuns=1, MaxActions=1, MaxSeq=3, Digests={0,1}` — small bounds for exhaustive checking
- Kani bounded unwind values chosen conservatively (3–12)

## Blocked Tooling

- `cargo kani -p vb_core --harness kani_hydrate_events_preconditions` — function lives in vb_storage, not vb_core. The artifact path in PO-VB-IDEM-011a appears to be a typo (`crates/vb_core/src/action.rs` should be `crates/vb_storage/src/recovery/hydrate.rs`). Existing `hydrate_run_frame_from_events_precond_kani` in vb_storage covers this function.

## Pending Deep Executions

- Full Kani run: `cargo kani -p vb_storage --harness kani_recovery_hydrate --tests` (expensive — requires bounded model checking with unwind)
- Full TLA+ TLC: `java -jar tla2tools.jar verification/tla/IdempotencySafety.tla -config verification/tla/IdempotencySafety.cfg`
- Full Verus: `cargo verus --verify verification/verus/vb_rpch_action_replay_tracker.rs`
- Full proptest: `cargo test -p vb_core -- test_key_computation_deterministic -- --nocapture`

These require significant compute time and are marked `PENDING_FORMAL_EXECUTION` after smoke evidence exists.
