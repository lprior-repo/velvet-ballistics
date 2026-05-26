# Proof Strategy — Idempotency Hydration for ActionTicket

## Bead: vb-8mdp.6
## Title: Add deterministic idempotency hydration tests for ActionTicket

---

## 1. Scope

This proof plan covers the idempotency hydration subsystem for `ActionTicket`, specifically:
- `compute_action_idempotency_key` (vb_core/src/action.rs:157)
- `ActionReplayTracker` (vb_storage/src/recovery/types.rs:373)
- Hydration path functions (vb_storage/src/recovery/hydrate.rs)
- Slot taint validation for key ingredients

## 2. Proof Seed Summary

| PS-ID | Requirement | Risk Tags |
|-------|-------------|-----------|
| PS-VB-IDEM-001 | GI3: Idempotency Key Determinism | arithmetic-overflow, non-determinism, idempotency |
| PS-VB-IDEM-002 | GI1: ActionTicket Identity | identity-confusion, tracker-semantics, replay-divergence |
| PS-VB-IDEM-003 | AC3: Forbidden Unchecked Key Derivation | secret-leak, taint-propagation, information-disclosure |
| PS-VB-IDEM-004 | GI6: Hydration Atomicity | atomicity, recovery-integrity, partial-state |
| PS-VB-IDEM-005 | GI7: Value Digest Binding | digest-mismatch, divergent-completion, data-integrity |
| PS-VB-IDEM-006 | GI4: Journal Sequence Strictness | sequence-violation, ordering, hydration-integrity |
| PS-VB-IDEM-007 | AC1: Forbidden Non-Idempotent Replay | replay-attack, non-idempotent, side-effect |
| PS-VB-IDEM-008 | GI2: Tracker Key Independence | replay-divergence, data-corruption, tracker-identity |
| PS-VB-IDEM-009 | GI5: No Secret in Key | secret-leak, taint-propagation, information-disclosure |
| PS-VB-IDEM-010 | Post: hydrate_snapshot_tail_preconditions | precondition, run-id-mismatch, sequence-violation |
| PS-VB-IDEM-011 | Post: hydrate_events_preconditions | precondition, empty-events |
| PS-VB-IDEM-012 | Post: action_ticket_has_valid_key | idempotency-key, canonical-key, key-mismatch |
| PS-VB-IDEM-013 | GI4: Journal Event Ordering | sequence-ordering, event-processing, hydration-integrity |
| PS-VB-IDEM-014 | Post: mark_completed_envelope_effect divergence | digest-mismatch, divergent-completion, replay-divergence |
| PS-VB-IDEM-015 | Post: mark_completed_envelope_effect already-resolved | duplicate-completion, already-resolved, non-idempotent |
| PS-VB-IDEM-016 | GI9: Frame Dimension Bounds | dimension-overflow, dimension-underflow, u16-bound |
| PS-VB-IDEM-017 | Post: verify_idempotency MissingKey | missing-key, key-required, unsafe-retry |
| PS-VB-IDEM-018 | Post: ActionReplayTracker is_resolved | already-resolved, non-idempotent, tracker-state |
| PS-VB-IDEM-019 | GI8: Boundary Independence | boundary-violation, circular-dependency, purity |
| PS-VB-IDEM-020 | Post: require_scheduled_ticket | missing-schedule, envelope-without-schedule, replay-divergence |

## 3. Verifier Lane Selection

### 3.1 Kani (Bounded Model Checking) — **REQUIRED**

**Rationale**: The idempotency key computation uses wrapping arithmetic on u128, with bounded integer inputs (RunId u64, SeqNo u64, ActionId u32). Kani can exhaustively check key collision space for small bounds and verify no panic/unwrap in the tracker methods.

**Targets**:
- `compute_action_idempotency_key`: determinism, no overflow panic, collision detection
- `ActionReplayTracker::mark_scheduled_ticket_effect`: divergence detection logic
- `ActionReplayTracker::mark_completed_envelope_effect`: digest comparison, already-resolved paths
- `hydrate_snapshot_tail_preconditions`: boolean combination logic
- `hydrate_dimensions_positive`: const fn bounds

**Existing coverage**: `crates/vb_storage/src/kani_recovery_hydrate.rs` exists with partial coverage.

### 3.2 TLA+ (Temporal Model Checking) — **REQUIRED**

**Rationale**: Existing `IdempotencySafety.tla` and `RecoveryHydration.tla` already model the core invariants. Proof obligations must be traced to these specs.

**Targets**:
- PS-VB-IDEM-001 → IdempotencySafety.tla: key determinism invariant
- PS-VB-IDEM-002 → IdempotencySafety.tla: tracker key independence invariant
- PS-VB-IDEM-005 → IdempotencySafety.tla: digest binding invariant
- PS-VB-IDEM-006 → IdempotencySafety.tla: journal sequence strictness invariant
- PS-VB-IDEM-007 → IdempotencySafety.tla: non-idempotent replay blocking
- PS-VB-IDEM-008 → IdempotencySafety.tla: divergent tickets → ReplayDivergence
- PS-VB-IDEM-014 → IdempotencySafety.tla: envelope divergence detection
- PS-VB-IDEM-016 → RecoveryHydration.tla: dimension bounds

### 3.3 Verus (Refinement Proofs) — **REQUIRED**

**Rationale**: The ActionReplayTracker state machine (new → scheduled → completed/failed) requires refinement proofs to connect abstract spec to concrete Rust. Existing `verification/verus/idempotency_replay_tracker.rs` and `vb_rpch_action_replay_tracker.rs` provide the spine.

