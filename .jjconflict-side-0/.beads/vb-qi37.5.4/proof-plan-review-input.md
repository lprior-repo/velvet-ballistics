# Proof Plan Review Input — vb-qi37.5.4

## Bead Context

- **Bead ID**: vb-qi37.5.4
- **Title**: verifier: Idempotency gate evidence tests
- **Current State**: 4 (proof-planner)
- **Next State**: 5 (proof-writer)
- **Source Checkout**: /home/lewis/src/Velvet-ballistics
- **Isolated Workspace**: /home/lewis/src/vb-qi37-5-4

---

## Scope

The proof plan covers the idempotency gate logic across three crates:
- **vb_validate**: static decision table (`is_statically_idempotent_contract`)
- **vb_core**: runtime gate (`verify_idempotency`, `validate_idempotency_key_ingredients`)
- **vb_compile**: compile-time gate (`check_idempotency_gates`)

The vb_runtime build failure is **DEFERRED_GLOBAL** and outside this bead's scope.

---

## 24 Obligations by Layer

### Kani (12 obligations)

| ID | Target | Claim | Mode |
|----|--------|-------|------|
| KANI-DECISION-001 | is_statically_idempotent_contract | Decision table confluent for all 45 combos | verify-proof |
| KANI-DECISION-002 | is_statically_idempotent_contract | Ok branch: side_effect==None or IdempotentExternal+Safe/KeyRequired | verify-proof |
| KANI-DECISION-003 | is_statically_idempotent_contract | Err(SideEffectingRetryUnsafe) for Unsafe + non-None side_effect | verify-proof |
| KANI-DECISION-004 | is_statically_idempotent_contract | Err(SideEffectingAtLeastOnceExternal) for AtLeastOnceExternal + non-None side_effect | verify-proof |
| KANI-DECISION-005 | is_statically_idempotent_contract | Err(SideEffectingDeterministicPure) for DeterministicPure + non-None side_effect | verify-proof |
| **KANI-PARITY-001** | check_idempotency_gates + is_statically_idempotent_contract | **Both gates agree on all 45 combinations (CRITICAL)** | verify-proof |
| KANI-RUNTIME-001 | verify_idempotency | Ok when all key slots clean | verify-proof |
| KANI-RUNTIME-002 | verify_idempotency | Err(MissingKey) for empty key_slots with IdempotentExternal | verify-proof |
| KANI-RUNTIME-003 | verify_idempotency | Err(SecretInKey) for SecretTaint in key slot | verify-proof |
| KANI-RUNTIME-004 | verify_idempotency | Err(RandomInKey) for Random in key slot | verify-proof |
| KANI-RUNTIME-005 | verify_idempotency | Err(TimeInKey) for TimeDependent in key slot | verify-proof |
| KANI-RUNTIME-006 | verify_idempotency | At most one error variant returned (no dual reporting) | verify-proof |

### Verus (5 obligations)

| ID | Target | Claim | Mode |
|----|--------|-------|------|
| VERUS-DECISION-001 | is_statically_idempotent_contract | Decision table determinism: f(x) = f(x) | verify-proof |
| VERUS-DECISION-002 | IdempotencyViolation enum | 4 exhaustive variants, mutually exclusive | verify-proof |
| VERUS-DECISION-003 | IdempotencyContractViolation enum | 3 exhaustive variants covering 3 rejection branches | verify-proof |
| VERUS-RUNTIME-001 | verify_idempotency | At most one IdempotencyViolation variant returned | verify-proof |
| VERUS-RUNTIME-002 | verify_idempotency | Precondition: requires IdempotentExternal && non-empty key_slots | verify-standard |

### Miri (2 obligations — deferred to State 11)

| ID | Target | Claim | Mode |
|----|--------|-------|------|
| MIRI-RUNTIME-001 | verify_idempotency + validate_idempotency_key_ingredients | No UB, no OOB, no use-after-free for slot index ops | verify-deep |
| MIRI-RUNTIME-002 | validate_idempotency_key_ingredients | No UB for slot index iteration 0..16 | verify-deep |

### Proptest (2 obligations — deferred to State 8)

| ID | Target | Claim | Mode |
|----|--------|-------|------|
| PROPTEST-001 | is_statically_idempotent_contract | Confluence across broad input generation | verify-standard |
| PROPTEST-002 | verify_idempotency | Consistent results across broad key slot taint patterns | verify-standard |

### cargo test (3 obligations — deferred to State 8)

| ID | Target | Claim | Mode |
|----|--------|-------|------|
| TEST-UNIT-001 | is_statically_idempotent_contract | All 5 decision table branches covered | verify-standard |
| TEST-UNIT-002 | verify_idempotency | All 5 runtime paths covered | verify-standard |
| TEST-INTEGRATION-001 | check_idempotency_gates | Parity with is_statically_idempotent_contract across workflow scenarios | verify-standard |

---

## Critical Review Points

### 1. KANI-PARITY-001 — compile/runtime gate parity

