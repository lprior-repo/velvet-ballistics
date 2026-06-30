# Proof Strategy — vb-qi37.5.4

## Overview

Bead **vb-qi37.5.4** ("verifier: Idempotency gate evidence tests") requires proof evidence
across 24 obligations in 5 verification layers. The proof target is the idempotency gate
logic: compile-time static validation (`is_statically_idempotent_contract`) and runtime
validation (`verify_idempotency`).

**vb_runtime build failure is DEFERRED_GLOBAL. It is outside this bead's scope.**

---

## Verification Layer Summary

| Layer | Count | Critical? | Notes |
|-------|-------|----------|-------|
| Kani  | 12    | YES      | Decision table (6), runtime paths (5), compile/runtime parity (1) |
| Verus | 5     | Yes      | Decision confluence (1), enum exhaustiveness (2), runtime single-error (2) |
| Miri  | 2     | High     | Slot index UB — deferred to State 11 execution |
| Proptest | 2 | Medium | Broad input coverage, deferred to State 8 (test loop) |
| cargo test | 3 | Medium | Unit/integration coverage, deferred to State 8 |

---

## Critical Obligation: KANI-PARITY-001

**KANI-PARITY-001** is the most critical obligation in this bead.

- **What**: Cross-crate parity between `check_idempotency_gates` (vb_compile) and
  `is_statically_idempotent_contract` (vb_validate). Both must accept/reject identical
  `(side_effect, retry_safety, idempotency)` combinations.
- **Why critical**: This is the core correctness guarantee for the idempotency gate feature.
  If the compile-time gate and static validation gate disagree, action contracts validated
  at compile time may be rejected at runtime (or vice versa), breaking the contract model.
- **Approach**: A single Kani harness that:
  1. Enumerates all 45 combinations.
  2. Feeds each to both `check_idempotency_gates` and `is_statically_idempotent_contract`.
  3. Asserts both return the same `Ok/Err` result.
- **Harness location**: `kani/idempotency_gate_parity.rs`
- **Execution**: `cargo kani --harness idempotency_gate_parity`
- **Scope note**: vb_compile and vb_validate are separate crates. The harness must include
  both in the Kani proof scope via Cargo.toml workspace members or explicit `-p` flags.

---

## Layer-Specific Strategy

### Kani (12 obligations: KANI-DECISION-001..005, KANI-PARITY-001, KANI-RUNTIME-001..006)

**Target files**:
- `crates/vb_validate/src/idempotency_contract.rs` — static decision table
- `crates/vb_core/src/action.rs` — runtime gate
- `crates/vb_compile/src/lib.rs` — compile-time gate (for KANI-PARITY-001)

**Strategy**:
- All 12 Kani obligations require creating new harnesses in `kani/` directory.
- No existing `kani::` annotations in the target files.
- Harnesses must be written as separate `.rs` files in `kani/` using `kani::harness`.
- Decision-table harnesses use `kani::any()` for SideEffect/RetrySafety/Idempotency and
  constrain to the relevant subset per branch.
- Runtime harnesses use `kani::any()` for RunFrame and SlotIdx with constrained taint values.
- KANI-PARITY-001 requires both vb_compile and vb_validate in scope; use
  `cargo kani -p vb_compile -p vb_validate --harness idempotency_gate_parity`.

**Bounds**:
- 45 combinations = 5 SideEffect × 3 RetrySafety × 3 Idempotency
- Key slot indices: 0..16 (bounded for Kani)
- Taint values: SecretTaint, Random, TimeDependent (3 variants + clean)

**Commands** (State 5 proof-writer):
```sh
cargo kani -p vb_validate --harness is_statically_idempotent_contract
cargo kani -p vb_validate --harness decision_table_ok_branch
cargo kani -p vb_validate --harness decision_table_unsafe_rejected
cargo kani -p vb_validate --harness decision_table_at_least_once_rejected
cargo kani -p vb_validate --harness decision_table_deterministic_rejected
cargo kani -p vb_compile -p vb_validate --harness idempotency_gate_parity
cargo kani -p vb_core --harness verify_idempotency_all_clean
cargo kani -p vb_core --harness verify_idempotency_missing_key
cargo kani -p vb_core --harness verify_idempotency_secret_in_key
cargo kani -p vb_core --harness verify_idempotency_random_in_key
cargo kani -p vb_core --harness verify_idempotency_time_in_key
cargo kani -p vb_core --harness verify_idempotency_single_error
```

---

### Verus (5 obligations: VERUS-DECISION-001..003, VERUS-RUNTIME-001..002)

**Target files**:
- `crates/vb_validate/src/idempotency_contract.rs`
- `crates/vb_core/src/action.rs`

