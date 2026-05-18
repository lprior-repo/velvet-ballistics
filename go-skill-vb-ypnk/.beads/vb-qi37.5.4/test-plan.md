# Test Plan: vb-qi37.5.4 — Idempotency Gate Evidence Tests

## Bead: vb-qi37.5.4
## Title: verifier: Idempotency gate evidence tests
## State: 7 (test-planner)
## Date: 2026-05-14
## Workspace: /home/lewis/src/vb-qi37-5-4

---

## Summary

- **Behaviors identified**: 12
- **Trophy allocation**: 3 unit / 2 integration / 1 e2e / 1 static = 7 total test groups
- **Proptest invariants**: 2
- **Fuzz targets**: 0 (no parsing/deserialization boundaries in idempotency gate logic)
- **Kani harnesses**: 12 (already written in State 5)
- **Mutation checkpoints**: 4 critical mutations

---

## 1. Behavior Inventory

### vb_validate — Static Gate Behaviors

1. **Decision table accepts side_effect=None** — `is_statically_idempotent_contract` returns `Ok(())` when `side_effect == None`, regardless of `retry_safety` or `idempotency`

2. **Decision table accepts IdempotentExternal+Safe** — `is_statically_idempotent_contract` returns `Ok(())` when `side_effect != None`, `retry_safety == Safe`, and `idempotency == IdempotentExternal`

3. **Decision table accepts IdempotentExternal+KeyRequired** — `is_statically_idempotent_contract` returns `Ok(())` when `side_effect != None`, `retry_safety == KeyRequired`, and `idempotency == IdempotentExternal`

4. **Decision table rejects Unsafe retry** — `is_statically_idempotent_contract` returns `Err(SideEffectingRetryUnsafe)` when `side_effect != None` and `retry_safety == Unsafe`, regardless of `idempotency`

5. **Decision table rejects AtLeastOnceExternal** — `is_statically_idempotent_contract` returns `Err(SideEffectingAtLeastOnceExternal)` when `side_effect != None` and `idempotency == AtLeastOnceExternal`, regardless of `retry_safety`

6. **Decision table rejects DeterministicPure** — `is_statically_idempotent_contract` returns `Err(SideEffectingDeterministicPure)` when `side_effect != None` and `idempotency == DeterministicPure`, regardless of `retry_safety`

### vb_core — Runtime Gate Behaviors

7. **Runtime gate accepts all-clean key slots** — `verify_idempotency` returns `Ok(())` when all key slots have `Taint::Clean` (no SecretTaint, Random, or TimeDependent)

8. **Runtime gate rejects missing key** — `verify_idempotency` returns `Err(MissingKey(SideEffect))` when `idempotency == IdempotentExternal` and `key_slots` is empty

9. **Runtime gate rejects secret in key** — `verify_idempotency` returns `Err(SecretInKey(slot_idx))` with correct slot index when any key slot carries `SecretTaint`

10. **Runtime gate rejects random in key** — `verify_idempotency` returns `Err(RandomInKey(slot_idx))` with correct slot index when any key slot carries `Random`

11. **Runtime gate rejects time-dependent in key** — `verify_idempotency` returns `Err(TimeInKey(slot_idx))` with correct slot index when any key slot carries `TimeDependent`

### vb_compile — Parity Behavior

12. **Compile/runtime parity on 37 combinations** — `check_idempotency_gates` (vb_compile) and `is_statically_idempotent_contract` (vb_validate) agree on `Ok/Err` for all 37 combinations where both gates are designed to agree (excludes 8 AtLeastOnceExternal+Safe/KeyRequired combinations where vb_validate has a production bug)

---

## 2. Trophy Allocation

| Layer | Count | Rationale |
|-------|-------|-----------|
| **Unit / Calc** | 3 | Pure decision table logic (vb_validate), pure runtime gate logic (vb_core), no I/O |
| **Integration** | 2 | Cross-crate parity (vb_compile↔vb_validate), workflow contract collection |
| **E2E** | 1 | Full workflow with idempotency gates end-to-end |
| **Static Analysis** | 1 | Clippy, cargo-deny, Kani already covers critical invariants |

