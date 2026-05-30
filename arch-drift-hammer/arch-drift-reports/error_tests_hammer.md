# Architectural Drift Report: `error_tests.rs`

**File:** `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/error_tests.rs`
**Line Count:** 558 (exceeds 300-line limit by **86%**)
**Status:** VIOLATION

---

## 1. Line Count Violation

| Metric | Value |
|--------|-------|
| Actual lines | 558 |
| Limit | 300 |
| Excess | 258 lines |
| % Over | 86% |

This file MUST be split into focused test modules.

---

## 2. Primitive Obsession Violations

### 2.1 `MissingRequiredProofFlag` — `&'static str` instead of `ProofFlag`

**Location:** `error/mod.rs:146-149`

```rust
MissingRequiredProofFlag {
    /// The flag that is missing.
    flag: &'static str,   // ← PRIMITIVE OBSESSION
},
```

**Test usage:** `error_tests.rs:519-527`

```rust
let err = JournalError::MissingRequiredProofFlag {
    flag: "contract_seal",   // ← raw string literal
};
```

**Domain type exists:** `crate::admission::ProofFlag` (defined in `admission.rs:47-58`) is the proper newtype but is NOT used here.

**Inconsistency:** `ArtifactEnvelopeError::MissingRequiredProofFlag` (in `artifact.rs:76-78`) correctly uses `ProofFlag`, but `JournalError::MissingRequiredProofFlag` uses `&'static str`.

### 2.2 `InputTooLarge` — Raw `u32` for byte sizes

**Location:** `error/mod.rs:167-172`

```rust
InputTooLarge {
    /// Observed input length.
    len: u32,   // ← raw u32
    /// Maximum accepted input length.
    max: u32,   // ← raw u32
},
```

**Test usage:** `error_tests.rs:133-164`

```rust
let err = JournalError::InputTooLarge {
    len: 1024,  // ← raw integer
    max: 512,
};
```

**Should be:** `ByteSize(u32)` newtype wrapper.

### 2.3 `TooManyEvents` — Raw `usize` for counts

**Location:** `error/mod.rs:207-214`

```rust
TooManyEvents {
    /// Run being replayed.
    run: RunId,
    /// Maximum event count allowed.
    limit: usize,      // ← raw usize
    /// Observed event count that crossed the limit.
    observed: usize,   // ← raw usize
},
```

**Test usage:** `error_tests.rs:339-391`

```rust
let err = JournalError::TooManyEvents {
    run,
    limit: 100,     // ← raw integer
    observed: 200,
};
```

**Should be:** `EventCount(usize)` newtype wrapper.

### 2.4 `ReplayAllocationFailed` — Raw `usize` for allocation request

**Location:** `error/mod.rs:217-222`

```rust
ReplayAllocationFailed {
    /// Run being replayed.
    run: RunId,
    /// Event capacity requested.
    requested: usize,   // ← raw usize
},
```

**Test usage:** `error_tests.rs:398-448`

```rust
let err = JournalError::ReplayAllocationFailed {
    run,
    requested: 1024,
};
```

**Should be:** `EventCount(usize)` or `AllocationRequest(usize)` newtype.

### 2.5 `InvalidGateCount` — Raw `u8` for gate count

**Location:** `error/mod.rs:139-143`

```rust
InvalidGateCount {
    /// Found gate count.
    found: u8,   // ← raw u8
},
```

**Test usage:** `error_tests.rs:479-511`

```rust
let err = JournalError::InvalidGateCount { found: 42 };
```

**Should be:** `GateCount(u8)` newtype with validation (gate counts have specific valid range per contract §4.2).

---

## 3. Test Code Quality Violations

### 3.1 `panic!` in Tests (6 instances)

The file suppresses `clippy::panic` at line 8, then uses `panic!` in test assertions:

| Line | Test Function | Issue |
|------|---------------|-------|
| 100 | `artifact_invalid_variant_and_fields` | `panic!("expected ArtifactInvalid, got {other:?}")` |
| 142 | `input_too_large_variant_and_fields` | `panic!("expected InputTooLarge, got {other:?}")` |
| 356 | `too_many_events_variant_and_fields` | `panic!("expected TooManyEvents, got {other:?}")` |
| 413 | `replay_allocation_failed_variant_and_fields` | `panic!("expected ReplayAllocationFailed, got {other:?}")` |
| 485 | `invalid_gate_count_variant_and_fields` | `panic!("expected InvalidGateCount, got {other:?}")` |
| 526 | `missing_required_proof_flag_variant_and_fields` | `panic!("expected MissingRequiredProofFlag, got {other:?}")` |

**Rule violation:** Engineering Rules state "No `panic`" — this applies to test code.

**Fix:** Use `#[allow(clippy::panic)]` per-test (if needed), but prefer `assert!` or `try_from` patterns.

