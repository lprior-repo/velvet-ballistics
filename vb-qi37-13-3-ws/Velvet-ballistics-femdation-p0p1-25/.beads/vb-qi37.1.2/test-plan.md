# Test Plan: vb-qi37.1.2 — Journal Slot Writes with Taint Propagation

## Scope

Functions under test:
- `write_slot_with_taint` — `crates/vb_core/src/frame.rs:229`
- `recovered_slot_taint` — `crates/vb_storage/src/recovery/replay/summary.rs:428`
- `legacy_slot_taint` — `crates/vb_storage/src/recovery/replay/summary.rs:435`
- `encoded_slot_taint_extra` — `crates/vb_runtime/src/journal/chunk_002.rs:192`
- `join_taint` — `crates/vb_core/src/value.rs:24`

## Test Artifacts to Produce

| File | Purpose |
|------|---------|
| `crates/vb_core/src/frame/tests/taint_tests.rs` | Unit + proptest for `write_slot_with_taint` |
| `crates/vb_storage/src/recovery/replay/tests/taint_recovery_tests.rs` | Unit + proptest for `recovered_slot_taint`, `legacy_slot_taint` |
| `crates/vb_runtime/src/journal/tests/taint_chunk_tests.rs` | Unit + proptest for `encoded_slot_taint_extra` |

---

## Unit Tests

### `write_slot_with_taint` (frame.rs:229)

| ID | Scenario | Expected |
|----|----------|----------|
| UT-wst-001 | `write_slot_with_taint(idx, val, taint)` for in-bounds `idx` | `Ok(())`, `slots[idx] == Some(val)`, `taint[idx] == taint` |
| UT-wst-002 | `write_slot_with_taint` with out-of-bounds `slot` | `Err(CoreError::SlotOutOfBounds { slot })`, `slots` and `taint` unchanged |
| UT-wst-003 | Write slot 0, then slot 1 — both read back correct | Each slot reflects its own value/taint |
| UT-wst-004 | Repeated write to same slot — later write wins | `slots[i]` and `taint[i]` match most recent call |
| UT-wst-005 | Write all slots sequentially in a frame with `slot_count=16` | All 16 slots readable after their writes |

### `recovered_slot_taint` (summary.rs:428)

| ID | Scenario | Expected |
|----|----------|----------|
| UT-rst-001 | `extra = Some(bytes)` where bytes = postcard-encoded `Taint::Secret` | Returns `Taint::Secret` |
| UT-rst-002 | `extra = Some(bytes)` where bytes = postcard-encoded `Taint::Clean` | Returns `Taint::Clean` |
| UT-rst-003 | `extra = Some(invalid_bytes)` — postcard decode fails | Falls back to `legacy_slot_taint(value)` |
| UT-rst-004 | `extra = None` with `SlotValue::Bool(false)` | Returns `Taint::Clean` |
| UT-rst-005 | `extra = None` with `SlotValue::Bool(true)` | Returns `Taint::DerivedFromSecret` |
| UT-rst-006 | `extra = None` with `SlotValue::Null` | Returns `Taint::DerivedFromSecret` |
| UT-rst-007 | `extra = None` with `SlotValue::I64(42)` | Returns `Taint::Secret` |
| UT-rst-008 | `extra = None` with `SlotValue::Symbol(_)` | Returns `Taint::Secret` |
| UT-rst-009 | `extra = None` with `SlotValue::List(_)` | Returns `Taint::Secret` |
| UT-rst-010 | `extra = None` with `SlotValue::Object(_)` | Returns `Taint::Secret` |
| UT-rst-011 | `extra = None` with `SlotValue::Blob(_)` | Returns `Taint::Secret` |

### `legacy_slot_taint` (summary.rs:435)

| ID | SlotValue | Expected Taint |
|----|-----------|----------------|
| UT-legacy-001 | `Bool(false)` | `Clean` |
| UT-legacy-002 | `Bool(true)` | `DerivedFromSecret` |
| UT-legacy-003 | `Null` | `DerivedFromSecret` |
| UT-legacy-004 | `I64(0)`, `I64(-1)`, `I64(i64::MAX)` | `Secret` |
| UT-legacy-005 | `F64(FiniteF64)` | `Secret` |
| UT-legacy-006 | `Symbol`, `List`, `Object`, `Blob` handles | `Secret` |

### `encoded_slot_taint_extra` (chunk_002.rs:192)

| ID | Scenario | Expected |
|----|----------|----------|
| UT-est-001 | `extra = Some(existing_bytes)`, any `taint` | Returns `Some(existing_bytes)` unchanged |
| UT-est-002 | `extra = None`, `taint = Clean` | Returns `Some(bytes)` where postcard decode yields `Taint::Clean` |
| UT-est-003 | `extra = None`, `taint = DerivedFromSecret` | Returns `Some(bytes)` where postcard decode yields `Taint::DerivedFromSecret` |
| UT-est-004 | `extra = None`, `taint = Secret` | Returns `Some(bytes)` where postcard decode yields `Taint::Secret` |
| UT-est-005 | `extra = None`, postcard encode fails | Returns `None` |