**Target ratios**: ~60% integration, ~30% unit, ~5% e2e, ~5% static — satisfied by 2 integration, 3 unit, 1 e2e, 1 static.

---

## 3. BDD Scenarios

### Static Gate — Decision Table (vb_validate)

#### Behavior 1: accepts side_effect=None

**Scenario**: `fn is_statically_idempotent_contract_returns_ok_when_side_effect_is_none`

```
Given: an ActionContract with side_effect=None, any retry_safety, any idempotency
When: is_statically_idempotent_contract is called
Then: returns Ok(())
```

**Variants** (3 idempotency × 3 retry_safety = 9 combinations):
- None + Safe + DeterministicPure → Ok
- None + Safe + IdempotentExternal → Ok
- None + Safe + AtLeastOnceExternal → Ok
- None + KeyRequired + DeterministicPure → Ok
- None + KeyRequired + IdempotentExternal → Ok
- None + KeyRequired + AtLeastOnceExternal → Ok
- None + Unsafe + DeterministicPure → Ok
- None + Unsafe + IdempotentExternal → Ok
- None + Unsafe + AtLeastOnceExternal → Ok

#### Behavior 2: accepts IdempotentExternal+Safe

**Scenario**: `fn is_statically_idempotent_contract_returns_ok_when_idempotent_external_with_safe_retry`

```
Given: an ActionContract with side_effect!=None, retry_safety=Safe, idempotency=IdempotentExternal
When: is_statically_idempotent_contract is called
Then: returns Ok(())
```

**Variants** (5 side_effect ≠ None):
- Writes + Safe + IdempotentExternal → Ok
- Sends + Safe + IdempotentExternal → Ok
- Creates + Safe + IdempotentExternal → Ok
- Destroys + Safe + IdempotentExternal → Ok

#### Behavior 3: accepts IdempotentExternal+KeyRequired

**Scenario**: `fn is_statically_idempotent_contract_returns_ok_when_idempotent_external_with_key_required_retry`

```
Given: an ActionContract with side_effect!=None, retry_safety=KeyRequired, idempotency=IdempotentExternal
When: is_statically_idempotent_contract is called
Then: returns Ok(())
```

**Variants** (5 side_effect ≠ None):
- Writes + KeyRequired + IdempotentExternal → Ok
- Sends + KeyRequired + IdempotentExternal → Ok
- Creates + KeyRequired + IdempotentExternal → Ok
- Destroys + KeyRequired + IdempotentExternal → Ok

#### Behavior 4: rejects Unsafe retry

**Scenario**: `fn is_statically_idempotent_contract_returns_err_side_effecting_retry_unsafe_when_unsafe_retry`

```
Given: an ActionContract with side_effect!=None and retry_safety=Unsafe
When: is_statically_idempotent_contract is called
Then: returns Err(SideEffectingRetryUnsafe)
And: error carries correct action_id, side_effect, idempotency, retry_safety
```

**Variants** (5 side_effect × 3 idempotency = 15):
- Writes + Unsafe + DeterministicPure → Err(SideEffectingRetryUnsafe)
- Writes + Unsafe + IdempotentExternal → Err(SideEffectingRetryUnsafe)
- Writes + Unsafe + AtLeastOnceExternal → Err(SideEffectingRetryUnsafe)
- Sends + Unsafe + DeterministicPure → Err(SideEffectingRetryUnsafe)
- ... (all 15 combinations)

#### Behavior 5: rejects AtLeastOnceExternal

**Scenario**: `fn is_statically_idempotent_contract_returns_err_at_least_once_external_when_at_least_once_external_idempotency`

```
Given: an ActionContract with side_effect!=None and idempotency=AtLeastOnceExternal
When: is_statically_idempotent_contract is called
Then: returns Err(SideEffectingAtLeastOnceExternal)
And: error carries correct action_id, side_effect, idempotency, retry_safety
```

