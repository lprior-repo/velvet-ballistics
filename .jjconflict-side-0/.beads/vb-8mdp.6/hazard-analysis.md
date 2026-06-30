# Hazard Analysis — Idempotency Hydration for ActionTicket

## H1: Key Collision Hazard

**Description**: Two distinct `ActionTicket`s with different `(run, seq, action)` tuples produce the same `idempotency_key` (u128 collision).

**Risk Tags**: `arithmetic-overflow`, `hash-collision`, `idempotency`

**Domain Claim**: `compute_action_idempotency_key` uses 128-bit wrapping arithmetic. The function is NOT injective — multiple `(run, seq, action)` inputs can produce the same `u128` output due to the pigeonhole principle (3-tuple of 64+64+32 = 160 bits mapped to 128 bits).

**Mechanism**:
```rust
// Wrapping arithmetic means collision possible when:
// (run_part * K1 + seq_part) * K2 + action_part) * K3 overflows 128 bits
// Different inputs can map to same output after wrapping
```

**Impact**:
- If two distinct tickets share the same key and both are `KeyRequired`, the second completion could be incorrectly treated as a duplicate of the first (or vice versa).
- Silent data corruption: wrong action output associated with wrong ticket.

**Mitigation**:
- `ActionReplayTracker` keys on `(action, step)`, NOT `idempotency_key`. So replay detection is collision-free at the tracker level.
- The `idempotency_key` is used only for caller-provided explicit keys, not for replay detection.
- Canonical keys from `compute_action_idempotency_key` are verified via `action_ticket_has_valid_key` — if key mismatch, ticket is invalid.

**Verification Lanes**: Kani (bounded model checking of key collision space), TLA+ (prove injector of key collision is astronomically unlikely given bounded inputs).

---

## H2: Divergent Tickets Hazard

**Description**: During hydration, `ActionScheduledTicket` events exist for the same `(action, step)` but with different `ActionTicket` evidence (different `seq`, `attempt`, `idempotency_key`, or `capacity`).

**Risk Tags**: `replay-divergence`, `data-corruption`, `recovery-integrity`

**Domain Claim**: `mark_scheduled_ticket_effect` returns `Err(ReplayDivergence)` when existing evidence differs from new evidence.

**Mechanism**:
```
Event1: ActionScheduledTicket { ticket = TicketA }
Event2: ActionScheduledTicket { ticket = TicketB }
  where TicketA.action == TicketB.action
    && TicketA.step == TicketB.step
    && TicketA != TicketB
```
This indicates either:
- Bug in runtime (issued two different tickets for same action/step)
- Data corruption in journal
- Replay of events from different runs mixed up

**Impact**: `RecoveryError::ReplayDivergence` — hydration fails closed.

**Mitigation**:
- Typed error return forces fail-closed behavior.
- Event sourcing with sequence numbers prevents this in normal operation (each action scheduling produces exactly one event).
- Journal append-only semantics make divergent tickets impossible unless corruption occurs.

**Verification Lanes**: Kani (exhaustively test tracker with divergent ticket pairs), TLA+ (invariants on event uniqueness).

---

## H3: Replay Attack Hazard (Non-Idempotent Action Replay)

**Description**: A non-idempotent action (e.g., `SideEffect::Writes` + `RetrySafety::Unsafe`) is replayed during recovery, causing duplicate side effects (e.g., double database writes).

**Risk Tags**: `replay-attack`, `non-idempotent`, `side-effect`, `integrity-violation`

**Domain Claim**: `ActionReplayTracker` blocks replay of non-idempotent actions by marking them completed/failed and rejecting subsequent `ActionScheduledTicket` events for the same `(action, step)`.

**Mechanism**:
```
Original execution:
  1. ActionScheduledTicket { action=Writes, step=S1 }
  2. ActionCompletedEnvelope { action=Writes, step=S1 }
  3. Crash

Recovery:
  1. ActionScheduledTicket { action=Writes, step=S1 } — tracker shows completed
  2. Err(NonIdempotentActionBlocked) — fail closed
```

**Impact**: Recovery fails with `NonIdempotentActionBlocked` instead of silently replaying the write.

**Mitigation**:
- `ActionReplayTracker` marks actions as completed/failed after processing completion envelopes.
- Any subsequent `ActionScheduledTicket` for a resolved action returns `Err(NonIdempotentActionBlocked)`.
- `RetrySafety::Unsafe` actions MUST NOT be retried; they fail closed on recovery.

