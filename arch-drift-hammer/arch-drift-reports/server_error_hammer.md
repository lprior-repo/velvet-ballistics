# Architectural Drift Report: `vb_ipc/src/server/error.rs`

**Agent**: architectural-drift enforcer
**File**: `/home/lewis/src/velvet-ballistics/crates/vb_ipc/src/server/error.rs`
**Original Line Count**: 447 lines
**Limit**: 300 lines
**Status**: 🚨 DRIFT DETECTED — REFACTOR REQUIRED

---

## Executive Summary

| Category | Finding | Severity |
|----------|---------|----------|
| Line Count | 447 lines (149% of limit) | CRITICAL |
| Test Distribution | 348/447 lines (77.8%) are tests | HIGH |
| Primitive Obsession | `std::io::Error` used directly without NewType | HIGH |
| Manual Derive | `PartialEq` manually implemented instead of derived | MEDIUM |

---

## 1. Line Count Violation

**Rule**: All `.rs` files MUST be ≤300 lines.

**Reality**: This file is **447 lines** — **149% of the allowed size**.

**Breakdown**:
- Enum + impl blocks: ~97 lines (21.7%)
- Tests: 348 lines (77.8%)
- Blank/comments: ~2 lines (0.4%)

---

## 2. Primitive Obsession Violations

### 2.1 `std::io::Error` Without NewType Wrapper

**Location**: Lines 12-15, 18-21, 24-27, 36-39

```rust
#[error("bind failed: {source}")]
BindFailed {
    /// Underlying IO error.
    source: std::io::Error,
},
```

**Problem**: `std::io::Error` does NOT implement `PartialEq` or `Eq`. This forces:

1. Manual `PartialEq` implementation (lines 54-77)
2. Comparing `io::Error` by `kind()` + `to_string()` instead of by identity

**Scott Wlaschin DDD Violation**: `std::io::Error` is a primitive — a framework type with no domain meaning. It should be wrapped in a domain-typed NewType like `IoError` that:
- Implements `PartialEq`, `Eq`, `Hash`
- Carries semantic intent (which phase of the server lifecycle failed)

### 2.2 Suggested NewType

```rust
/// Domain-typed IO error with proper equality semantics.
#[derive(Debug, Eq, Hash)]
pub struct IoError {
    kind: std::io::ErrorKind,
    message: String,
}

impl From<std::io::Error> for IoError {
    fn from(e: std::io::Error) -> Self {
        Self { kind: e.kind(), message: e.to_string() }
    }
}

impl PartialEq for IoError {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.message == other.message
    }
}
```

---

## 3. Manual `PartialEq` Implementation

**Location**: Lines 54-77 (23 lines of boilerplate)

```rust
impl PartialEq for IpcServerError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::BindFailed { source: a }, Self::BindFailed { source: b }) => {
                a.kind() == b.kind() && a.to_string() == b.to_string()
            }
            // ... 7 more arms ...
        }
    }
}
```

**Problem**: This manual implementation exists ONLY because `std::io::Error` lacks `PartialEq`. With a proper `IoError` NewType wrapper that implements `PartialEq`, this entire block becomes derivable.

**Correct Form**:
```rust
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum IpcServerError { ... }
```

---

## 4. Test Extraction Required

**Location**: Lines 99-447

The test module is **348 lines** — larger than the entire allowed file budget for non-test code. Per the architectural rules, tests should be in `tests/` subdirectory or a sibling test file.

**Proposed Structure**:
```
vb_ipc/src/server/error.rs     (~99 lines: enum + impl + runtime_code)
vb_ipc/src/server/error/tests.rs   (~348 lines: all tests)
```

---

## 5. Domain Model Analysis (POSITIVE)

Despite the violations above, the core DDD modeling is sound:

| Aspect | Assessment |
|--------|------------|
| Error enum variants | ✅ Well-named, distinct failure modes |
| Error categorization | ✅ Structural vs operational errors properly separated |
| `runtime_code()` method | ✅ Good domain mapping to Section 17 codes |
| `diagnostic_code()` delegation | ✅ Proper bridge to `IpcError` |
| Layering | ✅ `IpcServerError` (server) vs `IpcError` (frame/payload) are properly separate |

---

## 6. Refactoring Prescription

### Phase 1: Extract Tests (Eliminates ~348 lines)

Create `vb_ipc/src/server/error/tests.rs` with all test code. Update `error.rs` to:
```rust
#[cfg(test)]
mod tests;
```

### Phase 2: Create `IoError` NewType

Add to `vb_ipc/src/server/error.rs` (or a new `io_error.rs` submodule):

```rust
/// Domain-typed IO error with proper equality semantics.
#[derive(Debug, Clone, Eq, Hash)]
pub struct IoError {
    kind: std::io::ErrorKind,
    message: String,
}

impl From<std::io::Error> for IoError {
    fn from(e: std::io::Error) -> Self {
        Self { kind: e.kind(), message: e.to_string() }
    }
}

impl PartialEq for IoError {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.message == other.message
    }
}

impl std::error::Error for IoError {}
```

### Phase 3: Derive `PartialEq` on `IpcServerError`

Change:
```rust
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IpcServerError {
    BindFailed { source: IoError },  // Not std::io::Error
    PollFailed { source: IoError },
    // ...
}
```

Remove manual `PartialEq` impl (lines 54-77).

---

## 7. Success Metrics

| Metric | Before | After |
|--------|--------|-------|
| Line count | 447 | ~99 |
| Test lines in file | 348 | 0 |
| Manual PartialEq | Yes (23 lines) | No (derived) |
| Primitive obsession | `std::io::Error` | `IoError` NewType |

---

## 8. Risk Assessment

- **Low**: Test extraction is mechanical
- **Low**: `IoError` NewType is a pure wrapper with no behavioral change
- **Low**: `#[derive(PartialEq)]` is behavior-preserving (same equality semantics)

---

## Verdict

**DRIFT STATUS**: VIOLATED

**Required Actions**:
1. Extract 348 lines of tests to sibling test file
2. Create `IoError` NewType wrapper
3. Derive `PartialEq` instead of manual implementation
4. Target: ≤300 lines, zero primitive obsession, zero manual trait impls

**Estimated Post-Refactor Size**: ~110 lines (well under 300)

---

*Generated by: architectural-drift enforcer*
*Date**: 2026-05-29*