This is the bead's most critical obligation. The proof must show that:
- `check_idempotency_gates` in vb_compile (compile-time gate)
- `is_statically_idempotent_contract` in vb_validate (static gate)

...produce identical accept/reject results for all 45 `(side_effect, retry_safety, idempotency)` combinations.

**Reviewer must verify**: The harness enumerates all 45 combinations and asserts `result_a == result_b` for each. If the two functions have different return types, the harness must normalize comparison appropriately.

### 2. Cross-crate Kani scope

KANI-PARITY-001 requires vb_compile and vb_validate in the same Kani proof scope. The proof-writer must ensure the Cargo workspace or explicit `-p vb_compile -p vb_validate` flags include both crates.

**Reviewer must verify**: The kani harness for KANI-PARITY-001 is located where both crates are accessible, and the command includes both `-p` flags.

### 3. No existing proof annotations

Discovery confirmed: no `kani::`, `verus::`, or proof annotations exist in the target source files. All 17 Kani/Verus proof artifacts must be created from scratch by proof-writer.

**Reviewer must verify**: New harness files are created in `kani/` and/or annotations added to source files, not modified from existing proofs.

### 4. Miri deferred to State 11

MIRI-RUNTIME-001 and MIRI-RUNTIME-002 have `owner_state: 11` and `rerun_from: 11`. These are not proof artifacts to be written in State 5, but execution obligations for State 11.

**Reviewer must verify**: These obligations are marked `status: planned` with `owner_state: 11`, not `owner_state: 5`.

### 5. Proptest and test deferred to State 8

PROPTEST-001, PROPTEST-002, TEST-UNIT-001, TEST-UNIT-002, and TEST-INTEGRATION-001 have `owner_state: 8`. These are test obligations, not proof obligations for State 5.

**Reviewer must verify**: These obligations are correctly routed to State 8 and the proof-strategy.md does not claim proof-writer will execute them.

### 6. Enum exhaustiveness (VERUS-DECISION-002, VERUS-DECISION-003)

These prove that the error enum variants are exhaustive and non-overlapping. For Rust enums without `#[repr]` or complex invariants, Verus `proof_by_cases` is the canonical approach.

**Reviewer must verify**: The Verus proofs cover all variants and demonstrate mutual exclusion.

---

## Contract Clause Coverage

All 24 contract clauses from `contract.md` have at least one proof or test obligation:

| Clause | Proof | Test |
|--------|-------|------|
| INV-001 | VERUS-DECISION-002, KANI-RUNTIME-001..006 | TEST-UNIT-002 |
| INV-002 | VERUS-DECISION-003, KANI-DECISION-001 | TEST-UNIT-001 |
| INV-003 | VERUS-DECISION-001, KANI-DECISION-001..005 | PROPTEST-001, TEST-UNIT-001 |
| INV-004 | VERUS-RUNTIME-001, KANI-RUNTIME-006 | TEST-UNIT-002 |
| PRE-001 | KANI-DECISION-001, KANI-DECISION-002 | TEST-UNIT-001 |
| PRE-003 | VERUS-RUNTIME-002, KANI-RUNTIME-001 | TEST-UNIT-002 |
| PRE-004 | MIRI-RUNTIME-001, MIRI-RUNTIME-002 | TEST-UNIT-002 |
| POST-001 | KANI-DECISION-002 | TEST-UNIT-001, PROPTEST-001 |
| POST-002 | KANI-DECISION-003 | TEST-UNIT-001 |
| POST-003 | KANI-DECISION-004 | TEST-UNIT-001 |
| POST-004 | KANI-DECISION-005 | TEST-UNIT-001 |
| POST-005 | KANI-RUNTIME-001, VERUS-RUNTIME-001 | TEST-UNIT-002, PROPTEST-002 |
| POST-006 | KANI-RUNTIME-002 | TEST-UNIT-002 |
| POST-007 | KANI-RUNTIME-003 | TEST-UNIT-002 |
| POST-008 | KANI-RUNTIME-004 | TEST-UNIT-002 |
| POST-009 | KANI-RUNTIME-005 | TEST-UNIT-002 |
| POST-010 | KANI-PARITY-001 | TEST-INTEGRATION-001 |

---

## Reviewer Checklist

- [ ] proof-strategy.md exists and is non-empty
- [ ] proof-obligations.planned.jsonl exists with 24 rows
- [ ] All 12 Kani obligations have harness names and commands
- [ ] All 5 Verus obligations have spec/proof function names
- [ ] KANI-PARITY-001 is flagged as critical and cross-crate
- [ ] Miri obligations correctly have `owner_state: 11`
- [ ] Proptest/test obligations correctly have `owner_state: 8`
- [ ] Every obligation has a `requirement_id` or `contract_clause` mapping
- [ ] No obligation is marked `PASS` (only `planned`, `waived`, `blocked_tooling`, `not_applicable`)
- [ ] vb_runtime DEFERRED_GLOBAL is not included in any obligation
- [ ] JSONL parses correctly with `jq -c .`