**Verification Lanes**: Kani (harness with non-idempotent action + completed tracker state + replay event), TLA+ (model non-idempotent replay blocking).

---

## H4: Value Digest Mismatch Hazard

**Description**: During hydration, two `ActionCompletedEnvelope` events exist for the same `(action, step)` with different `value_digest` — indicating the action produced different outputs in the original execution vs. what was recorded.

**Risk Tags**: `digest-mismatch`, `divergent-completion`, `data-integrity`

**Domain Claim**: `mark_completed_envelope_effect` returns `Err(ReplayDivergence)` when existing envelope evidence differs from new evidence.

**Mechanism**:
```
Envelope1: ActionCompletedEnvelope { value_digest = DigestA }
Envelope2: ActionCompletedEnvelope { value_digest = DigestB }
  where DigestA != DigestB
```
This indicates the action was completed twice with different outputs — a fundamental divergence.

**Impact**: `RecoveryError::ReplayDivergence` — fail closed.

**Mitigation**:
- BLAKE3 `value_digest` is computed from the actual output bytes before journaling.
- On replay, we require byte-exact match.
- No mitigation for the root cause (which is already divergence) — we fail closed.

**Verification Lanes**: Kani (envelope with different digests), TLA+ (invariant: all completions for same (action, step) have same value_digest).

---

## H5: Secret Leak via Idempotency Key Hazard

**Description**: An `idempotency_key` is derived from slots containing `Taint::Secret` or `Taint::DerivedFromSecret`, causing the secret to be encoded into the key and potentially leaked through logs, error messages, or the journal itself.

**Risk Tags**: `secret-leak`, `taint-propagation`, `information-disclosure`

**Domain Claim**: `validate_idempotency_key_ingredients` returns `Err(SecretInKey(slot))` when any key slot has `Taint::Secret | Taint::DerivedFromSecret`.

**Mechanism**:
```
Slot[0] contains SecretValue
KeySlots = [Slot[0]]
idempotency_key = derive_from(SecretValue)
Journal entry ActionScheduledTicket { ticket with idempotency_key }
→ Secret exposed in journal
```

**Impact**: Secret data written to journal, potentially exposed in logs or error messages.

**Mitigation**:
- `validate_idempotency_key_ingredients` is called by `verify_idempotency` before any ticket is issued.
- `IdempotencyViolation::SecretInKey` is returned, blocking ticket issuance.
- Admission control rejects artifacts with KeyRequired actions whose key slots contain secrets.

**Verification Lanes**: Flux (refinement type for slot taint), Kani (key ingredient validation), TLA+ (no secret taint in key slots).

---

## H6: Random/Time-Dependent Key Hazard

**Description**: An `idempotency_key` is derived from slots containing `Taint::Random` or `Taint::TimeDependent`, causing the key to be non-deterministic across retries (different each time) or time-dependent (changes over wall-clock time).

**Risk Tags**: `non-determinism`, `time-dependent`, `idempotency-violation`

**Domain Claim**: `validate_idempotency_key_ingredients` returns `Err(RandomInKey(slot))` or `Err(TimeInKey(slot))` when key slots have `Taint::Random | Taint::TimeDependent`.

**Mechanism**:
```
Slot[0] contains RandomValue
idempotency_key = derive_from(RandomValue)  // different every time!
→ First call: key=K1
→ Retry: key=K2 ≠ K1
→ Recovery: cannot match original key
```

**Impact**: KeyRequired action cannot be correctly replayed because key changes per invocation.

**Mitigation**:
- `validate_idempotency_key_ingredients` blocks this at admission time.
- `IdempotencyViolation::RandomInKey` or `TimeInKey` returned.
- Only deterministic, non-time-dependent slot values allowed in key derivation.

**Verification Lanes**: Flux (refinement for taint), Kani (key ingredient validation), TLA+ (key determinism invariant).

---

## H7: Stale Event Hazard (Pre-Snapshot Events)

**Description**: A journal contains events with sequence numbers BEFORE the snapshot's sequence number. These "stale" events are incorrectly processed during hydration, leading to incorrect state.

**Risk Tags**: `sequence-violation`, `ordering`, `hydration-integrity`