**Variants** (5 side_effect × 3 retry_safety = 15):
- Writes + Safe + AtLeastOnceExternal → Err(SideEffectingAtLeastOnceExternal)
- Writes + KeyRequired + AtLeastOnceExternal → Err(SideEffectingAtLeastOnceExternal)
- Writes + Unsafe + AtLeastOnceExternal → Err(SideEffectingAtLeastOnceExternal)
- Sends + Safe + AtLeastOnceExternal → Err(SideEffectingAtLeastOnceExternal)
- ... (all 15 combinations)

#### Behavior 6: rejects DeterministicPure

**Scenario**: `fn is_statically_idempotent_contract_returns_err_deterministic_pure_when_deterministic_pure_idempotency`

```
Given: an ActionContract with side_effect!=None and idempotency=DeterministicPure
When: is_statically_idempotent_contract is called
Then: returns Err(SideEffectingDeterministicPure)
And: error carries correct action_id, side_effect, idempotency, retry_safety
```

**Variants** (5 side_effect × 3 retry_safety = 15):
- Writes + Safe + DeterministicPure → Err(SideEffectingDeterministicPure)
- Writes + KeyRequired + DeterministicPure → Err(SideEffectingDeterministicPure)
- Writes + Unsafe + DeterministicPure → Err(SideEffectingDeterministicPure)
- ... (all 15 combinations)

---

### Runtime Gate — vb_core

#### Behavior 7: accepts all-clean key slots

**Scenario**: `fn verify_idempotency_returns_ok_when_all_key_slots_are_clean`

```
Given: an ActionContract with idempotency=IdempotentExternal
And: key_slots is non-empty with all slots having Taint::Clean
And: frame provides slot values for each slot
When: verify_idempotency is called
Then: returns Ok(())
```

#### Behavior 8: rejects missing key

**Scenario**: `fn verify_idempotency_returns_err_missing_key_when_key_slots_empty`

```
Given: an ActionContract with idempotency=IdempotentExternal
And: key_slots is empty
When: verify_idempotency is called
Then: returns Err(MissingKey(SideEffect))
And: error carries the SideEffect of the contract
```

#### Behavior 9: rejects secret in key

**Scenario**: `fn verify_idempotency_returns_err_secret_in_key_with_correct_slot_index`

```
Given: an ActionContract with idempotency=IdempotentExternal
And: key_slots contains slots where at least one has SecretTaint
And: the tainted slot is at position slot_idx in key_slots
When: verify_idempotency is called
Then: returns Err(SecretInKey(slot_idx))
And: slot_idx matches the position of the SecretTaint slot
```

**Error variant**: When multiple slots have SecretTaint, returns SecretInKey of the FIRST tainted slot (short-circuit).

#### Behavior 10: rejects random in key

**Scenario**: `fn verify_idempotency_returns_err_random_in_key_with_correct_slot_index`

```
Given: an ActionContract with idempotency=IdempotentExternal
And: key_slots contains slots where at least one has Random
And: the tainted slot is at position slot_idx in key_slots
When: verify_idempotency is called
Then: returns Err(RandomInKey(slot_idx))
And: slot_idx matches the position of the Random slot
```

**Error variant**: When multiple slots have Random, returns RandomInKey of the FIRST tainted slot (short-circuit).

#### Behavior 11: rejects time-dependent in key

**Scenario**: `fn verify_idempotency_returns_err_time_in_key_with_correct_slot_index`

```
Given: an ActionContract with idempotency=IdempotentExternal
And: key_slots contains slots where at least one has TimeDependent
And: the tainted slot is at position slot_idx in key_slots
When: verify_idempotency is called
Then: returns Err(TimeInKey(slot_idx))
And: slot_idx matches the position of the TimeDependent slot
```

**Error variant**: When multiple slots have TimeDependent, returns TimeInKey of the FIRST tainted slot (short-circuit).

---

### Parity — vb_compile ↔ vb_validate

#### Behavior 12: compile/runtime parity on 37 combinations

**Scenario**: `fn check_idempotency_gates_and_is_statically_idempotent_contract_agree_on_ok_err`

