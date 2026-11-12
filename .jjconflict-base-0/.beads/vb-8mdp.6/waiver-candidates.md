# Waiver Candidates — Idempotency Hydration

## Bead: vb-8mdp.6

Documents non-behavior-affecting exceptions where a required lane was deemed not applicable or waived.

---

## 1. Flux Refinement Types for Slot Taint (WAIVED)

### Waiver ID: W001

**Proof Seeds**: PS-VB-IDEM-003, PS-VB-IDEM-009

**Contract Clause**: AC3: Forbidden Unchecked Key Derivation / GI5: No Secret in Key

**Requested Waiver**: Flux refinement types enforcing that `validate_idempotency_key_ingredients` only accepts `Taint::Clean` slots for KeyRequired actions.

**Rationale for Waiver**:
1. Existing `verification/flux/vb_rpch_flux_r8.rs` and `verification/flux/vb_rpch_flux_r9.rs` already provide partial Flux coverage for `ActionReplayTracker` surface (PS-VB-IDEM-002).
2. Kani exhaustively tests taint rejection paths: `Err(SecretInKey)`, `Err(RandomInKey)`, `Err(TimeInKey)` for each taint variant.
3. TLA+ `IdempotencySafety.tla` models the `NoSecretInKey` invariant.
4. The slot taint refinement is a separate type-level effort that does not affect the core behavioral verification of the idempotency hydration tests.

**Compensating Evidence**:
- Kani harness `kani_validate_key_ingredients` covers all taint rejection cases
- TLA+ `NoSecretInKey` invariant holds in the model
- Proptest generates random slot taint combinations

**Waiver Expiry**: This waiver is permanent unless a dedicated Flux effort is initiated for slot taint refinement in a future bead.

**Owner**: proof-planner (vb-8mdp.6)

---

## 2. Miri for Wrapping Arithmetic (NOT APPLICABLE)

### Not-Applicable ID: NA001

**Proof Seed**: PS-VB-IDEM-001

**Contract Clause**: GI3: Idempotency Key Determinism

**Reason for Not-Applicable**:
- `compute_action_idempotency_key` uses `u128::wrapping_mul` and `u128::wrapping_add`
- Wrapping arithmetic is **defined behavior** in Rust (not undefined behavior)
- Miri detects raw pointer misuse, use-after-free, double-free, and invalid values
- Miri does NOT detect issues with wrapping multiplication/addition since they are defined

**Evidence**: Rust Reference confirms `wrapping_mul` and `wrapping_add` are deterministic defined operations with no UB.

**Owner**: N/A (not applicable, no waiver needed)

---

## 3. Loom for Concurrency (NOT APPLICABLE)

### Not-Applicable ID: NA002

**Proof Seeds**: PS-VB-IDEM-007, PS-VB-IDEM-004

**Contract Clause**: AC1: Forbidden Non-Idempotent Replay / GI6: Hydration Atomicity

**Reason for Not-Applicable**:
- `ActionReplayTracker` is not shared across threads during recovery
- Recovery processing is single-threaded and sequential
- `apply_tail_events` iterates over events in seq order, no parallelization
- There are no `Mutex`, `RwLock`, `Arc`, or other concurrent data structures in the hydration path

**Evidence**: Source inspection confirms `ActionReplayTracker` contains only `HashMap` and `HashSet` (not `ConcurrentHashMap`). Recovery is invoked from a single task.

**Owner**: N/A (not applicable, no waiver needed)

---

## 4. Cargo-Fuzz for Key Computation (NOT APPLICABLE)

### Not-Applicable ID: NA003

**Proof Seed**: PS-VB-IDEM-001

**Contract Clause**: GI3: Idempotency Key Determinism

**Reason for Not-Applicable**:
- Kani exhaustively checks the bounded input space: `RunId (u64) × SeqNo (u64) × ActionId (u32)`
- The input space is 2^64 × 2^64 × 2^32 — far too large for random fuzzing but perfectly suited for bounded model checking
- Fuzzing with random inputs provides less coverage than exhaustive bounded checking
- Proptest provides statistical coverage with random sampling for regression testing

**Evidence**: Kani's bounded model checking is strictly stronger than fuzzing for this pure function with bounded integer inputs.

**Owner**: N/A (not applicable, no waiver needed)

---

## 5. Summary Table

| Waiver ID | Proof Seeds | Lane | Status | Reason |
|-----------|-------------|------|--------|--------|
| W001 | PS-VB-IDEM-003, PS-VB-IDEM-009 | Flux | WAIVED | Separate effort; Kani+TLA+ provide equivalent coverage |
| NA001 | PS-VB-IDEM-001 | Miri | NOT APPLICABLE | Wrapping arithmetic is defined behavior |
| NA002 | PS-VB-IDEM-007, PS-VB-IDEM-004 | Loom | NOT APPLICABLE | Single-threaded recovery, no concurrent data structures |
| NA003 | PS-VB-IDEM-001 | Cargo-fuzz | NOT APPLICABLE | Kani exhausts bounded input space |

---

## 6. Behavioral Impact Assessment

**None of these waivers affect behavior.** All decisions are based on:
1. Technical equivalence (Miri: wrapping is not UB)
2. Design constraints (Loom: single-threaded by design)
3. Superior alternative coverage (Kani > Fuzz for bounded inputs, Kani+TLA+ > Flux for taint validation)

The proof obligations remain complete with the required lanes (Kani, TLA+, Verus, Proptest, Cargo) providing full coverage of all 20 proof seeds.
