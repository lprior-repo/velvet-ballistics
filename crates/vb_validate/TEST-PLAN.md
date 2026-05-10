# vb_validate Test Plan

## Crate: vb_validate

### Current State
- `cargo test -p vb_validate`: **973 passed**, 2 warnings
- Clippy: **0 errors**, 2 warnings
- Coverage: **90.29%**
- VERDICT: REJECTED (source lint violations)

---

## Section 1 — Behavior Inventory

### Core Validation Functions (`src/type_taint.rs`)

| # | Subject | Action | Outcome when Condition |
|---|---------|--------|----------------------|
| 1 | `validate_taint` | rejects secret result leak | `Err(ValidationError::SecretResultLeak)` when finish references `$secrets.*` |
| 2 | `validate_taint` | rejects secret via slot | `Err(ValidationError::SecretResultLeak)` when slot contains secret taint |
| 3 | `validate_taint` | rejects secret input in finish | `Err(ValidationError::SecretResultLeak)` when input marked `is_secret=true` |
| 4 | `validate_taint` | accepts clean finish | `Ok(())` when finish uses clean values only |
| 5 | `validate_taint` | accepts clean input finish | `Ok(())` when input `is_secret=false` |
| 6 | `validate_taint` | taint propagates through composite | taint merges across composite elements |
| 7 | `validate_taint` | taint propagates through slot chain | N-hop relay carries taint |
| 8 | `validate_taint` | choose condition taint does not propagate | downstream steps unaffected |
| 9 | `validate_types` | rejects non-boolean choose | `Err(ValidationError::TypeMismatch { expected: "boolean" })` |
| 10 | `validate_types` | accepts boolean choose | `Ok(())` |
| 11 | `validate_types` | accepts Any type choose | `Ok(())` |
| 12 | `validate_resource_limits` | rejects exceeded steps | `Err(ValidationError::LimitExceeded { resource: "max_steps" })` |
| 13 | `validate_resource_limits` | rejects declared > hard | `Err(ValidationError::LimitExceeded { resource: ... })` |
| 14 | `validate_resource_limits` | rejects zero limit | `Err(ValidationError::LimitRequired { resource: ... })` |
| 15 | `validate_resource_limits` | accepts within bounds | `Ok(())` |

### Idempotency Contract Functions (`src/idempotency_contract.rs`)

| # | Subject | Action | Outcome when Condition |
|---|---------|--------|----------------------|
| 16 | `validate_action_idempotency_contract` | returns unit for pure safe | `Ok(())` when `SideEffect::None + DeterministicPure + Safe` |
| 17 | `validate_action_idempotency_contract` | returns unit for side-effecting idempotent | `Ok(())` when `SideEffect::Writes + IdempotentExternal + Safe` |
| 18 | `validate_action_idempotency_contract` | returns `RetryUnsafe` violation | `Err(SideEffectingRetryUnsafe { ... })` when unsafe combo |
| 19 | `validate_action_idempotency_contract` | returns `AtLeastOnce` violation | `Err(SideEffectingAtLeastOnceExternal { ... })` |
| 20 | `validate_action_idempotency_contract` | returns `DeterministicPure` violation | `Err(SideEffectingDeterministicPure { ... })` |
| 21 | `collect_idempotency_contract_violations` | returns unit for empty slice | `Ok(())` |
| 22 | `collect_idempotency_contract_violations` | returns all violations in input order | `Err(IdempotencyContractErrors(Box::from([...])))` |
| 23 | `is_statically_idempotent_contract` | returns unit for pure contracts | `Ok(())` for all 9 pure combos |
| 24 | `verify_idempotency` | returns `MissingKey` | when key-required action has empty key slots |
| 25 | `verify_idempotency` | returns `SecretInKey` | when key slot taint is `Secret` or `DerivedFromSecret` |
| 26 | `validate_idempotency_key_ingredients` | returns unit for clean key | `Ok(())` |
| 27 | `validate_workflow_idempotency_contracts` | returns unit when no DO nodes | `Ok(())` |
| 28 | `validate_workflow_idempotency_contracts` | returns `ActionContractMissing` | when DO node has no matching contract |
| 29 | `validate_workflow_idempotency_contracts` | returns `ActionContractOrphan` | when registry contract has no DO node |

### Error Enum Coverage

Every `ValidationError` variant must have a test:
- `SecretResultLeak` — tested (behaviors 1-3, 6-8)
- `TypeMismatch { expected, found }` — tested (behavior 9)
- `LimitExceeded { resource }` — tested (behaviors 12-13)
- `LimitRequired { resource }` — tested (behavior 14)

Every `IdempotencyViolation` variant must have a test:
- `MissingKey(SideEffect)` — tested (behavior 24)
- `SecretInKey(u8)` — tested (behavior 25)

