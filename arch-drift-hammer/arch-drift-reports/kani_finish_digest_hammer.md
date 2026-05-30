# Architectural Drift Hammer Report

**File:** `crates/vb_compile/src/kani_finish_digest.rs`
**Line Count:** 312 (exceeds 300-line limit by 12 lines, 4% over)
**Classification:** KANI VERIFICATION HARNESS / PROOF INFRASTRUCTURE
**Workspace:** `arch-drift-hammer`

---

## Executive Summary

This 312-line Kani verification harness file **violates the <300 line rule** and exhibits **severe primitive obsession** throughout. The file proves properties of `digest_step_primitive`'s Finish arm encoding but does so using raw primitives (`usize`, `[u8; N]`, tuples) instead of domain types. The encoding helpers and proof infrastructure are intermingled, violating separation of concerns.

---

## Violation 1: LINE COUNT (312 > 300)

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 312 | 300 | **FAIL (+12 lines, +4%)** |

---

## Violation 2: PRIMITIVE OBSESSION (Scott Wlaschin DDD)

### 2.1 `MAX_BYTE_LEN` — Naked Primitive

```rust
const MAX_BYTE_LEN: usize = 16;
```

**Problem:** `usize` is a primitive. This should be a newtype domain value:

```rust
struct FinishDigestBound(const u8);  // Max 16
impl FinishDigestBound {
    const MAX: Self = Self(16);
}
```

### 2.2 Return Type `([u8; MAX_BYTE_LEN], usize)` — Tuple of Primitives

**Affected functions:**
- `encode_finish_string_bytes` (line 102)
- `kani_digest_finish_result` (line 127)

**Problem:** The encoding returns a raw tuple `([u8; MAX_BYTE_LEN], usize)` — this is a **primitive obsession violation**. The domain concept is "Finish encoding" but it's represented as raw bytes + length.

**Should be:**

```rust
struct FinishEncoding {
    bytes: [u8; MAX_BYTE_LEN],
    len: FinishDigestBound,
}
```

### 2.3 `encode_finish_integer(value: i64) -> [u8; 8]` — Raw Array Primitive

**Problem:** Returns `[u8; 8]` — raw array primitive with no domain wrapper.

**Should be:**

```rust
struct IntegerEncoding([u8; 8]);
impl IntegerEncoding {
    fn new(value: i64) -> Self { Self(value.to_le_bytes()) }
}
```

### 2.4 `encodings_differ` — Tuple Parameter Primitive Obsession

```rust
fn encodings_differ(
    (bytes1, len1): &([u8; MAX_BYTE_LEN], usize),
    (bytes2, len2): &([u8; MAX_BYTE_LEN], usize),
) -> bool {
```

**Problem:** The domain concept is "comparing two encodings" but parameters are raw tuples of primitives.

**Should accept:**

```rust
fn encodings_differ(e1: &FinishEncoding, e2: &FinishEncoding) -> bool
```

### 2.5 `string_vs_integer_differ` — More Tuple/Primitive Parameters

```rust
fn string_vs_integer_differ(string_enc: &([u8; MAX_BYTE_LEN], usize), int_enc: &[u8; 8]) -> bool
```

**Problem:** Same issue — raw tuples instead of domain types.

### 2.6 Kani Harness Parameters — No Domain Wrappers

```rust
fn finish_string_result_injectivity() {
    let bytes1: [u8; MAX_BYTE_LEN] = kani::any();
    let bytes2: [u8; MAX_BYTE_LEN] = kani::any();
    let len1: usize = kani::any();
    let len2: usize = kani::any();
```

**Problem:** `kani::any()` returns raw primitives. The domain concept is "symbolic Finish byte content" but it's represented as raw arrays and lengths.

---

## Violation 3: DDD COHESION VIOLATIONS

### 3.1 No `FinishEncoding` Value Object

The entire file operates on raw bytes and lengths. There is **no domain type** representing a "Finish encoding". This violates Wlaschin's principle: **"Make illegal states unrepresentable"** — raw `[u8; 16]` with `usize` length can represent invalid states (e.g., `len > 16`).

### 3.2 `kani_digest_finish_result` Has Three Reasons to Change

```rust
pub(crate) fn kani_digest_finish_result(result: &ScalarValue) -> ([u8; MAX_BYTE_LEN], usize) {
    match result {
        ScalarValue::String(value) => { ... }
        ScalarValue::Integer(value) => { ... }
        _ => { ... }
    }
}
```

This function handles **three different encoding paths** in one function. Per Wlaschin, a function should have **one reason to change**. Each `ScalarValue` variant should have its own encoder.

### 3.3 Domain Logic Scattered Across Helper Functions

| Function | Responsibility | Problem |
|----------|----------------|---------|
| `encode_finish_string_bytes` | String encoding | Mixed with array manipulation |
| `encode_finish_integer` | Integer encoding | Mixed with array manipulation |
| `kani_digest_finish_result` | Dispatch + encode | Too many responsibilities |
| `encodings_differ` | Compare encodings | Should be `FinishEncoding::neq` |
| `string_vs_integer_differ` | Cross-variant compare | Should be on domain types |