**Targets**:
- PS-VB-IDEM-002: Tracker key independence refinement
- PS-VB-IDEM-007: is_resolved monotonicity (once resolved, stays resolved)
- PS-VB-IDEM-018: is_resolved correctness (completed ∨ failed)
- PS-VB-IDEM-015: already-resolved → Duplicate effect

### 3.4 Proptest (Property-Based Testing) — **REQUIRED**

**Rationale**: `compute_action_idempotency_key` determinism is a pure function property: f(x) = f(x) for all x. Proptest generates random (RunId, SeqNo, ActionId) triples and verifies key stability.

**Targets**:
- PS-VB-IDEM-001: deterministic key computation
- PS-VB-IDEM-012: action_ticket_has_valid_key correctness

### 3.5 Miri (Undefined Behavior Detection) — **CONDITIONALLY REQUIRED**

**Rationale**: The wrapping arithmetic in `compute_action_idempotency_key` is intentionally wrapping (not checked). Miri with `MIRIFLAGS=-Zmiri-tag-raw-pointers` detects any raw pointer misuse, but more importantly Miri can detect overflow semantics issues if the code uses `checked_*` or `overflowing_*` by mistake.

**Decision**: `wrapping_mul`/`wrapping_add` are defined behavior in Rust. Miri run is `not_applicable` for this specific function since wrapping is intentional. Miri applies to the broader hydration code paths where `unsafe` or raw pointers might exist.

**Blocked**: No `unsafe` blocks in the idempotency core. Miri not required.

### 3.6 Flux (Refinement Types) — **WAIVED**

**Rationale**: PS-VB-IDEM-003 and PS-VB-IDEM-009 involve slot taint validation. Flux refinement types could enforce at the type level that `validate_idempotency_key_ingredients` only accepts `Taint::Clean` slots for KeyRequired actions.

**Waiver Rationale**: Existing `verification/flux/vb_rpch_flux_r8.rs` and `vb_rpch_flux_r9.rs` cover the ActionReplayTracker surface with Flux, but slot taint refinement for key ingredients is a separate effort. The proof seeds do not require Flux as a mandatory lane; Kani + TLA+ provide sufficient coverage for the taint validation logic.

**Waiver Candidate**: Flux refinement for slot taint in key ingredients (PS-VB-IDEM-003, PS-VB-IDEM-009).

### 3.7 Loom (Concurrency Permutation Testing) — **NOT APPLICABLE**

**Rationale**: The idempotency hydration subsystem is single-threaded. ActionReplayTracker is not shared across threads during recovery; it is owned by a single task processing events sequentially.

**Evidence**: `ActionReplayTracker` contains no `unsafe`, no `Mutex`, no `Arc`. Recovery processing is sequential via `apply_tail_events`.

### 3.8 Cargo-Fuzz (Fuzz Testing) — **NOT APPLICABLE**

**Rationale**: The idempotency key computation is a pure deterministic function with no external input surface beyond the internal type constructors (RunId, SeqNo, ActionId). Fuzzing would require generating arbitrary u64/u32 values which is covered by Kani's exhaustive bounded model checking.

**Exception**: Fuzz could target the journal event decoding path, but that is outside the idempotency core.

## 4. Key Collision Analysis (H1)

The `compute_action_idempotency_key` function maps 160 bits of input (64 + 64 + 32) to 128 bits of output via wrapping multiplication. By the pigeonhole principle, collisions exist.

**Defense-in-Depth**:
1. `ActionReplayTracker` keys on `(action, step)`, NOT `idempotency_key` — collision does not affect replay detection
2. `action_ticket_has_valid_key` verifies canonical key equality before hydration proceeds
3. Tracker evidence comparison includes full `ActionTicket`, not just key

**Kani Obligation**: Exhaustively verify that `mark_scheduled_ticket_effect` returns `Duplicate` when ticket evidence matches, regardless of key collision. The key is NOT used in the tracker lookup — only `(action, step)` is.

## 5. Non-Applicable Lanes Summary

| Lane | Reason |
|------|--------|
| Flux (PS-VB-IDEM-003, PS-VB-IDEM-009) | Waiver candidate — Kani + TLA+ provide sufficient coverage |
| Miri | No `unsafe` in idempotency core; wrapping is intentional defined behavior |
| Loom | Single-threaded sequential processing; no concurrent data structures |
| Cargo-fuzz | Kani exhausts the bounded input space; no external input surface |

## 6. Artifact Inventory

| Artifact | Owner | Status |
|----------|-------|--------|
| `proof-strategy.md` | proof-planner | Written |
| `verifier-lane-decisions.jsonl` | proof-planner | Written |
| `verifier-lane-matrix.md` | proof-planner | Written |
| `proof-coverage-matrix.md` | proof-planner | Written |
| `proof-obligations.planned.jsonl` | proof-planner | Written |
| `trusted-base-plan.md` | proof-planner | Written |
| `proof-plan-review.md` | proof-plan-reviewer | PENDING_APPROVAL |
| `waiver-candidates.md` | proof-planner | Written |
| `waiver-candidates.jsonl` | proof-planner | Written |

## 7. Commands for Evidence

```bash
# Kani bounded model checking
cargo kani --harness kan_recovery_hydrate -p vb_storage

# TLA+ model checking
java -jar tla2tools.jar IdempotencySafety.tla -config IdempotencySafety.cfg
java -jar tla2tools.jar RecoveryHydration.tla -config RecoveryHydration.cfg

# Verus proof execution
cargo verus --verify vb_rpch_action_replay_tracker.rs

# Proptest unit tests
cargo test -p vb_core -- idempotency
cargo test -p vb_storage -- ActionReplayTracker
```