### `join_taint` (value.rs:24)

Lattice properties validated by existing `kani_taint.rs` (PO-011) and `integration_taint_propagation.rs` B-001–B-007. Unit tests here provide exhaustiveness:

| ID | Property |
|----|----------|
| UT-join-001 | `join_taint(Clean, x) == x` for all x |
| UT-join-002 | `join_taint(Secret, x) == Secret` for all x |
| UT-join-003 | `join_taint(x, x) == x` (idempotent) |
| UT-join-004 | `join_taint(a, b) == join_taint(b, a)` (commutative) |

---

## Proptest Strategies

### `write_slot_with_taint` proptest (`crates/vb_core/src/frame/tests/taint_tests.rs`)

```rust
proptest! {
    #[test]
    fn write_slot_with_taint_roundtrip_all_variants(
        slot_idx in 0u16..256u16,
        val in slot_value_strategy(),
        taint in taint_strategy(),
    )
}
```

- **Bounds**: `slot_idx` in `[0, 256)` — frame created with `slot_count >= 256`
- **SlotValue strategy**: covers all 9 variants (Null, Bool, I64, F64, Symbol, List, Object, Blob)
- **Taint strategy**: `prop_oneof![Just(Taint::Clean), Just(Taint::DerivedFromSecret), Just(Taint::Secret)]`
- **Assertions**: read back `slots[idx]` and `taint[idx]` match written values

### `recovered_slot_taint` proptest (`crates/vb_storage/src/recovery/replay/tests/taint_recovery_tests.rs`)

```rust
proptest! {
    #[test]
    fn recovered_slot_taint_decodes_valid_extra(
        val in slot_value_strategy(),
        taint in taint_strategy(),
    ) {
        // Encode taint, call recovered_slot_taint, assert roundtrip
    }

    #[test]
    fn recovered_slot_taint_falls_back_to_legacy_on_decode_failure(
        val in slot_value_strategy(),
    ) {
        // extra = Some(invalid_bytes), assert result == legacy_slot_taint(val)
    }

    #[test]
    fn recovered_slot_taint_legacy_when_extra_is_none(
        val in slot_value_strategy(),
    ) {
        // extra = None, assert result == legacy_slot_taint(val)
    }
}
```

### `encoded_slot_taint_extra` proptest (`crates/vb_runtime/src/journal/tests/taint_chunk_tests.rs`)

```rust
proptest! {
    #[test]
    fn encoded_slot_taint_extra_roundtrip(taint in taint_strategy()) {
        // encoded_slot_taint_extra(taint, None) -> Some(bytes)
        // postcard::from_bytes::<Taint>(&bytes) == Ok(taint)
    }

    #[test]
    fn encoded_slot_taint_extra_preserves_existing(existing in vec(any::<u8>(), 1..64)) {
        // encoded_slot_taint_extra(Taint::Secret, Some(existing.clone())) == Some(existing)
    }
}
```

---

## BDD Scenarios

### Feature: Taint Propagation on Journal Slot Write

```
Scenario: Successful slot write with Clean taint
  Given a RunFrame with slot_count = 4
  When I call write_slot_with_taint(SlotIdx(0), SlotValue::I64(42), Taint::Clean)
  Then the call returns Ok(())
  And slots[0] == Some(SlotValue::I64(42))
  And taint[0] == Taint::Clean
  And slots[1..4] remain None
  And taint[1..4] remain Taint::Clean

Scenario: Successful slot write with Secret taint
  Given a RunFrame with slot_count = 4
  When I call write_slot_with_taint(SlotIdx(2), SlotValue::Bool(true), Taint::Secret)
  Then the call returns Ok(())
  And slots[2] == Some(SlotValue::Bool(true))
  And taint[2] == Taint::Secret

Scenario: Out-of-bounds slot write returns error without state change
  Given a RunFrame with slot_count = 4
  And slots are initialized to None, taint to Clean
  When I call write_slot_with_taint(SlotIdx(99), SlotValue::Null, Taint::Clean)
  Then the call returns Err(CoreError::SlotOutOfBounds { slot: SlotIdx(99) })
  And all slots remain None
  And all taint entries remain Taint::Clean

Scenario: Slot overwrite takes last write
  Given a RunFrame with slot_count = 4
  When I call write_slot_with_taint(SlotIdx(1), SlotValue::I64(1), Taint::Clean)
  And I call write_slot_with_taint(SlotIdx(1), SlotValue::I64(2), Taint::Secret)
  Then slots[1] == Some(SlotValue::I64(2))
  And taint[1] == Taint::Secret

Scenario: Recovered taint from valid postcard bytes
  Given extra = Some(postcard::to_allocvec(&Taint::Secret))
  And value = SlotValue::I64(100)
  When I call recovered_slot_taint(value, &extra)
  Then the result == Taint::Secret

Scenario: Recovered taint falls back to legacy when extra is None
  Given extra = None
  And value = SlotValue::Bool(true)
  When I call recovered_slot_taint(value, &extra)
  Then the result == Taint::DerivedFromSecret

Scenario: Recovered taint falls back to legacy when postcard decode fails
  Given extra = Some(vec![0xFF, 0xFE])
  And value = SlotValue::Null
  When I call recovered_slot_taint(value, &extra)
  Then the result == Taint::DerivedFromSecret  // legacy_slot_taint(Null)

Scenario: Encoded extra preserves existing bytes
  Given existing = vec![0xDE, 0xAD, 0xBE, 0xEF]
  When I call encoded_slot_taint_extra(Taint::Secret, Some(existing.clone()))
  Then the result == Some(existing)

Scenario: Encoded extra encodes taint when None
  When I call encoded_slot_taint_extra(Taint::DerivedFromSecret, None)
  Then the result == Some(bytes)
  And postcard::from_bytes::<Taint>(&bytes) == Ok(Taint::DerivedFromSecret)

Scenario: join_taint lattice — Secret absorbs all
  Given a = Taint::Secret and b in {Clean, DerivedFromSecret, Secret}
  When I call join_taint(a, b)
  Then the result == Taint::Secret

Scenario: join_taint lattice — Clean is identity
  Given a = Taint::Clean and b in {Clean, DerivedFromSecret, Secret}
  When I call join_taint(a, b)
  Then the result == b
```