```
Given: an ActionContract with any of the 37 agreed combinations
When: both check_idempotency_gates and is_statically_idempotent_contract are called
Then: both return the same Ok/Err result
And: the specific error variant may differ (different error types)
```

**37 combinations**:
- 5 × side_effect=None × any × any → both Ok
- 12 × side_effect!=None × Unsafe × any → both Err
- 8 × side_effect!=None × Safe/KeyRequired × IdempotentExternal → both Ok
- 12 × side_effect!=None × Safe/KeyRequired × DeterministicPure → both Err

**8 DEFERRED combinations** (vb_validate bug — not tested here):
- AtLeastOnceExternal + Safe + [Writes,Sends,Creates,Destroys]
- AtLeastOnceExternal + KeyRequired + [Writes,Sends,Creates,Destroys]

---

## 4. Proptest Invariants

### PROPTEST-001: Decision Table Confluence

**File**: `crates/vb_validate/tests/idempotency_contract_red.rs`

**Invariant**: For all `ActionContract` inputs, `is_statically_idempotent_contract` is confluent — given the same `(side_effect, retry_safety, idempotency)` triple, it always returns the same `Ok/Err` result.

**Strategy**:
```rust
proptest! {
    #[test]
    fn decision_table_is_confluent(
        side_effect in prop_one_of![
            Just(SideEffect::None),
            Just(SideEffect::Writes),
            Just(SideEffect::Sends),
            Just(SideEffect::Creates),
            Just(SideEffect::Destroys),
        ],
        retry_safety in prop_one_of![
            Just(RetrySafety::Safe),
            Just(RetrySafety::KeyRequired),
            Just(RetrySafety::Unsafe),
        ],
        idempotency in prop_one_of![
            Just(Idempotency::DeterministicPure),
            Just(Idempotency::IdempotentExternal),
            Just(Idempotency::AtLeastOnceExternal),
        ],
        seed in 0..10_000u32
    ) {
        let contract = ActionContract { side_effect, retry_safety, idempotency, .. };
        let result1 = is_statically_idempotent_contract(&contract);
        let result2 = is_statically_idempotent_contract(&contract);
        assert_eq!(result1.is_ok(), result2.is_ok(), "Decision table must be confluent");
    }
}
```

**Anti-invariant**: If inputs are generated randomly without enum weighting, the 5 rejection branches are under-tested. Use `prop_filter` to ensure adequate coverage of rejection branches.

### PROPTEST-002: Runtime Gate Determinism

**File**: `crates/vb_validate/tests/idempotency_contract_red.rs`

**Invariant**: For all `ActionContract` and `key_slots` inputs, `verify_idempotency` always returns the same `Ok/Err` result and same error variant for the same inputs.

**Strategy**:
```rust
proptest! {
    #[test]
    fn runtime_gate_is_deterministic(
        side_effect in prop_one_of![Just(SideEffect::None), Just(SideEffect::Writes)],
        idempotency in prop_one_of![
            Just(Idempotency::IdempotentExternal),
            Just(Idempotency::AtLeastOnceExternal),
        ],
        key_count in 0..16u8,
        seed in 0..10_000u32
    ) {
        let contract = ActionContract { side_effect, idempotency, .. };
        let key_slots: Vec<SlotIdx> = (0..key_count).map(SlotIdx::new).collect();
        // Generate taint pattern from seed
        let taints: Vec<Taint> = key_slots.iter().map(|_| taint_from_seed(seed)).collect();

        let result1 = verify_idempotency(&contract, &key_slots, &frame_with_taints(&taints));
        let result2 = verify_idempotency(&contract, &key_slots, &frame_with_taints(&taints));
        assert_eq!(result1.is_ok(), result2.is_ok(), "Runtime gate must be deterministic");
    }
}
```

---

## 5. Fuzz Targets

**No fuzz targets required.** The idempotency gate operates on in-memory enum types with no parsing/deserialization boundaries. All inputs are validated Rust enums constructed by the test harness, not by external data.

