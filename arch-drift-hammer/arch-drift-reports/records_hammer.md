# Architectural Drift Report: `vb_storage/src/records.rs`

**File**: `crates/vb_storage/src/records.rs`
**Line Count**: 326 lines (**VIOLATION**: exceeds 300-line limit by 26 lines)
**Date**: 2026-05-29
**Enforcer**: arch-drift-hammer

---

## Executive Summary

| Category | Count | Severity |
|----------|-------|----------|
| Line Count Violations | 1 | 🔴 CRITICAL |
| Primitive Obsession Violations | 5 | 🔴 CRITICAL |
| Anemic Type Violations | 2 | 🟠 HIGH |
| Leaky Abstraction Violations | 1 | 🟠 HIGH |

---

## 1. LINE COUNT VIOLATION

**Rule**: All source files must be ≤300 lines.
**Status**: 🔴 **VIOLATION** — 326 lines total (26 lines over)

### Breakdown by Section

| Section | Lines |占比 |
|---------|-------|-----|
| `RunHeaderStatus` type system (1-133) | 133 | 40.8% |
| `RecordKind` enum + impl (135-224) | 90 | 27.6% |
| Record structs (226-283) | 58 | 17.8% |
| `RunHeaderRecord` impl (263-274) | 12 | 3.7% |
| Tests (285-326) | 42 | 12.9% |

### Root Cause

Two concerns are co-located: the `RunHeaderStatus` value object system (133 lines) and the storage record types (163 lines + 42 tests). These belong in separate files.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 `BlobRecord.digest: [u8; 32]` 🔴 CRITICAL

**Location**: Line 280

```rust
pub struct BlobRecord {
    pub digest: [u8; crate::constants::DIGEST_BYTES],  // ← VIOLATION
    pub bytes: Vec<u8>,
}
```

**Problem**: A 32-byte raw array is used instead of a domain `BlobDigest` type. Callers must know `DIGEST_BYTES = 32` to construct or destructure this. No type safety between blob digests and other digests (e.g., `WorkflowDigest`).

**Fix**: Create a `BlobDigest` newtype in `vb_core` or `vb_storage`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(transparent)]
pub struct BlobDigest([u8; crate::constants::DIGEST_BYTES]);

impl BlobRecord {
    pub fn digest(&self) -> BlobDigest { BlobDigest(self.digest) }
}
```

### 2.2 `RunHeaderRecord.status: u8` 🟠 HIGH

**Location**: Line 258

```rust
pub struct RunHeaderRecord {
    pub run: RunId,
    pub workflow_id: WorkflowId,
    pub compiled_digest: WorkflowDigest,
    pub status: u8,  // ← VIOLATION: wire format leaks into domain
    pub accepted_at_ms: u64,
}
```

**Problem**: The comment on lines 254-258 admits this is a "wire format" leak. The field is `u8` because of storage compatibility, but domain code must treat this as a `RunHeaderStatus`. Every call site must remember to call `run_header_status()`.

**Fix**: Change field to `status: RunHeaderStatus`. If wire compatibility absolutely requires `u8` at the persistence layer, add a separate `status_wire: u8` private field and keep the public `status: RunHeaderStatus` via serialization wrappers.

### 2.3 `WorkflowSourceRecord.source: Vec<u8>` 🟠 HIGH

**Location**: Line 232

```rust
pub struct WorkflowSourceRecord {
    pub digest: WorkflowDigest,
    pub source: Vec<u8>,  // ← VIOLATION: raw bytes
}
```

**Problem**: "Original strict YAML authoring bytes" stored as `Vec<u8>` has no domain type. Callers cannot distinguish between source bytes and IR bytes without context.

**Fix**: Add a `WorkflowSource` newtype wrapper:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkflowSource(Vec<u8>);
```

### 2.4 `CompiledIrRecord.ir: Vec<u8>` 🟠 HIGH

**Location**: Line 241

```rust
pub struct CompiledIrRecord {
    pub digest: WorkflowDigest,
    pub ir: Vec<u8>,  // ← VIOLATION: raw bytes
}
```

**Problem**: Same primitive obsession as 2.3. "Postcard-compatible compiled artifact bytes" has no domain wrapper.

**Fix**: Add a `CompiledIr` newtype wrapper.

### 2.5 `BlobRecord.bytes: Vec<u8>` 🟠 HIGH

**Location**: Line 282

```rust
pub struct BlobRecord {
    pub digest: [u8; crate::constants::DIGEST_BYTES],
    pub bytes: Vec<u8>,  // ← VIOLATION: raw bytes
}
```

**Problem**: Blob payload has no domain type. A `BlobPayload` wrapper would enforce invariants (e.g., max size) at the type level.

---

## 3. ANEMIC TYPE VIOLATIONS

### 3.1 `RecordKind` — Anemic Enum with Redundant ID Method

**Location**: Lines 135-224

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u16)]
#[non_exhaustive]
pub enum RecordKind {
    WorkflowSource = 1,
    CompiledIr = 2,
    // ... 22 variants
}