---

## Existing Test Coverage (Do Not Duplicate)

| File | Coverage |
|------|----------|
| `crates/vb_core/src/kani_taint.rs` | Kani proofs: `join_taint_ge_first_arg`, `join_taint_ge_second_arg`, `join_taint_idempotent`, `join_taint_commutative` — satisfies PO-011 |
| `crates/vb_core/src/engine/tests/integration_taint_propagation.rs` | B-001–B-007: `join_taint` lattice algebra; `SlotValue` roundtrip; `Taint` reflexivity |
| `crates/vb_core/src/value.rs:proptests` | `slot_value_postcard_roundtrips_for_all_variants`, `taint_ordering_is_reflexive`, `finite_f64_rejects_nan` |
| `crates/vb_runtime/src/journal/tests/chunk_002.rs` | `storage_runtime_journal_maps_action_wait_and_ask_events` |

---

## Verification Layer Assignments

| PO | Verifier | Test Coverage |
|----|----------|---------------|
| PO-001 | Kani | Unit tests UT-wst-001, UT-wst-002 + proptest `write_slot_with_taint_roundtrip_all_variants` |
| PO-002 | Kani | UT-wst-002 (no partial state on bounds error) |
| PO-003 | Verus | Not unit-testable (Verus-owned invariant) |
| PO-004 | proptest | `recovered_slot_taint_decodes_valid_extra` |
| PO-005 | proptest | `recovered_slot_taint_legacy_when_extra_is_none` |
| PO-006 | Verus | Not unit-testable (Verus-owned purity proof) |
| PO-007 | Verus | Not unit-testable (Verus-owned preservation proof) |
| PO-008 | proptest | `encoded_slot_taint_extra_roundtrip` |
| PO-009 | Kani | `encoded_slot_taint_extra_preserves_existing` + roundtrip proptest |
| PO-010 | TLA+ | Journal atomicity model — see `specs/journal_atomicity.tla` |
| PO-011 | proptest | Unit tests UT-join-001–UT-join-004 + existing `kani_taint.rs` proofs |

---

## Test Execution Order

1. `cargo test -p vb_core frame::tests::taint_tests` — frame unit + proptest
2. `cargo test -p vb_storage recovery::replay::tests::taint_recovery_tests` — recovery unit + proptest
3. `cargo test -p vb_runtime journal::tests::taint_chunk_tests` — journal unit + proptest
4. `cargo test -p vb_core integration_taint_propagation` — full BDD suite (B-001–B-007)
5. `cargo kani --harness write_slot_with_taint` — PO-001/PO-002
6. `cargo kani --harness encoded_slot_taint_extra_kani` — PO-009
7. `cargo verify --spec write_slot_with_taint_spec` — PO-003 (Verus)
8. `cargo verify --spec recovered_slot_taint_spec` — PO-006 (Verus)
9. `cargo verify --spec encoded_slot_taint_extra_spec` — PO-007 (Verus)
10. TLA+ model check via `java -jar tla2tools.jar specs/journal_atomicity.tla` — PO-010

---

## Pass Criteria

- All 11 unit test scenarios pass
- All proptest runs (1000 iterations each) pass with no counterexamples
- BDD scenarios in `integration_taint_propagation.rs` remain green
- Kani harnesses report no failures
- TLA+ TLC reports no invariant violations
- `cargo clippy -p vb_core -p vb_storage -p vb_runtime` reports zero warnings