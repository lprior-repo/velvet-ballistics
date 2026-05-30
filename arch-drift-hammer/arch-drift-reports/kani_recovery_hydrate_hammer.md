# Architectural Drift Report: `kani_recovery_hydrate.rs`

**File**: `crates/vb_storage/src/kani_recovery_hydrate.rs`
**Total Lines**: 327
**Limit**: 300
**Violation**: YES (+27 lines over limit)

---

## 1. File Size Assessment

| Metric | Value |
|--------|-------|
| Actual lines | 327 |
| Maximum allowed | 300 |
| Overage | 27 lines (9%) |
| **Status** | **VIOLATION** |

---

## 2. Primitive Obsession Violations

### 2.1 `arbitrary_run_id()` — Raw `u64` Injection
**Location**: Lines 46-48

```rust
fn arbitrary_run_id() -> RunId {
    RunId::new(kani::any())
}
```

**Problem**: `kani::any()` returns an unconstrained `u64`. If `RunId::new()` has invariants (e.g., non-zero, range bounds, format constraints), this harness generates values that could violate those invariants. This is not `Arbitrary` for `RunId` — it's `Arbitrary` for `u64` masquerading as `RunId`.

**DDD Principle Violated**: "Parse, don't validate" — the harness does not parse a valid representation; it creates the type directly from a raw primitive.

**Fix Required**: Either constrain the `kani::any()` to valid `RunId` raw values, or introduce a `RunId::arb_unchecked()` marker for verification-only construction.

---

### 2.2 `arbitrary_taint()` — Primitive-to-Domain Enum Mapping
**Location**: Lines 50-58

```rust
fn arbitrary_taint() -> Taint {
    match kani::any::<u8>() % 5 {
        0 => Taint::Clean,
        1 => Taint::DerivedFromSecret,
        2 => Taint::Secret,
        3 => Taint::Random,
        _ => Taint::TimeDependent,
    }
}
```

**Problem**: `u8 % 5` produces a uniform distribution over `[0, 4]`, but the mapping is:
- `0` → `Clean`
- `1` → `DerivedFromSecret`
- `2` → `Secret`
- `3` → `Random`
- `4` → `TimeDependent`

The `_` arm catches only `4`, so all five variants are reachable. However, this is **primitive obsession**: encoding the domain enum via arithmetic on a raw `u8`. If `Taint` ever adds a variant, this code silently misroutes it.

**DDD Principle Violated**: Exhaustive enum matching without compiler enforcement when new variants are added.

**Fix Required**: Implement `kani::Arbitrary` for `Taint` directly via `match kani::any::<u8>() { 0 => Taint::Clean, ... }` with an explicit `_ =>` that calls `kani::panic()` or `kani::cover!` for unreachable.

---

### 2.3 `TailMetadataBatch` — Unenforced Length Constraint
**Location**: Lines 26-29

```rust
struct TailMetadataBatch {
    len: u8,
    events: [TailEventMetadata; MAX_TAIL_EVENTS_USIZE],
}
```

**Problem**: The `len` field (`u8`) is a runtime constraint that should be a type-level constraint. The array is always `[TailEventMetadata; 4]` regardless of `len`. This allows creating a batch with `len = 255` but only 4 valid entries.

**DDD Principle Violated**: Making illegal states representable. The type system should encode the invariant.

**Fix Required**: Either use a bounded vector type (if available) or create a `TailMetadataBatch<const N: usize>` newtype wrapper. At minimum, add a runtime invariant check that `len <= MAX_TAIL_EVENTS`.

---

### 2.4 Raw `u64` in `replay_next_seq_overflow_boundary`
**Location**: Line 115

```rust
let raw: u64 = kani::any();
```

**Problem**: Using raw `u64` instead of a constrained `EventSeqRaw` or similar newtype. If `EventSeq` wraps `u64` with invariants (e.g., non-max for some operations), this harness does not respect them.

---

### 2.5 Raw `usize` Primitives for Limits
**Location**: Lines 138-139

```rust
let raw_limit: usize = kani::any();
let current_len: usize = kani::any();
```

**Problem**: These are later converted to `EventReplayLimit` via `EventReplayLimit::new(raw_limit)`. The harness creates a `usize` then immediately validates it. This is the "validate, then parse" anti-pattern.

**DDD Principle Violated**: "Parse, don't validate" — the harness should generate `EventReplayLimit` directly, not a raw `usize`.

---

## 3. DDD Structural Observations

### 3.1 Batch Predicate Functions — Potential Anemia
**Locations**:
- `batch_has_run_mismatch` (lines 88-98)
- `batch_has_seq_not_after` (lines 100-110)

These functions duplicate the logic in `tail_run_scan` and `tail_seq_scan`. They are technically correct but represent a code smell:

