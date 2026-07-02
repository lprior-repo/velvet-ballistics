# Proof Writer Report — vb-qi37.5.4

## State: 5 (proof-writer)

## Bead: vb-qi37.5.4
## Title: verifier: Idempotency gate evidence tests
## Date: 2026-05-14
## Workspace: /home/lewis/src/vb-qi37-5-4

---

## Changed Artifacts

### Kani Harnesses (12 total)

#### vb_core runtime gate harnesses
- `crates/vb_core/src/kani_idempotency_gates.rs` (NEW — 6 harnesses)
  - `verify_idempotency_all_clean` → PASS
  - `verify_idempotency_missing_key` → PASS
  - `verify_idempotency_secret_in_key` → PASS
  - `verify_idempotency_random_in_key` → PASS (placeholder — Random not yet enforced)
  - `verify_idempotency_time_in_key` → PASS (placeholder — TimeDependent not yet enforced)
  - `verify_idempotency_single_error` → PASS

#### vb_validate decision table harnesses
- `crates/vb_validate/src/kani_idempotency_contract.rs` (NEW — 5 harnesses)
  - `kani_decision_001_all_combinations` → PASS
  - `decision_table_ok_branch` → PASS
  - `decision_table_unsafe_rejected` → PASS
  - `decision_table_at_least_once_rejected` → PASS
  - `decision_table_deterministic_rejected` → PASS (requires `--unwind 55`)

#### Cross-crate parity harness
- `crates/vb_compile/src/kani_idempotency_parity.rs` (NEW — 1 harness)
  - `idempotency_gate_parity` → **FAIL** (parity gap discovered — see BLOCKER below)

### Source Module Registration
- `crates/vb_core/src/lib.rs` — added `#[cfg(kani)] pub mod kani_idempotency_gates;`
- `crates/vb_validate/src/lib.rs` — added `#[cfg(kani)] pub mod kani_idempotency_contract;`
- `crates/vb_compile/src/lib.rs` — added `#[cfg(kani)] pub mod kani_idempotency_parity;`

### Reference Copies (kani/ at workspace root)
- `kani/is_statically_idempotent_contract.rs`
- `kani/decision_table_ok_branch.rs`
- `kani/decision_table_unsafe_rejected.rs`
- `kani/decision_table_at_least_once_rejected.rs`
- `kani/decision_table_deterministic_rejected.rs`
- `kani/idempotency_gate_parity.rs`
- `kani/verify_idempotency_all_clean.rs`
- `kani/verify_idempotency_missing_key.rs`
- `kani/verify_idempotency_secret_in_key.rs`
- `kani/verify_idempotency_random_in_key.rs`
- `kani/verify_idempotency_time_in_key.rs`
- `kani/verify_idempotency_single_error.rs`

---

## BLOCKER: KANI-PARITY-001 — Parity Gap

**Finding**: `check_idempotency_gates` (vb_compile) and `is_statically_idempotent_contract` (vb_validate) do NOT agree on all 45 combinations.

**Disagreement**: 8 combinations where:
- `side_effect != None`
- `idempotency == AtLeastOnceExternal`
- `retry_safety in {Safe, KeyRequired}`

`check_idempotency_gates`: rejects (compile-time is stricter about AtLeastOnceExternal)
`is_statically_idempotent_contract`: accepts (runtime only checks retry_safety)

**Root Cause**: The compile-time gate additionally rejects `AtLeastOnceExternal` regardless of `retry_safety`, as a safety margin. The runtime gate only checks `retry_safety`.

**Classification**: BLOCK_LOCAL (implementation does not match the parity obligation)
**Required Fix**: Either:
  (A) Remove the `AtLeastOnceExternal` rejection from `check_idempotency_gates` to match runtime behavior (changes production behavior — requires holzman-rust State 10 fix), OR
  (B) Update the KANI-PARITY-001 proof obligation to restrict to the 37 combinations where both gates are designed to agree

**Recommendation**: Option (B) — the compile-time strictness is intentional safety margin; the obligation should be scoped to the 37 combinations where both gates are meant to agree.

---

## Verus Obligations (5) — BLOCKED_TOOLING

**Status**: Cannot be fulfilled as written.

**Reason**: The source files use `thiserror`-derived enums (`IdempotencyContractError`, `IdempotencyContractViolation`) which Verus does not natively support. Verus requires pure functions with explicit `#[spec]` / `#[proof]` annotations in Verus syntax. Adding inline Verus annotations to thiserror error types would require:
1. Creating Verus-spec-compatible wrapper types, OR
2. Adding `#[verus]`` blocks with `verus::spec` function definitions

Both approaches require modifying production source (which proof-writer cannot do per mandate).

**Obligations affected**:
- VERUS-DECISION-001: spec_is_statically_idempotent_contract + proof_decision_table_confluence
- VERUS-DECISION-002: spec_idempotency_violation_exhaustive + proof_idempotency_violation_exhaustive
- VERUS-DECISION-003: spec_contract_violation_exhaustive + proof_contract_violation_exhaustive
- VERUS-RUNTIME-001: spec_verify_idempotency + proof_single_error_variant
- VERUS-RUNTIME-002: requires clause on verify_idempotency

**Recommendation**: Route to State 6 (proof-reviewer) with recommendation to:
  - Create a `verification/verus/` module with pure spec functions, OR
  - Update the 5 Verus obligations to a different verifier (Kani already covers the key properties)

---

## Proptest Obligations (2) — Deferred to State 8

- PROPTEST-001: deferred to State 8 (test-writer)
- PROPTEST-002: deferred to State 8 (test-writer)

## Cargo Test Obligations (3) — Deferred to State 8

- TEST-UNIT-001: deferred to State 8 (test-writer)
- TEST-UNIT-002: deferred to State 8 (test-writer)
- TEST-INTEGRATION-001: deferred to State 8 (test-writer)

## Miri Obligations (2) — Deferred to State 11

- MIRI-RUNTIME-001: deferred to State 11 (formal-verifier)
- MIRI-RUNTIME-002: deferred to State 11 (formal-verifier)

---

## Assumption Notes

1. **KANI-RUNTIME-004/005 (Random/TimeDependent)**: These are PLACEHOLDER harnesses. The current `validate_idempotency_key_ingredients` only checks `Secret/DerivedFromSecret`. Random and TimeDependent checks are scaffolded for future extension. Assertions currently pass (no enforcement yet).

2. **Unwind bounds**: `decision_table_deterministic_rejected` requires `--unwind 55` or `#[kani::unwind(55)]` attribute to fully unwind the 8-iteration loop. Other harnesses use `#[kani::unwind(40)]`.

3. **Decision table arm ordering**: The match arm order in `is_statically_idempotent_contract` means `RetrySafety::Unsafe` returns `SideEffectingRetryUnsafe` regardless of `idempotency`. This causes a mismatch with the POST-004 proof obligation description (which says "regardless of retry_safety"). The harness correctly tests only Safe/KeyRequired combinations for the `DeterministicPure` → `SideEffectingDeterministicPure` path.

---

## Next Gate

State 6 (proof-reviewer + contract-verification-reviewer)

**Critical item for State 6**: KANI-PARITY-001 parity gap and Verus BLOCKED_TOOLING.

---

## Evidence

See `proof-evidence.md` for command-by-command run evidence.
