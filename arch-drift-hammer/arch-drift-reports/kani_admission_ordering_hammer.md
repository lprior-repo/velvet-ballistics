# ARCHITECTURAL DRIFT REPORT
## File: `vb_runtime/src/verification/kani/kani_admission_ordering.rs`
## Lines: 340 (VIOLATION: exceeds 300-line limit by 40 lines)
## Date: 2026-05-29
## Severity: CRITICAL

---

## EXECUTIVE SUMMARY

This Kani proof harness file violates the **<300-line rule** and exhibits **severe primitive obsession** throughout. The file uses raw `u8` integers to enumerate `JournalError` variants instead of leveraging the type system properly, defeating the purpose of formal verification.

---

## VIOLATION 1: LINE COUNT (40 lines over limit)

| Metric | Value |
|--------|-------|
| Actual | 340 lines |
| Limit | 300 lines |
| Overflow | 40 lines (13.3% violation) |

**Required Action:** Split into two files:
- `kani_admission_ordering.rs` (core harnesses)
- `kani_admission_error_conversion.rs` (error conversion specific)

---

## VIOLATION 2: PRIMITIVE OBSESSION — u8 Variant Enumeration

**Lines 55-68, 303-317**

```rust
// VIOLATION: Using raw u8 to enumerate domain error variants
let variant: u8 = kani::any();
kani::assume(variant < 6);
let journal_error = match variant {
    0 => JournalError::KeyCapacity,
    1 => JournalError::DuplicateEvent { run, seq: StorageEventSeq(kani::any::<u64>()) },
    2 => JournalError::SequenceOverflow,
    3 => JournalError::WriteLockPoisoned,
    4 => JournalError::QueueFull,
    _ => JournalError::QueueCapacity,
};
```

**Problem:** This is textbook primitive obsession. The code:
1. Creates a raw `u8` to model a domain concept (`JournalError` variant selection)
2. Uses a magic number `6` as the upper bound with no связь to the actual variant count
3. Maps integers to domain types via a fragile match expression that can silently desync

**Correct Approach:**
```rust
// If JournalError implements Arbitrary:
let journal_error: JournalError = kani::any();

// Or if manual generation is required, use a proper generator:
fn any_journal_error() -> JournalError {
    match kani::any::<u8>() % 6 {
        0 => JournalError::KeyCapacity,
        // ... variants with proper domain types
    }
}
```

---

## VIOLATION 3: RAW u64 IN type_newtype PATTERN

**Lines 62, 184, 214, 310**

```rust
// Line 62, 310: raw u64 passed to StorageEventSeq
seq: StorageEventSeq(kani::any::<u64>())

// Line 184: literal 42 (magic number)
let seq = StorageEventSeq(42);
```

**Problem:** `StorageEventSeq` is a newtype wrapping `u64`. Directly passing raw `u64` values:
1. Bypasses any invariants the constructor might enforce
2. Uses magic numbers (42) instead of meaningful constants
3. Duplicates the constraint logic that should be in `StorageEventSeq::new()`

**Correct Approach:**
```rust
// Use the constructor if it validates
let seq = StorageEventSeq::new(kani::any::<u64>())?; // if Result
// Or ensure the newtype constructor is used
```

---

## VIOLATION 4: Arc STRONG_COUNT INTROSPECTION

**Lines 84-87**

```rust
kani::assert(
    Arc::strong_count(&source) >= 1,
    "Arc must have at least one strong reference",
);
```

**Problem:** This assertion:
1. Checks Rust internal reference counting, not domain behavior
2. Is trivially satisfied by any Arc that exists
3. Proves nothing about the correctness of the error conversion

**Correct Approach:** Remove this assertion entirely. The proof should verify:
- The error type is correctly preserved (`RuntimeError::AdmissionHeaderPersistenceFailed`)
- The error can be handled downstream
- Not Arc reference counts

---

## VIOLATION 5: REPEATED SHARD SETUP

