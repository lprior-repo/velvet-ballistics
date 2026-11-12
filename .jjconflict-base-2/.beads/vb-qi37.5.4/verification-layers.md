# Verification Layers — vb-qi37.5.4

## Boundary

- **Verus-owned kernel**: Pure Rust logic — decision table (5 branches, 45 combinations), key taint propagation, error lattice invariants
- **TLA+ temporal model**: Decision table as reference specification (not model-checked — see tla-spec.md)
- **Theorem projection**: None (see lean-contract.md)
- **Runtime shell**: `verify_idempotency` traversal — pure deterministic function, no I/O, no async, no external calls
- **External systems excluded**: None

---

## Layer Assignment

| Contract Clause | Layer(s) | Rationale |
|---|---|---|
| INV-001: IdempotencyViolation exhaustive | verus + kani | Verus proof of exhaustiveness; Kani for concrete combinations |
| INV-002: IdempotencyContractViolation exhaustive | verus | 3-variant enum — direct Verus proof_by_cases |
| INV-003: Decision table confluence | kani + verus | Kani enumerates all 45 combinations; Verus for formal proof |
| INV-004: Single-variant error return | verus | Loop invariant in verify_idempotency |
| PRE-001: is_statically_idempotent preconditions | verus + kani | Precondition enforcement by Verus specs |
| PRE-003/PRE-004: verify_idempotency preconditions | verus + miri | Preconditions + slot index safety |
| POST-001 through POST-004: Decision table postconditions | kani + verus | Kani exhaustiveness + Verus postcondition proofs |
| POST-005 through POST-009: verify_idempotency postconditions | kani + verus + miri | Key taint propagation + slot safety |
| POST-010: Compile/runtime parity | kani | Cross-function comparison harness |
| ERR-001: SideEffectingRetryUnsafe | kani + verus | Covered by decision table |
| ERR-002: SideEffectingAtLeastOnceExternal | kani + verus | Covered by decision table |
| ERR-003: SideEffectingDeterministicPure | kani + verus | Covered by decision table |
| ERR-004 through ERR-007: Runtime violations | kani + miri | Taint checks + slot arithmetic |

---

## Verus Scope

**Rust target**: `crates/vb_validate/src/idempotency_contract.rs` and `crates/vb_core/src/action.rs`

**Spec/proof functions**:
- `spec_is_statically_idempotent_contract(contract) -> Result<(), IdempotencyContractViolation>` — Verus spec matching the Rust decision table
- `proof_decision_table_confluence()` — Verus proof that the decision table is confluent
- `proof_single_error_variant()` — Verus proof that `verify_idempotency` returns at most one error variant
- `proof_side_effecting_retry_unsafe_rejected()` — Verus proof that Unsafe + non-None side_effect is always rejected
- `proof_at_least_once_external_rejected()` — Verus proof that AtLeastOnceExternal + non-None side_effect is always rejected
- `proof_deterministic_pure_rejected()` — Verus proof that DeterministicPure + non-None side_effect is always rejected

**Invariants**:
- `INV-003`: Decision table confluence — \forall se, rs, id: DecisionTable(se, rs, id) is deterministic
- `INV-004`: Single-variant return — \forall key_slots, slot_taints: RuntimeTaint_FirstError returns at most one error

**Trusted boundary**: `is_statically_idempotent_contract` is trusted after Verus proof. Core vb_core functions assume static gate has already passed.

**Shell exclusions**: I/O, async, storage, wall-clock time — none of these apply to the pure decision table.

---

## TLA+ Scope

**Module/model path**: `IdempotencyDecisionTable`, `IdempotencyRuntimeTaint`

**Variables**:
- `side_effect ∈ {None, Writes, Sends, Creates, Destroys}`
- `retry_safety ∈ {Safe, KeyRequired, Unsafe}`
- `idempotency ∈ {DeterministicPure, IdempotentExternal, AtLeastOnceExternal}`
- `slot_taints ∈ [0..MAX_SLOTS-1 → SUBSET {Clean, SecretTaint, Random, TimeDependent}]`

**Actions**: N/A — pure function specification, no state transitions

**Safety invariants**:
- `Safety_TotalFunction`: DecisionTable always returns a valid result
- `Safety_NoneAlwaysOk`: side_effect=None always returns Ok
- `Safety_UnsafeAlwaysRejected`: Unsafe + non-None side_effect always rejected
- `Safety_OkBranches`: Exactly two Ok conditions
- `Safety_Confluence`: DecisionTable is deterministic
- `Safety_SingleErrorReporting`: RuntimeTaint_FirstError returns at most one error

**Temporal properties**: None — all properties are safety invariants

**Fairness/deadlock stance**: Not applicable

**Refinement boundary**: TLA+ modules are reference specifications only. Kani exhaustiveness testing of Rust enum combinations is the primary verification mechanism.

**Evidence command**: None — TLA+ modules are declarative specification artifacts, not model-checked.

---

## Theorem Scope

**None** — no Lean/Aeneas/Hax theorems required. Verus covers all Rust-local pure critical behavior.

---

## Kani Scope

**Rust target**: `crates/vb_validate/src/idempotency_contract.rs`

**Harness**: `idempotency_decision_table_kani` — exhaustively tests all 45 `(side_effect, retry_safety, idempotency)` combinations and verifies:
1. Each combination produces exactly one of the 4 expected results
2. No panic occurs for any combination
3. The result matches the TLA+ DecisionTable specification

**Command**: `cargo kani --harness idempotency_decision_table_kani`

**Rust target**: `crates/vb_core/src/action.rs`

**Harness**: `verify_idempotency_taint_kani` — tests all combinations of:
- 3 idempotency values × key slot count (0, 1, 2, 3) × taint patterns
- Verifies first-error priority order and no dual reporting

**Command**: `cargo kani --harness verify_idempotency_taint_kani`

**Rust target**: `crates/vb_compile/src/lib.rs` and `crates/vb_validate/src/idempotency_contract.rs`

**Harness**: `idempotency_gate_parity_kani` — cross-function comparison proving `check_idempotency_gates` and `is_statically_idempotent_contract` produce identical results for all 45 combinations

**Command**: `cargo kani --harness idempotency_gate_parity_kani`

---

## Miri Scope

**Rust target**: `crates/vb_core/src/action.rs` — `verify_idempotency` and `validate_idempotency_key_ingredients`

**Focus**: Slot index arithmetic, `SlotIdx` operations, frame slot access, bounds checks

**Command**: `cargo miri test verify_idempotency -- -Zrandom-seed=0`

---

## Proptest Scope

**Rust target**: `crates/vb_validate/src/idempotency_contract.rs`

**Strategy**: `prop_compose` over `(side_effect, retry_safety, idempotency)` tuples — broad coverage beyond the 45 enumerated cases

**Command**: `cargo test --test idempotency_contract_red -- test_is_statically_idempotent_contract` (proptest harness already exists)

---

## Waivers

- **Loom/Shuttle**: Not applicable — journal replay is single-threaded sequential; no concurrency to explore
- **Fuzzing**: Not required — proptest already covers the input space; no parser or malformed input handling
- **TLA+ model-checking**: Waived — decision table is pure deterministic function; Kani exhaustive testing provides stronger assurance than symbolic model-checking for this finite enum domain
- **Lean/Aeneas/Hax**: Not required — Verus covers all Rust-local pure critical behavior; no algebraic theorem kernels needed