impl RecordKind {
    pub const fn id(self) -> u16 {
        match self {
            Self::WorkflowSource => 1,
            Self::CompiledIr => 2,
            // ... repeats every variant
        }
    }
}
```

**Problems**:

1. The `#[repr(u16)]` already maps variants to u16 values, but Rust doesn't auto-implement `.id()`. The match body is **redundant boilerplate** — every variant just returns its own discriminant.
2. The enum is behavior-free — no validation, no transitions, no domain logic.
3. This is a **Scott Wlaschin "Anemic Domain Model"** anti-pattern: types that are just data bags with no behavior.

**Fix Options**:

- **Option A (Minimal)**: Derive `Into<u16>` and remove the manual `id()` method.
- **Option B (DDD Proper)**: Move `RecordKind` to `vb_core` and add domain behavior (e.g., `RecordKind::is_event()`, `RecordKind::category()`).

### 3.2 `BlobRecord` — Plain Data Bag

**Location**: Lines 276-283

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BlobRecord {
    pub digest: [u8; crate::constants::DIGEST_BYTES],
    pub bytes: Vec<u8>,
}
```

**Problem**: No constructor, no invariants, no methods. A proper DDD aggregate would have factory methods, validation, and encapsulated behavior.

---

## 4. LEAKY ABSTRACTION

### 4.1 `RunHeaderRecord` Status Wire Leak

**Location**: Lines 245-274

The `RunHeaderRecord` struct exposes `status: u8` as a public field (line 258), forcing the wire format into the domain API. The typed accessors `run_header_status()` and `set_run_header_status()` (lines 266-273) are a **post-hoc correction** — the field should never have been public.

**Evidence** (from existing code):
```rust
// Every caller must do this:
let typed_status = record.run_header_status();  // u8 → RunHeaderStatus
// instead of:
let typed_status = record.status;  // direct u8 access is available but wrong
```

---

## 5. WHAT IS ACTUALLY GOOD

### `RunHeaderStatus` Type System (Lines 1-133) ✅

This is **exemplary DDD**. The value object wrapping a `u8` with `Known`/`Unknown` classification, lossless `classify()`, and `TryFrom` implementations is exactly Scott Wlaschin style:

- **Type encoding**: Invalid bytes cannot be constructed accidentally
- **Lossless classification**: No data lost to runtime errors
- **Exhaustive matching**: Compile-time enforcement of completeness
- **Zero unsafe**: No `unsafe_code`, no `unwrap`, no `panic`

This section alone is 133 lines but is **justified** — it replaces a raw `u8` status field throughout the codebase.

---

## 6. PRESCRIPTIVE REFACTORING PLAN

### Phase 1: Split File (26 lines over limit)

```
records.rs (300 max)
├── run_header_status.rs (133 lines) — value object, KEEP
├── record_kinds.rs (90 lines) — enum + impl, FIX redundant id()
└── storage_records.rs (103 lines) — structs, FIX primitive obsession
```

### Phase 2: Fix Primitive Obsession

| Field | Current Type | Target Type |
|-------|-------------|-------------|
| `BlobRecord.digest` | `[u8; 32]` | `BlobDigest` newtype |
| `BlobRecord.bytes` | `Vec<u8>` | `BlobPayload` newtype |
| `WorkflowSourceRecord.source` | `Vec<u8>` | `WorkflowSource` newtype |
| `CompiledIrRecord.ir` | `Vec<u8>` | `CompiledIr` newtype |
| `RunHeaderRecord.status` | `u8` | `RunHeaderStatus` (private wire field) |

### Phase 3: Fix `RecordKind` Anemia

Derive `Into<u16>` instead of manual `id()`:

```rust
impl From<RecordKind> for u16 {
    fn from(kind: RecordKind) -> Self {
        kind as Self
    }
}
```

Delete the 28-line `id()` match that mirrors the derive.

---

## 7. VERIFICATION COMMANDS

```bash
# Count lines
wc -l crates/vb_storage/src/records.rs
# Expected: ≤300

# Check for primitive types in struct fields
rg 'pub (?:digest|source|ir|bytes|status):\s+(?:u8|Vec<u8>|\[u8;' crates/vb_storage/src/records.rs
# Expected: zero matches after fix
```

---

## 8. SUMMARY SCORECARD

| Metric | Before | After (Target) |
|--------|--------|----------------|
| Line count | 326 | ≤300 |
| Primitive-obsessed fields | 5 | 0 |
| Anemic types | 2 | 0 |
| Leaky abstractions | 1 | 0 |
| Unsafe blocks | 0 | 0 |

---

**VERDICT**: 🔴 **ARCHITECTURAL DRIFT CONFIRMED**

File exceeds line limit and contains 5 primitive obsession violations. The `RunHeaderStatus` type system is exemplary; the rest of the file needs disciplined refactoring per the plan above.