**Lines 36-38, 119, 142, 183, 210, 250**

```rust
fn new_shard() -> Shard {
    Shard::new(ShardConfig::default())
}
```

This helper is repeated but the file doesn't establish a shared test infrastructure module. Each harness recreates the shard, leading to code duplication that bloats line count.

---

## SCOTT WLASCHIN DDD ASSESSMENT

| Principle | Status | Finding |
|-----------|--------|---------|
| Make Illegal States Unrepresentable | ❌ FAIL | Raw `u8` integers used instead of `JournalError` enum directly |
| Value Objects | ❌ FAIL | `StorageEventSeq(u64)` constructed with raw integers |
| Domain Errors as Types | ⚠️ PARTIAL | `RuntimeError` enum used but primitives leak through in generators |
| No Primitive Obsession | ❌ FAIL | `u8`, `u64` used throughout as variant selectors |
| Single Responsibility | ⚠️ PARTIAL | Each proof function is focused but helpers are duplicated |

---

## PROOF OBLIGATIONS MAPPING

| Obligation | Target | Status |
|------------|--------|--------|
| PO-vb282my-AD-KANI-001 | `apply(Submit) → Initial` | ✅ Targets production code |
| PO-vb282my-AD-KANI-002 | `apply(Submit) produces Initial` | ✅ Correct |
| PO-vb282my-AD-KANI-003 | Error conversion + cleanup | ⚠️ Mixed with Arc introspection |
| PO-vb282my-AD-KANI-004 | RunAdmission failure cleanup | ✅ Correct |
| PO-vb282my-AD-KANI-005 | Error conversion | ❌ Primitive obsession pollutes |
| PO-vb282my-AD-KANI-006 | No live state on failure | ✅ Correct |

---

## REQUIRED REFACTORING

### 1. Split the File (Mandatory)

```
kani_admission_ordering.rs      (260 lines) — core ordering proofs
kani_admission_error_paths.rs   ( 80 lines) — error conversion coverage
```

### 2. Replace Primitive Obsession with Domain-Generative Types

Create a generator in the crate's test infrastructure:

```rust
// In vb_runtime/src/verification/kani/journal_error_gen.rs
impl kani::Arbitrary for JournalError {
    fn any() -> Self {
        match kani::any::<u8>() % 7 {
            0 => JournalError::KeyCapacity,
            1 => JournalError::DuplicateEvent { run: any_run_id(), seq: any_storage_event_seq() },
            // ...
        }
    }
}
```

### 3. Remove Arc Introspection (Lines 84-87)

Replace with behavioral assertion:
```rust
// Verify error flows correctly through the system
kani::cover!(matches!(result, RuntimeError::AdmissionHeaderPersistenceFailed { .. }));
```

### 4. Replace Magic Numbers

```rust
// Instead of:
kani::assume(variant < 6);

// Use:
const JOURNAL_ERROR_VARIANT_COUNT: usize = 6;
kani::assume(variant < JOURNAL_ERROR_VARIANT_COUNT as u8);
```

---

## GOD RULE COMPLIANCE

| Rule | Status | Finding |
|------|--------|---------|
| Rule 1: All inputs use kani::any() | ⚠️ PARTIAL | `u8` variant selector uses any(), but `JournalError` is constructed via fragile match |
| Rule 2: Harnesses call production functions | ✅ PASS | All harnesses call `apply()`, `admission_header_persistence_failed()`, `discard_journal_sequence()` |

---

## VERDICT

**GUILTY** — File exceeds line limit and contains severe primitive obsession that undermines the formal verification mission. The use of raw `u8` integers to model domain error variants defeats type safety and creates silent desync risks.

**IMMEDIATE ACTIONS REQUIRED:**
1. Split file to meet 300-line limit
2. Implement proper `Arbitrary` for `JournalError`
3. Remove Arc strong_count introspection
4. Replace magic numbers with named constants

---

*Report generated by architectural-drift agent*