---

## Section 2 — Trophy Allocation

| Layer | Target % | Scope |
|-------|----------|-------|
| **Integration** (`tests/`) | 60% | Full workflow validation, contract validation, workflow idempotency |
| **Unit** (`#[cfg(test)]` in `src/`) | 30% | Calc-layer: `validate_taint`, `validate_types`, `validate_resource_limits`, taint merge, value fact constructors |
| **E2E** | 5% | `capability_contract_schema.rs`, `idempotency_contract_red.rs` |
| **Static** (clippy, types) | 5% | Already enforced: 0 errors, 2 warnings |

Current: 973 tests pass. No new tests needed — fixes are lint/style.

---

## Section 3 — BDD Scenarios

### Fix 1: Remove unused `ValidationResult` import
**File**: `src/type_taint_tests.rs`, line 8
```rust
// BEFORE
use crate::{ValidationError, ValidationResult};

// AFTER
use crate::ValidationError;
```

### Fix 2: Replace 52x `unwrap()` with `expect()` in Result-returning test helpers
**File**: `src/type_taint_tests.rs`

All helper functions return `Result<(), String>` and are called from `#[test]` wrappers. The pattern:
```rust
// BEFORE
#[test]
fn run_taint_propagates_through_arithmetic_style_composite() {
    taint_propagates_through_arithmetic_style_composite().unwrap()
}

// AFTER — change helper to #[track_caller] and use expect
#[track_caller]
fn taint_propagates_through_arithmetic_style_composite() -> Result<(), String> {
    // ... existing body ...
    Ok(())
}

#[test]
fn run_taint_propagates_through_arithmetic_style_composite() {
    taint_propagates_through_arithmetic_style_composite()
        .expect("taint propagation must not fail for arithmetic composite")
}
```

Affected test helper functions (all return `Result<(), String>` and are wrapped with `.unwrap()`):
1. `taint_propagates_through_arithmetic_style_composite` → line 1555
2. `taint_propagates_through_comparison_style_composite` → line 1560
3. `taint_propagates_through_logic_style_composite` → line 1565
4. `clean_composite_stays_clean` → line 1570
5. `secret_origin_propagates_through_all_downstream_paths` → line 1643
6. `secret_origin_relay_slot_is_tainted` → line 1648
7. `secret_origin_composite_slot_is_tainted` → line 1653
8. `slot_to_slot_single_relay_propagates_taint` → line 1724
9. `slot_to_slot_clean_relay_stays_clean` → line 1729
10. `slot_to_slot_branching_relays_both_tainted` → line 1734
11. `slot_to_slot_two_hop_relay_carries_taint` → line 1739
12. `conditional_taint_choose_does_not_taint_downstream` → line 1832
13. `conditional_taint_finish_after_choose_reads_tainted` → line 1837
14. `conditional_taint_multiple_chooses_interleaved` → line 1842
15. `conditional_taint_clean_boolean_choose_passes_both_validators` → line 1847
16. `accessor_secret_input_field_carries_taint` → line 1977
17. `accessor_clean_input_nested_field_stays_clean` → line 1982
18. `accessor_secret_field_via_secrets_namespace` → line 1987
19. `accessor_var_field_is_clean` → line 1992
20. `accessor_secret_in_composite_propagates_taint` → line 1997
21. `accessor_composite_of_clean_accessors_stays_clean` → line 2002
22. `fully_clean_workflow_passes_both_validators` → line 2093
23. `clean_path_through_relay_chain` → line 2098
24. `clean_composite_in_finish_passes` → line 2103
25. `clean_finish_with_secrets_in_other_paths` → line 2108
26. `taint_merge_secret_plus_secret` → line 2229
27. `taint_merge_clean_plus_clean` → line 2234
28. `taint_merge_secret_plus_clean_directions` → line 2239
29. `taint_merge_composite_of_two_secret_sources` → line 2244
30. `taint_merge_secret_dominates_over_clean` → line 2249
31. `taint_merge_three_distinct_secret_sources` → line 2254
32. `boundary_empty_workflow_passes` → line 2462
33. `boundary_no_secrets_at_all` → line 2467
34. `boundary_all_slots_tainted` → line 2472
35. `boundary_all_slots_tainted_finish_uses_literal` → line 2477
36. `boundary_forward_slot_reference_is_clean` → line 2482
37. `boundary_self_referential_slot_is_clean` → line 2487
38. `boundary_cycle_like_pattern_all_clean` → line 2492
39. `boundary_bare_finish_literal` → line 2497
40. `boundary_slot_overwrite_second_write_clean` → line 2502
41. `boundary_slot_index_overwritten_to_clean` → line 2507
42. `boundary_long_clean_chain_passes` → line 2512
43. `type_check_object_in_choose_rejected` → line 2615
44. `type_check_list_in_choose_rejected` → line 2620
45. `type_check_number_in_choose_rejected` → line 2625
46. `type_check_any_from_unresolved_ref_accepted` → line 2630
47. `type_check_save_composite_passes` → line 2635
48. `type_check_multiple_finishes_first_tainted` → line 2640
49. `resource_limits_zero_constant_pool_rejected` → line 2744
50. `resource_limits_collect_items_exceeding_hard_rejected` → line 2749
51. `resource_limits_retry_attempts_exceeding_hard_rejected` → line 2754
52. `resource_limits_zero_queue_depth_rejected` → line 2759

