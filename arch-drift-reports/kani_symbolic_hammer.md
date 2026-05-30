# Architectural Drift Report: `kani_symbolic_code_validation.rs`

**File**: `crates/vb_core/src/kani/kani_symbolic_code_validation.rs`
**Total Lines**: 984
**Drift Status**: ❌ CRITICAL VIOLATIONS

---

## Executive Summary

| Rule | Status | Severity |
|------|--------|----------|
| Line Count (<300) | 984 lines | CRITICAL |
| Primitive Obsession | VIOLATED | HIGH |
| DDD Cohesion | FRAGMENTED | MEDIUM |

---

## 1. LINE COUNT VIOLATION

**Required**: < 300 lines
**Actual**: 984 lines
**Violation**: 328% of allowable size

### Breakdown by Responsibility

| Section | Lines | Purpose |
|---------|-------|---------|
| Types + Impl (13-92) | ~80 | Type definitions |
| CODE_REGISTRY (102-867) | 766 | Static data (90 entries) |
| Lookup fns (870-932) | ~63 | Helper functions |
| Kani harnesses (934-984) | ~51 | Verification proofs |

### Structural Violation

The file bundles **three concerns** that must be split:

1. **Type definitions** (lines 13-92) → Extract to `types.rs`
2. **CODE_REGISTRY data** (lines 102-867) → Extract to `registry.rs` or `codes.rs`
3. **Kani harnesses** (lines 934-984) → Extract to `kani_harnesses.rs`

---

## 2. KANI VERIFICATION HARNESSES MAPPING

### H1: `kani_from_static_validation`
- **Lines**: 939-955
- **Purpose**: Prove `from_static(s).is_some()` for all registered strings
- **Method**: Iterative assertion over `CODE_REGISTRY` entries
- **Unwind bound**: 100

### H2: `kani_from_static_rejects_unknown`
- **Lines**: 960-983
- **Purpose**: Prove `from_static(s).is_none()` for unregistered strings
- **Method**: Hardcoded negative cases + registry iteration
- **Unwind bound**: 100

### GOD RULES VIOLATION (CRITICAL)

**H2 uses hardcoded dummy data**: Line 968 hardcodes `"__DEFINITELY_NOT_REGISTERED__"` — this is **NOT** a valid Kani proof. The harness should use `kani::any()` to generate arbitrary unregistered strings, not a single fixed string.

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### PO-001: Raw `u16` for Diagnostic Codes

**Violation 1**: `CodeEntry::numeric` uses raw `u16`

```rust
// Line 94-98 — VIOLATION
pub struct CodeEntry {
    pub symbolic: &'static str,
    pub numeric: u16,           // ❌ Should be DiagnosticCode
    pub category: CodeCategory,
}
```

**Violation 2**: `symbolic_to_numeric` / `numeric_to_symbolic` use raw `u16`

```rust
// Line 870 — VIOLATION
pub const fn symbolic_to_numeric(symbolic: &str) -> Option<u16>  // ❌ Should be Option<DiagnosticCode>

// Line 882 — VIOLATION
pub const fn numeric_to_symbolic(numeric: u16) -> Option<&'static str>  // ❌ Should accept DiagnosticCode
```

**Violation 3**: `is_supported_code` accepts raw `u16`

```rust
// Line 895 — VIOLATION
pub const fn is_supported_code(code: u16) -> bool  // ❌ Should accept DiagnosticCode
```

**Violation 4**: `DiagnosticCode::code()` returns raw `u16`

```rust
// Line 23 — VIOLATION
pub const fn code(self) -> u16 {  // ❌ Leaks internal representation
    self.0
}
```

### Fix Prescription

```rust
// DIAGNOSTIC CODE — value object wrapping u16
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticCode(u16);

impl DiagnosticCode {
    #[must_use]
    pub const fn new(code: u16) -> Self { Self(code) }
    
    #[must_use]
    pub const fn code(self) -> DiagnosticCode {  // ✅ Returns Self
        self.0
    }
}

// CODE ENTRY — uses typed DiagnosticCode
pub struct CodeEntry {
    pub symbolic: &'static str,
    pub numeric: DiagnosticCode,  // ✅ Typed
    pub category: CodeCategory,
}

// LOOKUP — typed signature
pub const fn symbolic_to_numeric(symbolic: &str) -> Option<DiagnosticCode>
pub const fn numeric_to_symbolic(code: DiagnosticCode) -> Option<&'static str>
pub const fn is_supported_code(code: DiagnosticCode) -> bool
```

---

## 4. DDD STRUCTURAL VIOLATIONS

### Missing Value Object: `DiagnosticCode`

`DiagnosticCode` exists but is **incomplete** — its interface perpetuates primitive obsession by exposing raw `u16` through:
- `DiagnosticCode::code() -> u16`
- `DiagnosticCode::new(u16) -> Self`

### Registry as Plain Data

`CODE_REGISTRY` is a `&[CodeEntry]` slice — a dumb data container. Per Wlaschin DDD, this should be a **collection backed by a domain service** that enforces invariants.

### Mixed Concerns in Single File

| Concern | Should Be |
|---------|-----------|
| Type definitions | `types.rs` |
| CODE_REGISTRY | `registry.rs` (with lookup methods) |
| Kani harnesses | `kani_harnesses.rs` (behind feature gate) |

---

## 5. REQUIRED REFACTORS

### R1: Extract Types Module
Extract `DiagnosticCode`, `SymbolicCode`, `CodeCategory`, `CodeEntry` to `types.rs`

### R2: Extract Registry Data
Extract `CODE_REGISTRY` and lookup functions to `registry.rs`

### R3: Extract Kani Harnesses
Extract `#[cfg(kani)]` module to `kani_harnesses.rs` behind `kani` feature

### R4: Fix H2 Harness
Replace hardcoded `"__DEFINITELY_NOT_REGISTERED__"` with `kani::any::<&str>()` loop that synthesizes unregistered strings

### R5: Type-Level Seal on DiagnosticCode
```rust
// Seal the constructor — only registry can create valid DiagnosticCodes
impl DiagnosticCode {
    const fn new_unchecked(code: u16) -> Self;
    
    pub const fn from_numeric(code: u16) -> Option<Self> {
        if is_supported_code(code) {
            Some(Self(code))
        } else {
            None
        }
    }
}
```

---

## 6. VERDICT

| Metric | Score |
|--------|-------|
| File Size | 0/100 (984 >> 300) |
| Primitive Obsession | 40/100 |
| DDD Cohesion | 50/100 |
| Harness Quality | 60/100 |

**Overall**: ❌ **REJECTED** — Requires immediate refactoring before approval.

---

## 7. EVIDENCE COMMANDS

```bash
# Check line count
wc -l crates/vb_core/src/kani/kani_symbolic_code_validation.rs

# Verify kani harness compilation
cargo kani --package vb_core --harness kani_from_static_validation 2>&1 | head -50

# List all kani harnesses
rg '#\[kani::proof\]' crates/vb_core/src/kani/
```