**Observation**: The batch predicate functions are **anemic helper functions** that exist only to support the proof. In proper DDD, the `TailMetadataBatch` type should implement methods that return results directly, not standalone functions.

**Example of better design**:
```rust
impl TailMetadataBatch {
    fn has_run_mismatch(&self, run_id: RunId) -> bool { ... }
    fn has_seq_not_after(&self, snapshot_seq: EventSeq) -> bool { ... }
}
```

---

### 3.2 Module Coupling
**Observation**: This file imports from:
- `crate::journal::EventReplayLimit`
- `crate::journal::replay::{ReplayPushLimitDecision, classify_replay_push_len}`
- `crate::recovery::hydrate::{...}`
- `crate::recovery::hydrate_support::{...}`

This is a **cross-module integration test** at the Kani harness level. This is acceptable for verification harnesses but creates tight coupling between verification and production modules.

---

### 3.3 Proof Inventory

| Proof Name | Lines | Focus |
|------------|-------|-------|
| `replay_next_seq_overflow_boundary` | 112-133 | `next_seq` overflow at `u64::MAX` |
| `replay_push_limit_decision_matches_checked_count` | 135-195 | `ReplayPushLimitDecision` exhaustively covers all `checked_add` outcomes |
| `snapshot_metadata_rejects_run_mismatch` | 197-224 | `validate_snapshot_metadata` run mismatch rejection |
| `tail_run_scan_matches_any_metadata_batch_len_le_4` | 226-248 | `tail_run_scan` with bounded batch |
| `tail_seq_scan_matches_any_metadata_batch_len_le_4` | 250-269 | `tail_seq_scan` with bounded batch |
| `recovery_data_presence_rejects_only_all_empty` | 271-289 | `validate_recovery_data_present` all-empty rejection |
| `slot_taint_resolution_fails_closed_on_read_failure` | 291-300 | `SlotTaintResolution` fail-closed on `Failed` |
| `slot_taint_resolution_defaults_clean_only_for_uninitialized` | 302-311 | `SlotTaintResolution` Clean-only for `Uninitialized` |
| `slot_taint_resolution_preserves_existing_taint` | 313-327 | `SlotTaintResolution` preserves `Existing` taint |

**Observation**: 9 proofs, 327 lines. Average ~36 lines per proof. This suggests the file is doing too much and could be split by domain concern.

---

## 4. Recommendations

### 4.1 File Split Required

Split into at least **3 files**:

1. **`kani_recovery_hydrate_seq.rs`** (~110 lines)
   - `replay_next_seq_overflow_boundary`
   - `replay_push_limit_decision_matches_checked_count`

2. **`kani_recovery_hydrate_snapshot.rs`** (~110 lines)
   - `snapshot_metadata_rejects_run_mismatch`
   - `tail_run_scan_matches_any_metadata_batch_len_le_4`
   - `tail_seq_scan_matches_any_metadata_batch_len_le_4`

3. **`kani_recovery_hydrate_taint.rs`** (~100 lines)
   - `slot_taint_resolution_fails_closed_on_read_failure`
   - `slot_taint_resolution_defaults_clean_only_for_uninitialized`
   - `slot_taint_resolution_preserves_existing_taint`
   - `recovery_data_presence_rejects_only_all_empty`

### 4.2 Shared Arbitrary Types

Create **`kani_recovery_hydrate_arbs.rs`** (~70 lines) containing:
- `TailMetadataBatch` struct
- `impl kani::Arbitrary for TailEventMetadata`
- `impl kani::Arbitrary for TailMetadataBatch`
- `arbitrary_run_id()`
- `arbitrary_taint()`
- `MAX_TAIL_EVENTS` and `MAX_TAIL_EVENTS_USIZE` constants

### 4.3 Primitive Obsession Fixes

1. **Add `RunId::arb_unchecked()`**: Mark this as only for verification, with a comment explaining the constraint.

2. **Add `Taint::arb()`**: Implement `kani::Arbitrary` directly on `Taint` with explicit variant coverage.

3. **Add runtime invariant to `TailMetadataBatch`**: Ensure `len <= MAX_TAIL_EVENTS` in constructor.

---

## 5. Summary

| Category | Severity | Count |
|----------|----------|-------|
| File size violation | **HIGH** | 1 |
| Primitive obsession | **MEDIUM** | 5 |
| DDD anemic functions | **LOW** | 2 |
| Cross-module coupling | **INFO** | 1 |

**Overall Status**: `ARCH-DRIFT-DETECTED`

**Required Actions**:
1. Split file into 4 modules (3 harness + 1 shared arbitrary types)
2. Fix `arbitrary_run_id()` to respect `RunId` invariants
3. Fix `arbitrary_taint()` to use direct enum `Arbitrary`
4. Add `TailMetadataBatch` invariant check

---

*Report generated by: architectural-drift enforcer*
*Date: Fri May 29 2026*