**Total: 52 `.unwrap()` calls** — all in `src/type_taint_tests.rs`.

### Fix 3: Fix `#[cfg(kani)]` in `src/gate_08_accessor.rs`
**File**: `src/gate_08_accessor.rs`, line 505

```rust
// BEFORE
#[cfg(kani)]

// AFTER — use #[cfg(test)] for test-only compilation, or remove entirely
// If kani proofs exist here, gate with feature:
// #[cfg(all(test, feature = "kani"))]
#[cfg(test)]
```

### Fix 4: Verify idempotency_contract_red.rs has no `panic!` in Result tests
**File**: `tests/idempotency_contract_red.rs`

Re-check required: The 8 tests returning `Result` (lines 550-613) currently use `?` operator on `RunFrame::new()?` and `frame.write_slot_with_taint(...)`. No `panic!` found. Verify if any `panic!` was introduced post-Verdict.

---

## Section 4 — Proptest Invariants

The `idempotency_contract_red.rs` already contains 2 proptest harnesses:

### Invariant 1: Pure action acceptance
```rust
proptest! {
    #[test]
    fn proptest_pure_action_acceptance_holds_for_representative_action_ids(
        action_raw in 0u16..128u16
    ) {
        prop_assert_eq!(
            is_statically_idempotent_contract(&contract(
                ActionId::new(action_raw),
                SideEffect::None,
                Idempotency::AtLeastOnceExternal,
                RetrySafety::Unsafe,
            )),
            Ok(())
        );
    }
}
```
**Property**: For any action ID, pure contracts (SideEffect::None) always return `Ok(())`.
**Strategy**: `0u16..128u16` — covers all small action IDs.

### Invariant 2: Retry unsafe violation reports original action
```rust
proptest! {
    #[test]
    fn proptest_retry_unsafe_side_effecting_contracts_report_original_action(
        action_raw in 0u16..128u16
    ) {
        let action_id = ActionId::new(action_raw);
        prop_assert_eq!(
            is_statically_idempotent_contract(&contract(
                action_id,
                SideEffect::Destroys,
                Idempotency::IdempotentExternal,
                RetrySafety::Unsafe,
            )),
            Err(retry_unsafe_violation(action_id))
        );
    }
}
```
**Property**: The violation always echoes back the original action ID.
**Strategy**: `0u16..128u16` — matches invariant 1.

### Missing Propertes to Add

For `type_taint.rs` pure functions, add:

```rust
// Taint merge is commutative — always holds
proptest! {
    #[test]
    fn taint_merge_commutative(a: bool, b: bool) {
        let t1 = if a { Taint::Secret } else { Taint::Clean };
        let t2 = if b { Taint::Secret } else { Taint::Clean };
        prop_assert_eq!(t1.merge(t2), t2.merge(t1));
    }
}

// Taint merge is associative — always holds
proptest! {
    #[test]
    fn taint_merge_associative(a: bool, b: bool, c: bool) {
        let t1 = if a { Taint::Secret } else { Taint::Clean };
        let t2 = if b { Taint::Secret } else { Taint::Clean };
        let t3 = if c { Taint::Secret } else { Taint::Clean };
        prop_assert_eq!(t1.merge(t2).merge(t3), t1.merge(t2.merge(t3)));
    }
}

// Taint merge idempotent — always holds
proptest! {
    #[test]
    fn taint_merge_idempotent(a: bool) {
        let t = if a { Taint::Secret } else { Taint::Clean };
        prop_assert_eq!(t.merge(t), t);
    }
}
```

---

## Section 5 — Fuzz Targets

No new fuzz targets needed. The crate:
- Has no parsers or deserializers accepting untrusted input
- Uses validated internal types (`ActionId`, `SlotIdx`, `StepIdx` via `new()` constructors)
- `TypedValue` construction is via enums with known variants

Existing integration tests cover the input space combinatorially.

---

## Section 6 — Kani Harnesses

