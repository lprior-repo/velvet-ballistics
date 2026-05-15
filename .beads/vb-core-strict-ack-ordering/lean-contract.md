# Theorem Kernel Projection — vb-core-strict-ack-ordering

## Context

- **bead_id**: vb-core-strict-ack-ordering
- **bead_title**: runtime/storage: Prove strict persistence before acknowledgement ordering
- **phase**: State 3
- **updated_at**: 2026-05-15T00:00:00Z
- **attempt**: 1

## Boundary

| Layer | Owner | Rationale |
|-------|-------|-----------|
| TLA+ temporal model | TLA+/TLC | Journal barrier state machine, EventSeq ordering, queued flush concurrency |
| **Verus** (Rust-local core) | **This bead — State 3** | `verify_ack_after_persist` purity, `EventSeq` invariants, `append_strict`/`append_journaled` postconditions, `DurabilityRow` static table properties, `AckPoint` zombie-variant unreachability |
| Lean/Aeneas/Hax | **Not required** | No algebraic theorem kernels, parser grammars, arithmetic lattice proofs, or refinement extractions exist in this bead's scope that exceed Verus expressiveness |
| Rust shell (I/O, async, storage) | Kani + Loom + integration | `persist_strict` calls Fjall; bounded by Kani harness + Loom concurrency tests |
| External systems | N/A | No FFI, network, or wall-clock time dependencies in core ordering contract |

---

## Verus-Owned Obligations

All Rust-local pure/deterministic core logic for this bead is owned by **Verus**. No Lean theorem-proving is required.

### VERUS-DM-001: `verify_ack_after_persist` Purity and Correctness

**Contract clause**: `POST-001`, `POST-002`, `INV-002`

**Target**: `crates/vb_runtime/src/durability_matrix.rs::verify_ack_after_persist`

**Spec function**:
```verus
spec fn ack_point_is_after_append(row: &DurabilityRow) -> bool {
    row.ack_point == AckPoint::AfterJournalAppend
}

spec fn verify_ack_after_persist_spec(matrix: &[DurabilityRow]) -> Result<(), DurabilityError> {
    if matrix.iter().all(|row| ack_point_is_after_append(row)) {
        Ok(())
    } else {
        Err(DurabilityError::AckBeforePersist {
            primitive: "?", handler: "?"
        })
    }
}
```

**Proof obligation**: Prove `verify_ack_after_persist() == verify_ack_after_persist_spec(DURABILITY_MATRIX)` — i.e., the runtime function refines the spec.

**Trusted boundary**: `DURABILITY_MATRIX` is a static `&'static [DurabilityRow]` initialized at compile time.

**Shell exclusions**: No I/O, async, storage, or wall-clock time in this pure function.

---

### VERUS-DM-002: `DURABILITY_MATRIX` Completeness

**Contract clause**: `INV-001`, `PRE-001`, `PRE-002`, `PRE-003`

**Target**: `crates/vb_runtime/src/durability_matrix.rs::DURABILITY_MATRIX` and `REQUIRED_PRIMITIVES`

**Spec function**:
```verus
spec fn required_primitives_set() -> Set<&str> {
    Set::from_slice(REQUIRED_PRIMITIVES)
}

spec fn matrix_primitives_set(matrix: &[DurabilityRow]) -> Set<&str> {
    Set::from_seq(Seq::mapped(matrix, |row: DurabilityRow| row.primitive))
}

spec fn verify_matrix_completeness_spec(matrix: &[DurabilityRow]) -> bool {
    required_primitives_set() == matrix_primitives_set(matrix)
}
```

**Proof obligation**: Prove `verify_matrix_completeness() == Ok(())` iff `verify_matrix_completeness_spec(DURABILITY_MATRIX)` holds.

**Additional check**: Prove `REQUIRED_PRIMITIVES` contains no duplicates.

---

### VERUS-DM-003: `EventSeq` Monotonicity and Validity

**Contract clause**: `POST-009`, `POST-010`, `INV-004`

**Target**: `crates/vb_storage/src/types.rs::EventSeq`

**Spec function**:
```verus
spec fn event_seq_valid(seq: EventSeq) -> bool {
    seq.0 >= 0  // u64 is always non-negative
}

spec fn event_seq_order_preserved(a: EventSeq, b: EventSeq) -> bool {
    a.0 < b.0  ==>  a.get() < b.get()
}
```

**Proof obligations**:
- `EventSeq::new(v).get() == v` for all `u64 v` — injectivity of constructor
- `EventSeq::new` is monotonic: `v1 < v2 ==> EventSeq::new(v1).get() < EventSeq::new(v2).get()`
- Serde roundtrip: `EventSeq::new(v).serialize().deserialize() == EventSeq::new(v)`