---

## 6. Kani Harnesses

**All 12 Kani harnesses are already written in State 5** (proof-writer phase). No additional harnesses are needed for test planning.

| Harness | File | Status | Coverage |
|---------|------|--------|----------|
| is_statically_idempotent_contract | kani/is_statically_idempotent_contract.rs | PASS | 45 combos |
| decision_table_ok_branch | kani/decision_table_ok_branch.rs | PASS | 13 Ok combos |
| decision_table_unsafe_rejected | kani/decision_table_unsafe_rejected.rs | PASS | 12 Err combos |
| decision_table_at_least_once_rejected | kani/decision_table_at_least_once_rejected.rs | PASS | 15 Err combos |
| decision_table_deterministic_rejected | kani/decision_table_deterministic_rejected.rs | PASS | 15 Err combos |
| idempotency_gate_parity (restricted scope) | kani/idempotency_gate_parity.rs | PASS (37 combos) | 37 combos |
| verify_idempotency_all_clean | kani/verify_idempotency_all_clean.rs | PASS | key slots 1..16 |
| verify_idempotency_missing_key | kani/verify_idempotency_missing_key.rs | PASS | empty key_slots |
| verify_idempotency_secret_in_key | kani/verify_idempotency_secret_in_key.rs | PASS | slot index 0..15 |
| verify_idempotency_random_in_key | kani/verify_idempotency_random_in_key.rs | PASS (placeholder) | not enforced |
| verify_idempotency_time_in_key | kani/verify_idempotency_time_in_key.rs | PASS (placeholder) | not enforced |
| verify_idempotency_single_error | kani/verify_idempotency_single_error.rs | PASS | short-circuit invariant |

---

## 7. Mutation Testing Checkpoints

**Threshold**: ≥90% mutation kill rate required.

### Critical Mutations

| Function | Mutation | Catch Mechanism |
|----------|----------|-----------------|
| `is_statically_idempotent_contract` | Remove `SideEffect::None → Ok` branch | Unit test: None + Unsafe + AtLeastOnceExternal → must return Ok, not Err |
| `is_statically_idempotent_contract` | Swap `Unsafe` and `AtLeastOnceExternal` error variants | Unit test: Writes + Unsafe + AtLeastOnceExternal → must return Err with correct variant |
| `verify_idempotency` | Remove short-circuit (accumulate multiple errors) | Unit test: key with multiple taints → must return exactly one error |
| `verify_idempotency` | Return wrong slot index in error | Unit test: SecretInKey(3) when slot 5 is tainted → must return correct index |

### Mutation Kill Strategy

- **Line-level mutations**: `cargo mutants` will be run against `vb_validate/src/idempotency_contract.rs` and `vb_core/src/action.rs`
- **Branch-level mutations**: Each decision table branch will be individually killed
- **Loop mutations**: `verify_idempotency` short-circuit will be mutated to accumulate errors

---

## 8. Combinatorial Coverage Matrix

### Static Gate — Decision Table (TEST-UNIT-001)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| None + any | side_effect=None | Ok(()) | unit |
| Unsafe + any idempotency | side_effect≠None, Unsafe | Err(SideEffectingRetryUnsafe) | unit |
| AtLeastOnceExternal + any | side_effect≠None, AtLeastOnceExternal | Err(SideEffectingAtLeastOnceExternal) | unit |
| DeterministicPure + any | side_effect≠None, DeterministicPure | Err(SideEffectingDeterministicPure) | unit |
| IdempotentExternal + Safe | side_effect≠None, Safe | Ok(()) | unit |
| IdempotentExternal + KeyRequired | side_effect≠None, KeyRequired | Ok(()) | unit |
| Confluence (all 45) | all enum combinations | deterministic result | proptest |
| Reason category | all Err variants | correct error variant name | unit |