No Kani harnesses in `vb_validate`. The `#[cfg(kani)]` at line 505 of `gate_08_accessor.rs` appears to be misapplied — it should likely be `#[cfg(test)]` or gated behind a feature.

**Recommended fix**: Remove or correctly gate the `#[cfg(kani)]` block. If formal verification is needed, add a separate `proofs/` directory with Kani harnesses.

---

## Section 7 — Mutation Testing Checkpoints

Target: **≥90% kill rate**

All 52 `unwrap()` → `expect()` changes maintain identical test semantics, so mutation kill rate is unaffected.

The existing test suite already achieves high coverage. Key mutations that would be caught:

| Mutation | Test that catches it |
|----------|---------------------|
| `SecretResultLeak` not returned for secret finish | `rejects_secret_finish_direct`, `rejects_secret_finish_via_slot` |
| Taint not propagating through composite | `taint_propagates_through_arithmetic_style_composite` |
| `TypeMismatch` wrong `expected` string | `validate_types_returns_type_mismatch_for_wrong_type` |
| `LimitExceeded` wrong `resource` field | `validate_resource_limits_rejects_too_many_steps` |
| Commutativity of taint merge broken | `blackhat_taint_merge_commutative` |

---

## Section 8 — Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| validate_taint: clean literal finish | `TypedValue::Literal(ValueType::Number)` | `Ok(())` | unit |
| validate_taint: secret direct reference | `$secrets.api_key` | `Err(SecretResultLeak)` | unit |
| validate_taint: tainted slot finish | slot holding secret | `Err(SecretResultLeak)` | unit |
| validate_taint: clean input reference | `$input.user` (not secret) | `Ok(())` | unit |
| validate_taint: secret input reference | `$input.password` (secret) | `Err(SecretResultLeak)` | unit |
| validate_taint: composite with secret | `[..., $secrets.x, ...]` | `Err(SecretResultLeak)` | unit |
| validate_taint: 3-hop relay | slot → slot → slot | `Err(SecretResultLeak)` | unit |
| validate_taint: choose taint not propagate | tainted condition, clean finish | `Ok(())` | unit |
| validate_types: boolean choose | `TypedValue::Literal(Boolean)` | `Ok(())` | unit |
| validate_types: text choose | `TypedValue::Literal(Text)` | `Err(TypeMismatch)` | unit |
| validate_types: null choose | `TypedValue::Literal(Null)` | `Err(TypeMismatch)` | unit |
| validate_types: Any choose | `TypedValue::Literal(Any)` | `Ok(())` | unit |
| validate_resource_limits: within bounds | all defaults | `Ok(())` | unit |
| validate_resource_limits: steps exceeded | `max_steps: 0`, 1 step | `Err(LimitExceeded)` | unit |
| validate_resource_limits: declared > hard | `decl: 100, hard: 50` | `Err(LimitExceeded)` | unit |
| validate_resource_limits: zero limit | `max_fanout: 0` | `Err(LimitRequired)` | unit |
| validate_idempotency: pure safe contract | None/DeterministicPure/Safe | `Ok(())` | unit |
| validate_idempotency: retry unsafe | Destroys/IdempotentExternal/Unsafe | `Err(SideEffectingRetryUnsafe)` | unit |
| validate_idempotency: at-least-once | Creates/AtLeastOnceExternal/Safe | `Err(SideEffectingAtLeastOnceExternal)` | unit |
| validate_idempotency: deterministic pure side-effecting | Writes/DeterministicPure/Safe | `Err(SideEffectingDeterministicPure)` | unit |
| verify_idempotency: missing key | empty key slots, KeyRequired | `Err(MissingKey)` | integration |
| verify_idempotency: secret in key | `Taint::Secret` key slot | `Err(SecretInKey)` | integration |
| workflow validation: no DO nodes | empty workflow | `Ok(())` | integration |
| workflow validation: contract missing | DO node, no registry | `Err(ActionContractMissing)` | integration |
| workflow validation: orphan contract | registry entry, no DO node | `Err(ActionContractOrphan)` | integration |

---

## Fix Checklist

- [ ] **F1**: Remove `ValidationResult` from import in `src/type_taint_tests.rs:8`
- [ ] **F2**: Replace all 52 `.unwrap()` with `.expect(...)` in `src/type_taint_tests.rs`
- [ ] **F3**: Fix `#[cfg(kani)]` → `#[cfg(test)]` or remove in `src/gate_08_accessor.rs:505`
- [ ] **F4**: Confirm no `panic!` in `tests/idempotency_contract_red.rs` Result-returning functions
- [ ] **F5**: Run `cargo test -p vb_validate` — must show 0 errors
- [ ] **F6**: Run `cargo clippy -p vb_validate` — must show 0 warnings
