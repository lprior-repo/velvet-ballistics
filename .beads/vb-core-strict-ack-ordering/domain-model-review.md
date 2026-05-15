# Domain Model Review — vb-core-strict-ack-ordering

## Context

- **bead_id**: vb-core-strict-ack-ordering
- **bead_title**: runtime/storage: Prove strict persistence before acknowledgement ordering
- **phase**: State 3 — Domain Model Review
- **updated_at**: 2026-05-15T00:00:00Z
- **attempt**: 1

## Reviewer

`scott-ddd-refactor` principles applied inline. `rust-contract` owns this document.

---

## Type-Boundary Analysis

### AckPoint — Two-Variant Enum

```rust
pub enum AckPoint {
    AfterJournalAppend,  // CORRECT — used in all matrix rows
    BeforeJournalAppend, // FORBIDDEN — no matrix row may hold this value
}
```

**Diagnosis**: `BeforeJournalAppend` is a "僵尸变体" — it exists in the type graph but is unreachable through any public constructor. However, it is NOT compile-time unrepresentable. A programmer could write:

```rust
DurabilityRow {
    primitive: "evil",
    ack_point: AckPoint::BeforeJournalAppend, // nothing stops this at compile time
    // ...
}
```

The enforcement is RUNTIME only (`verify_ack_after_persist()`). This is a **parse-don't-validate** violation: the type system should make illegal states unrepresentable.

**DDD Repair Option A** (preferred): Remove `BeforeJournalAppend` from the enum entirely. Replace `ack_point: AckPoint` field with a bool `ack_after_persist: ()` marker or a marker type `AfterJournalAppendMarker`. The runtime proof becomes a compile-time guarantee.

**DDD Repair Option B**: Keep the enum, accept runtime-only enforcement, and require Kani to prove the enum value is never observed in any matrix row at runtime.

**Recommendation**: Option A for `AckPoint`. The `BeforeJournalAppend` variant serves no constructive purpose — it exists only as a documentation of what must not happen.

---

### DurabilityRow — Anemic Record Structure

`DurabilityRow` is a plain data bag. No invariants are encoded in the type.

```rust
pub struct DurabilityRow {
    pub primitive: &'static str,
    pub compiled_node_kind: &'static str,
    pub journal_events: &'static [RecordKind],
    pub storage_partition: StoragePartition,
    pub ack_point: AckPoint,
    pub replay_assertion: &'static str,
    pub test_evidence: &'static [&'static str],
}
```

**Diagnosis**: Valid. No illegal states are representable through this structure given the `AckPoint` enum fix above. The `journal_events: &'static [RecordKind]` slice cannot be empty at runtime if the compile-time table is non-empty (which it is — 11 entries).

**Invariant that could be encoded**: `!journal_events.is_empty()`. Could be a `NonEmptySlice<RecordKind>` newtype, but the static initialization guarantees non-emptiness.

---

### EventSeq — Newtype Wrapper

```rust
#[repr(transparent)]
pub struct EventSeq(u64);
```

**Diagnosis**: Clean. The `#[repr(transparent)]` ensures zero-cost abstraction. Monotonicity is a runtime property (enforced by the sequencer logic in `impl_parts/chunk_001.rs`), not a type invariant. The `Ord` impl is valid because `u64` is totally ordered.

**Potential concern**: `EventSeq` implements `PartialOrd` and `Ord` based on raw `u64` values. For two `EventSeq` values `a` and `b`, `a < b` implies `a` was created earlier in the same run. This is the intended semantics.

---

### DurabilityProfile — Three-Variant Enum

```rust
pub enum DurabilityProfile {
    Volatile,   // in-memory only
    Journaled,  // group commit, no per-event barrier
    Strict,     // per-event barrier via append_strict
}
```

**Diagnosis**: Valid. Each variant has distinct runtime semantics enforced by dispatch logic in `StorageRuntimeJournal::append_storage_event`. No illegal states.

---

### JournalQueueCapacity / JournalBatchSize — NonZero Newtypes

```rust
#[repr(transparent)]
pub struct JournalQueueCapacity(NonZeroUsize);
#[repr(transparent)]
pub struct JournalBatchSize(NonZeroUsize);
```

**Diagnosis**: Correct. `NonZeroUsize` guarantees zero is never stored. The `try_from_usize` constructors make illegal states (zero) unrepresentable at the type level.

---

### StoragePartition — Three-Variant Enum

```rust
pub enum StoragePartition {
    RuntimeJournal,
    ActionJournal,
    TimerJournal,
}
```

**Diagnosis**: Valid. Pure marker enum for keyspace routing. No invariants needed.

---

## Illegal-State Summary

| Type | Illegal State | Currently Prevented By | Fix |
|------|-------------|----------------------|-----|
| `AckPoint` | `BeforeJournalAppend` used in matrix row | `verify_ack_after_persist()` runtime check | Remove variant or use marker type |
| `DurabilityRow` | Empty `journal_events` | Static table non-empty | Could use `NonEmptySlice` newtype |
| `EventSeq` | Negative values | `u64` has no negatives | None needed |
| `JournalQueueCapacity` | Zero capacity | `NonZeroUsize` + `try_from_usize` | None needed |
| `JournalBatchSize` | Zero batch size | `NonZeroUsize` + `try_from_usize` | None needed |

---

## Domain Model Findings

### Finding 1: `AckPoint::BeforeJournalAppend` is a zombie variant (Medium-High Risk)

The `BeforeJournalAppend` variant of `AckPoint` exists in the public API but must never be used in any `DURABILITY_MATRIX` row. The current enforcement is purely runtime — `verify_ack_after_persist()` will error if any row uses it, but compilation succeeds with no warning.

**Impact**: A future programmer adding a new matrix row could mistakenly use `BeforeJournalAppend` and the error would only surface at runtime during matrix verification. In the worst case, a production system could start with an incorrect matrix and only fail when `verify_matrix()` is called.

**Recommended Action**: Either (A) remove the variant from the enum and update `DurabilityRow.ack_point` to a marker type `AfterJournalAppend` that has no variant, or (B) add a Kani harness proving `DURABILITY_MATRIX` never contains `AckPoint::BeforeJournalAppend` at runtime.

**Bead Recommendation**: File a follow-up bead to remove the zombie variant. The current bead focuses on proving ordering for the existing matrix which is already correct.

---

### Finding 2: Matrix row test evidence paths need audit (Medium Risk)

`DURABILITY_MATRIX` rows point to `test_evidence: &["crates/vb_runtime/src/shard/tests.rs"]` for all 11 primitives. The path is identical for every row. This is likely a placeholder. `verify_matrix_replay_proofs()` only checks that the slice is non-empty, not that the referenced test actually exercises the specific primitive.

**Impact**: The proof completeness claim in `verify_matrix_replay_proofs()` is weak — it only confirms a path exists, not that the test actually runs the primitive.

**Recommended Action**: Audit the test file and update each `test_evidence` entry to point to the specific test function for that primitive, or add missing tests.

---

## DDD Assessment

The domain model is mostly sound. The `AckPoint` zombie variant is the primary concern. The other types use appropriate newtype patterns and `NonZeroUsize` to make illegal states unrepresentable.

The core ordering contract (`ack_after_persist` for every primitive) is correctly expressed in `contract.md` and enforced by the three verifier functions. The gap between type-level and runtime enforcement for `BeforeJournalAppend` is a known issue that warrants a follow-up bead.