**Domain Claim**: `hydrate_snapshot_tail_seq_after_snapshot` returns `false` if any tail event has `seq <= snapshot.seq`.

**Mechanism**:
```
Snapshot at seq=100
Tail events: [seq=50, seq=75, seq=101, seq=102]
                 ^^^^^ stale events
```

**Impact**: `SnapshotRecoveryInputViolation::TailSeqNotAfterSnapshot` — hydration fails with typed error.

**Mitigation**:
- `apply_tail_events` only processes events with `seq > snapshot.seq`.
- Precondition check `hydrate_snapshot_tail_seq_after_snapshot` returns false if stale events exist.
- The stale events are detected and rejected before any state is modified.

**Verification Lanes**: Kani (stale events in tail), TLA+ (sequence ordering invariant).

---

## H8: Overflow/Underflow in Key Computation

**Description**: `compute_action_idempotency_key` uses `wrapping_mul` and `wrapping_add`. If inputs are maliciously large or crafted, overflow could cause key collision (H1) or key wrap to a predictable value.

**Risk Tags**: `arithmetic-overflow`, `idempotency`, `collision`

**Domain Claim**: `compute_action_idempotency_key` is intentionally wrapping. The 128-bit space is large enough that accidental collision is astronomically unlikely, but deliberate adversarial inputs could cause collision.

**Mechanism**:
```rust
u128::MAX.wrapping_mul(K) could wrap to a known value
```

**Impact**: Potential collision if adversarial inputs are provided (but see H1: tracker keys on (action, step), not idempotency_key).

**Mitigation**:
- Inputs are bounded: `RunId`, `SeqNo`, `ActionId` all come from validated internal sources (not external input).
- External attackers cannot influence these values directly.
- Tracker independence from key (H1 mitigation) provides defense in depth.

**Verification Lanes**: Kani (exhaustively test key computation with all valid input ranges), TLA+ (collision probability analysis).

---

## H9: Hydration Frame Dimension Overflow

**Description**: Recovery events imply a `step_count` or `slot_count` that exceeds `u16::MAX`, causing `derive_dimensions_from_snapshot_and_tail` to fail or produce incorrect dimensions.

**Risk Tags**: `dimension-overflow`, `u16-bound`, `recovery-integrity`

**Domain Claim**: `hydrate_dimensions_positive(step_count, slot_count)` returns `false` if either dimension is 0. Dimension overflow is caught by `ensure_nonzero_step_count` and `FrameDimensionOverflow` error.

**Mechanism**:
```
Events imply step_count = 70000 (exceeds u16)
u16::MAX = 65535
→ FrameDimensionOverflow error
```

**Impact**: `RecoveryError::FrameDimensionOverflow` — hydration fails with typed error.

**Mitigation**:
- Typed error return prevents silent overflow.
- `u16` bounds are well-documented; extremely long workflows are not supported.

**Verification Lanes**: Kani (dimension overflow/underflow bounds), TLA+ (finite state space verification).

---

## H10: ActionReplayTracker Identity Confusion

**Description**: `ActionReplayTracker` uses `(ActionId, StepIdx)` as its key, NOT `idempotency_key`. This means two different `ActionTicket`s for the same `(action, step)` with different `idempotency_key` values are treated as the same action (divergence), which is correct. However, callers might mistakenly assume the tracker is keyed by `idempotency_key`.

**Risk Tags**: `identity-confusion`, `api-misuse`, `tracker-semantics`

**Domain Claim**: The tracker key is `(ActionId, StepIdx)` because this is the unique identity of a scheduled action within a run, independent of attempt number or idempotency key.

**Mechanism**:
```
TicketA: (action=A, step=S1, attempt=1, key=K1)
TicketB: (action=A, step=S1, attempt=2, key=K1)
  Both map to same tracker key (A, S1)
  Both are same action at same step, different attempt

TicketC: (action=A, step=S1, attempt=1, key=K2)
  Also maps to same tracker key (A, S1)
  This is REPLAY DIVERGENCE if evidence differs
```

**Impact**: No impact if used correctly. Confusion if caller assumes idempotency_key is the tracker key.

**Mitigation**:
- Documentation clearly states tracker key is `(ActionId, StepIdx)`.
- `mark_scheduled_ticket_effect` compares full `ActionScheduleEvidence` (including ticket) to detect divergence.

**Verification Lanes**: Kani (tracker behavior with same/different keys), code review.