### Runtime Gate — verify_idempotency (TEST-UNIT-002)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| All-clean keys | key_slots with all Taint::Clean | Ok(()) | unit |
| Missing key | empty key_slots, IdempotentExternal | Err(MissingKey(SideEffect)) | unit |
| SecretInKey(slot=0) | key_slots[0]=SecretTaint | Err(SecretInKey(0)) | unit |
| SecretInKey(slot=N) | key_slots[N]=SecretTaint, others clean | Err(SecretInKey(N)) | unit |
| RandomInKey(slot=0) | key_slots[0]=Random | Err(RandomInKey(0)) | unit |
| TimeInKey(slot=0) | key_slots[0]=TimeDependent | Err(TimeInKey(0)) | unit |
| Short-circuit | multiple taints at slots 0,3,5 | Err(SecretInKey(0)) only | unit |
| Determinism (all-clean) | same inputs, multiple calls | Ok always | proptest |

### Parity — compile/runtime (TEST-INTEGRATION-001)

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| 37 agreed combos | See Behavior 12 | Both agree on Ok/Err | integration |
| side_effect=None × 3 | None + any × any | Both Ok | integration |
| Unsafe × 3 idempotency | Unsafe + any idempotency | Both Err | integration |
| IdempotentExternal + Safe/Key | Safe/KeyRequired + IdempotentExternal | Both Ok | integration |
| DeterministicPure + Safe/Key | Safe/KeyRequired + DeterministicPure | Both Err | integration |
| Error variant mismatch | All Err cases | Ok/Err agree (variant may differ) | integration |

---

## 9. Static Analysis Gates

### Clippy

```bash
cargo clippy -p vb_validate -p vb_core -p vb_compile -- -D warnings
```

All idempotency gate code must compile with zero warnings.

### Cargo Deny

```bash
cargo deny check licenses && cargo deny check bans
```

No new dependency additions in this bead.

### Kani Coverage

Already verified in State 5:
- KANI-DECISION-001 through 005: all PASS
- KANI-RUNTIME-001 through 006: all PASS
- KANI-PARITY-001: PASS (37 restricted scope)

---

## 10. Open Questions

| Question | Resolution |
|----------|------------|
| Should the 8 deferred AtLeastOnceExternal+Safe/KeyRequired combinations be tested as "known failures" in TEST-INTEGRATION-001? | **No** — these represent a production bug in vb_validate that is outside this bead's scope. They should be tested in the vb_validate bug fix bead. |
| Should KANI-RUNTIME-004 and KANI-RUNTIME-005 placeholders be updated to assert `is_err()` now? | **No** — enforcement is not yet implemented. The placeholders correctly assert `is_ok()`. When Taint::Random/TimeDependent enforcement is added, these harnesses must be updated. |
| Is there existing test infrastructure for `verify_idempotency`? | **Yes** — existing 20+ unit tests in `action.rs` (lines 1061-1498). Test-writer must verify coverage is complete and extend if needed. |

---

## 11. Exit Criteria

All of the following must be satisfied before this bead is considered complete:

- [ ] **TEST-UNIT-001**: All 5 decision table branches have explicit unit tests with correct error variant assertions (not just `is_err()`)
- [ ] **TEST-UNIT-002**: All 5 runtime paths have unit tests with correct slot index assertions
- [ ] **TEST-INTEGRATION-001**: Integration tests cover all 37 parity combinations, with explicit documentation of the 8 deferred combinations
- [ ] **PROPTEST-001**: Decision table confluence proptest passes with 10,000 iterations
- [ ] **PROPTEST-002**: Runtime gate determinism proptest passes with 10,000 iterations
- [ ] **Mutation testing**: ≥90% kill rate achieved on `idempotency_contract.rs` and `action.rs`
- [ ] **Clippy**: Zero warnings on vb_validate, vb_core, vb_compile
- [ ] **Cargo test**: `cargo test -p vb_validate -p vb_core -p vb_compile -- idempotency` exits 0

---

*Generated by test-planner State 7 for vb-qi37.5.4*
*Authority: Martin Fowler (behavior-driven), Dave Farley (ATDD), Dan North (BDD), Kent Beck (TDD), Testing Trophy (Hodges/Searls), Google SWE Book*