**Shell exclusions**: No I/O, async, storage.

---

### VERUS-DM-004: `AckPoint::BeforeJournalAppend` Unreachability

**Contract clause**: `INV-005`, `INV-006`

**Target**: `crates/vb_runtime/src/durability_matrix.rs::AckPoint`

**Proof obligation**: Prove that `AckPoint::BeforeJournalAppend` cannot be constructed through any public API path that feeds into `DURABILITY_MATRIX` or any runtime handler.

**Strategy**: Kani harness proving `DURABILITY_MATRIX` contains no `BeforeJournalAppend` values at runtime (runtime proof). The Verus obligation is to prove the `AckPoint` type has exactly two variants and no constructor in the public API produces `BeforeJournalAppend` for use in matrix rows.

---

### VERUS-JA-001: `append_strict` Postcondition

**Contract clause**: `POST-006`

**Target**: `crates/vb_storage/src/journal/append.rs::append_strict`

**Spec function**:
```verus
spec fn append_strict_spec(journal: &FjallJournal, event: &JournalEvent) -> Result<(), JournalError> {
    let append_result = journal.append_unpersisted(event);
    if append_result.is_err() { append_result }
    else { journal.persist_strict() }
}
```

**Proof obligation**: Prove `append_strict(journal, event)` refines `append_strict_spec(journal, event)`. In particular, `Ok(())` is returned only if both `append_unpersisted` and `persist_strict` succeeded.

**Shell exclusions**: `persist_strict` calls `database.persist(SyncAll)` — treated as external oracle.

---

### VERUS-JA-002: `append_journaled` Postcondition

**Contract clause**: `POST-007`

**Target**: `crates/vb_storage/src/journal/append.rs::append_journaled`

**Spec function**:
```verus
spec fn append_journaled_spec(journal: &FjallJournal, event: &JournalEvent) -> Result<(), JournalError> {
    journal.append_unpersisted(event)  // no persist_strict call
}
```

**Proof obligation**: Prove `append_journaled` never calls `persist_strict`.

**Shell exclusions**: `append_unpersisted` is the only storage call.

---

## Lean Non-Applicability Statement

This bead does **not** require Lean, Aeneas, or Hax extraction for the following reasons:

1. **No algebraic theorem kernels**: The ordering properties (`append_strict` then `persist_strict`) are state-machine properties expressible in Verus as postconditions, not algebraic theorems requiring a proof assistant.
2. **No parser/codec grammar**: `RecordKind` enum encoding is covered by proptest roundtrip tests + Miri.
3. **No arithmetic lattice**: EventSeq monotonicity is a straightforward `u64` order property, not a complex lattice.
4. **No refinement extraction**: The Rust code is the ground truth; there is no more abstract model to refine to.

**Verus** is the correct tool for this bead's Rust-local proof obligations. It provides:
- `spec` functions for pure properties
- `proof` functions for deductive proofs
- `invariant` for loop/state invariants
- Trusted boundary wrappers for Fjall/SyncAll calls

---

## Theorem Obligations Summary

| ID | Contract Clause | Target | Layer | Status |
|----|----------------|--------|-------|--------|
| VERUS-DM-001 | POST-001/002, INV-002 | `verify_ack_after_persist` | Verus | Planned |
| VERUS-DM-002 | INV-001, PRE-001/002/003 | `DURABILITY_MATRIX` completeness | Verus | Planned |
| VERUS-DM-003 | POST-009/010, INV-004 | `EventSeq` monotonicity | Verus | Planned |
| VERUS-DM-004 | INV-005/006 | `AckPoint` zombie variant | Verus | Planned |
| VERUS-JA-001 | POST-006 | `append_strict` postcondition | Verus | Planned |
| VERUS-JA-002 | POST-007 | `append_journaled` postcondition | Verus | Planned |

---

## Waivers

| ID | Obligation | Owner | Reason | Compensating Evidence |
|----|-----------|-------|--------|------------------------|
| W-LEAN-001 | Lean kernel for journal barrier | rust-contract | Verus postconditions capture append-then-persist ordering sufficiently | Kani harness on `append_strict` + integration tests |
| W-LEAN-002 | Aeneas extraction | rust-contract | No algebraic refinement beyond Verus expressiveness | Kani + Loom coverage |
| W-LEAN-003 | Hax extraction | rust-contract | Not a language-level safety proof requiring Hax | Proptest + Miri codec coverage |