### 3.2 Test Duplication Pattern

Each error variant follows an identical 2-3 test pattern:

1. `_variant_and_fields` — destructures and asserts fields
2. `_display_format` — asserts `format!("{err}")` contains expected strings
3. `_error_code` — asserts `diagnostic_code()` returns expected code

This is **97 lines of repetitive scaffolding** that could be replaced with a procedural macro or a shared test helper module.

---

## 4. Test Coverage Debt

The file header (lines 31-62) documents **31 untested variants**:

```
// Untested variants:
// - Fjall: no direct test (requires fjall mock/integration)
// - Encode: no direct test (requires postcard mock)
// - KeyCapacity: no direct test
// - DuplicateEvent: no direct test
// - WriteLockPoisoned: no direct test
// - QueueCapacity: no direct test
// - QueueFull: no direct test
// - QueueShutdown: no direct test
// - WrongRun: no direct test
// - SequenceGap: no direct test
// - SequenceOverflow: no direct test
// - BadMagic: no direct test
// - UnsupportedSchemaVersion: no direct test
// - MigrationRequired: no direct test
// - UnknownRecordKind: no direct test
// - RecordKindFamilyMismatch: no direct test
// - HeaderLengthMismatch: no direct test
// - PayloadTooLarge: no direct test
// - HeaderChecksumMismatch: no direct test
// - PayloadDigestMismatch: no direct test
// - UnexpectedEof: no direct test
// - PostcardDecodeFailed: no direct test
// - InvalidEvent: no direct test
// - ArtifactMalformed: no direct test
// - ArtifactChecksumMismatch: no direct test
// - ArtifactNotFound: no direct test
// - InvalidRunId: no direct test
// - StrictDurabilityFailed: no direct test
// - ProcessLockHeld: no direct test
// - ProcessLockIo: no direct test
// - Trim: no direct test
```

This is a **living TODO list masquerading as documentation**. It should be tracked in beads, not embedded in test code.

---

## 5. Domain Model Inconsistencies

### 5.1 `ArtifactInvalidSource` Fragmentation

**`error/mod.rs:163`** uses:
```rust
source: ArtifactInvalidSource,
```
But `ArtifactInvalidSource` (from `artifact.rs`) only has ONE variant:
```rust
pub enum ArtifactInvalidSource {
    PayloadDigestMismatch,
}
```

Meanwhile, `ArtifactEnvelopeError` in the same file (`artifact.rs`) has 11+ variants covering the same error domain. This suggests `ArtifactInvalidSource` was split off incompletely.

### 5.2 `MissingRequiredProofFlag` Type Inconsistency

| Location | Type Used | Correct? |
|----------|-----------|----------|
| `JournalError::MissingRequiredProofFlag` (error/mod.rs:146) | `&'static str` | NO |
| `ArtifactEnvelopeError::MissingRequiredProofFlag` (artifact.rs:76) | `ProofFlag` | YES |

---

## 6. Refactoring Recommendations

### 6.1 Split the File

The 558-line file should be split into:

```
error_tests/
├── admission_required_tests.rs    (~15 lines)
├── artifact_invalid_tests.rs      (~25 lines)
├── input_too_large_tests.rs       (~25 lines)
├── too_many_events_tests.rs       (~40 lines)
├── replay_allocation_tests.rs     (~40 lines)
├── invalid_gate_count_tests.rs   (~30 lines)
├── missing_proof_flag_tests.rs    (~30 lines)
└── common_display_code_tests.rs   (shared helpers)
```

### 6.2 Create Newtypes

```rust
// In vb_storage/src/types.rs (new file)
pub struct ByteSize(u32);
pub struct EventCount(usize);
pub struct GateCount(u8);
// etc.
```

### 6.3 Fix `MissingRequiredProofFlag`

Change `JournalError::MissingRequiredProofFlag { flag: &'static str }` to use `ProofFlag` to match `ArtifactEnvelopeError`.

### 6.4 Replace `panic!` with `assert!`

Every `panic!("expected X, got {other:?}")` should be replaced with:

```rust
let other = match err {
    JournalError::ExpectedVariant { .. } => return, // already matched
    other => other,
};
assert!(false, "expected ExpectedVariant, got {other:?}");
```

Or better: use `matches!` with a `matches!(err, ExpectedVariant { .. })` assertion.

---

## Summary

| Category | Count | Severity |
|----------|-------|----------|
| Lines over limit | 258 | CRITICAL |
| Primitive obsession violations | 5 | HIGH |
| `panic!` in tests | 6 | HIGH |
| Untested variants documented | 31 | MEDIUM |
| Domain model inconsistencies | 2 | MEDIUM |

**Recommendation:** This file is a priority refactor. The 86% line count violation alone mandates immediate action. Begin by splitting into variant-specific test modules and addressing primitive obsession in the error types themselves.