**Strategy**:
- Verus obligations target type-level invariants and pure function specs, not executable proofs.
- `VERUS-DECISION-001`: spec + proof for decision table confluence (determinism).
- `VERUS-DECISION-002`: spec + proof for IdempotencyViolation enum exhaustiveness (4 variants).
- `VERUS-DECISION-003`: spec + proof for IdempotencyContractViolation enum exhaustiveness (3 variants).
- `VERUS-RUNTIME-001`: spec + proof for single-error invariant on verify_idempotency.
- `VERUS-RUNTIME-002`: requires clause on verify_idempotency (precondition).

**Approach**:
- Add `#[spec]` and `#[proof]` functions to the existing source files or a `verification/verus/`
  companion module.
- Use `proof_by_cases` for enum exhaustiveness.
- Use loop invariants for single-error property.
- Verus runs via `cargo verus` or `verus` binary on target files.

**Commands** (State 5 proof-writer):
```sh
verus crates/vb_validate/src/idempotency_contract.rs
verus crates/vb_core/src/action.rs
```

---

### Miri (2 obligations: MIRI-RUNTIME-001, MIRI-RUNTIME-002)

**Strategy**:
- Miri obligations are deferred to State 11 (formal-verifier) because they require actual
  execution with Miri, not proof annotation.
- `MIRI-RUNTIME-001`: tests `verify_idempotency` under Miri for UB/out-of-bounds.
- `MIRI-RUNTIME-002`: tests `validate_idempotency_key_ingredients` under Miri.
- Both obligations have `owner_state: 11` and `rerun_from: 11`.

**Commands** (State 11 formal-verifier):
```sh
cargo miri test verify_idempotency -- -Zrandom-seed=0
cargo miri test validate_idempotency_key_ingredients -- -Zrandom-seed=0
```

**Assumption**: Miri is available in the execution environment. If not, mark `blocked_tooling`.

---

### Proptest (2 obligations: PROPTEST-001, PROPTEST-002)

**Strategy**:
- Proptest obligations are deferred to State 8 (test-writer) because they are test obligations
  with `owner_state: 8` and `rerun_from: 8`.
- `PROPTEST-001`: proptest harness for decision table confluence.
- `PROPTEST-002`: proptest harness for verify_idempotency taint patterns.
- The existing `crates/vb_validate/tests/idempotency_contract_red.rs` has 2 proptest harnesses
  already (28+ unit tests + 2 proptest). These may cover PROPTEST-001 and PROPTEST-002, but
  proof-writer must verify and extend if coverage is incomplete.

**Commands** (State 8 test-writer):
```sh
cargo test --test idempotency_contract_red -- test_is_statically_idempotent_contract
cargo test --test idempotency_contract_red -- test_verify_idempotency
```

---

### cargo test (3 obligations: TEST-UNIT-001, TEST-UNIT-002, TEST-INTEGRATION-001)

**Strategy**:
- All 3 test obligations are deferred to State 8 (test-writer) with `owner_state: 8`.
- `TEST-UNIT-001`: unit tests for is_statically_idempotent_contract (5 branches).
- `TEST-UNIT-002`: unit tests for verify_idempotency (5 runtime paths).
- `TEST-INTEGRATION-001`: integration tests for check_idempotency_gates parity.

**Commands** (State 8 test-writer):
```sh
cargo test -p vb_validate -- idempotency
cargo test -p vb_core -- verify_idempotency
cargo test -p vb_compile -- check_idempotency
```

---

## Obligation Execution Order

### State 5 (proof-writer) creates:
1. Kani harnesses (12 total): decision table (5), parity (1), runtime (6)
2. Verus specs/proofs (5 total): added to source files or verus/ module

### State 11 (formal-verifier) executes:
1. Miri runs (2): `verify_idempotency` + `validate_idempotency_key_ingredients`
2. Kani runs (12): all Kani obligations
3. Verus runs (5): all Verus obligations

### State 8 (test-writer) creates:
1. Proptest harnesses (2): PROPTEST-001, PROPTEST-002
2. Unit tests (2): TEST-UNIT-001, TEST-UNIT-002
3. Integration tests (1): TEST-INTEGRATION-001

---

## Cross-Crate Dependency Notes

- **KANI-PARITY-001** requires both `vb_compile` and `vb_validate` in scope.
  The Kani workspace or Cargo.toml must include both crates.
- **MIRI-RUNTIME-001/002** target `vb_core` only — no cross-crate dependency.
- **TEST-INTEGRATION-001** requires `vb_compile` and `vb_validate` — test scope only,
  no proof dependency.

---

## Waiver Candidates

If any verifier tool is unavailable at State 11 execution time, the obligation row must be
updated with `status: waived` and a waiver object containing: owner, reason, expiry,
compensating evidence, and follow-up trigger.

---

## Risk Summary

| Risk | Mitigation |
|------|------------|
| KANI-PARITY-001 cross-crate harness complexity | proof-writer must verify both crates in Kani scope |
| No existing Kani/Verus annotations | proof-writer creates from scratch |
| vb_runtime DEFERRED_GLOBAL | out of scope; no mitigation needed |
| Proptest coverage gap | test-writer must verify existing 28+ tests cover all branches |