---

## Violation 4: MIXED CONCERNS (Proof Infrastructure + Encoding Helpers)

The file serves **two masters**:

1. **Encoding helpers** — replicate production encoding logic
2. **Kani proofs** — verification harnesses

These should be separated:

```
kani_finish_digest.rs          (312 lines) — SPLIT INTO:
├── finish_encoding.rs          (domain types + encoding logic)
└── kani_finish_digest.rs       (proofs only, imports domain)
```

---

## Violation 5: GOD RULES — HARNESS CORRECTNESS CONCERNS

### 5.1 Model Reduction Trust Issue

The file claims (line 52):
> "Proofs bind to production-equivalent logic that replicates the actual `digest_step_primitive` Finish arm byte-for-byte."

**Problem:** The binding is via **code duplication comment** (lines 21-33), not a verified link. If `part_05.rs:204-210` changes, this Kani file silently becomes stale.

**Production code** (`part_05.rs:204-210`):
```rust
vb_yaml::ast::StepPrimitive::Finish { result } => {
    hasher.update(b"finish");
    match result {
        vb_yaml::ast::ScalarValue::String(value) => hasher.update(value.as_bytes()),
        vb_yaml::ast::ScalarValue::Integer(value) => hasher.update(&value.to_le_bytes()),
        _ => hasher.update(b"unsupported"),
    };
}
```

**Verification code** (lines 127-147):
```rust
pub(crate) fn kani_digest_finish_result(result: &ScalarValue) -> ([u8; MAX_BYTE_LEN], usize) {
    match result {
        ScalarValue::String(value) => { ... }
        ScalarValue::Integer(value) => { ... }
        _ => { ... }
    }
}
```

**The match arms are identical but there's no compile-time guarantee.**

### 5.2 `#[kani::unwind(32)]` Magic Number

```rust
#[kani::proof]
#[kani::unwind(32)]
fn finish_string_result_injectivity() {
```

The unwind value 32 is explained in comments (lines 76-77) but not enforced by type system. A domain-specific `UnwindHint` type would make this explicit.

---

## Required Refactors (Smallest Safe)

### Phase 1: Extract Domain Types (No behavior change)

```rust
// New: crates/vb_compile/src/kani/finish_encoding.rs
#![cfg(kani)]

use vb_yaml::ast::ScalarValue;

const MAX_BYTE_LEN: usize = 16;

/// Finish digest encoding — domain value object
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishEncoding {
    bytes: [u8; MAX_BYTE_LEN],
    len: usize,
}

impl FinishEncoding {
    /// Encode a Finish String result
    pub fn from_string(value: &str) -> Self { ... }
    
    /// Encode a Finish Integer result  
    pub fn from_integer(value: i64) -> Self { ... }
    
    /// Encode unsupported variant
    pub fn unsupported() -> Self { ... }
    
    /// Check if encodings differ
    pub fn differs_from(&self, other: &Self) -> bool { ... }
}
```

### Phase 2: Reduce `kani_finish_digest.rs` to Proofs Only

Move all encoding logic to the new domain module. `kani_finish_digest.rs` becomes ~150 lines of pure proofs.

### Phase 3: Unwind as Const Generic (Optional)

```rust
struct UnwindHint<const N: usize>;
const STRING_UNWIND: UnwindHint<32> = UnwindHint;
```

---

## Impact Assessment

| Aspect | Current | Target |
|--------|---------|--------|
| Line count | 312 | ~150 (prooft only) |
| Domain types | 0 | 3 (`FinishEncoding`, `FinishDigestBound`, `IntegerEncoding`) |
| Primitive returns | 5 functions | 0 |
| Primitive params | 5 functions | 0 |

---

## Proof Obligations

| ID | Description | Status |
|----|-------------|--------|
| PO-KANI-FINISH-001 | String result injectivity | VERIFIED (but needs domain types) |
| PO-KANI-FINISH-002 | Integer result injectivity | VERIFIED (but needs domain types) |
| PO-KANI-FINISH-003 | Variant discrimination | VERIFIED (but needs domain types) |

---

## Recommendation

**IMMEDIATE ACTION REQUIRED:** File exceeds 300-line limit. Split into:

1. `crates/vb_compile/src/kani/finish_encoding.rs` — Domain types + encoding helpers (~150 lines)
2. `crates/vb_compile/src/kani_finish_digest.rs` — Proofs only (~150 lines)

This reduces the original 312-line file below the 300-line threshold while properly implementing DDD principles.

---

**Report Generated:** 2026-05-29
**Enforcer:** architectural-drift agent
**Classification:** ARCHITECTURAL VIOLATION — PRIMITIVE OBSESSION + LINE COUNT
