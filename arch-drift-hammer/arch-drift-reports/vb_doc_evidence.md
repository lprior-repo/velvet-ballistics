# Architectural Drift Report: vb_doc::evidence

**File**: `crates/vb_doc/src/evidence.rs`
**Date**: 2026-05-29
**Agent**: architectural-drift

---

## 1. Line Count

| Metric | Value | Status |
|--------|-------|--------|
| Total lines | 99 | ✅ PASS (< 300) |
| Code lines | ~85 | - |
| Blank lines | ~10 | - |

---

## 2. DDD Cohesion Analysis

### Aggregate Identification
- **EvidenceIndex** - Aggregate root, manages collection of `EvidenceSupport`
- **EvidenceSupport** - Value object representing a single evidence claim
- **EvidenceSupportKind** - Internal enum (Cited/Pending state)
- **required_evidence()** - Factory free function

### Cohesion Score: **GOOD**
All types in this file belong to the "Evidence" domain concept. The `EvidenceIndex` holds `Vec<EvidenceSupport>`, demonstrating proper aggregate composition.

### Encapsulation
- `EvidenceIndex` - **GOOD**: `pub(crate.)` internal methods, `new()` and `from_supports()` constructors
- `EvidenceSupport` - **GOOD**: Constructors are public, internal state methods (`is_cited_by`, `is_pending_for`, `matches_sentence`) are private
- `EvidenceSupportKind` - **GOOD**: Private internal enum

---

## 3. Violations

### ❌ VIOLATION 1: Misplaced Factory Function (Boundary Drift)

**Location**: Lines 97-99
```rust
pub(crate) fn required_evidence() -> RequiredEvidence {
    RequiredEvidence::ConcreteArtifactOrPendingMarker
}
```

**Problem**: `RequiredEvidence` is defined in `lib.rs` (line 85-87), not in `evidence.rs`. This free factory function creates a value of a type defined elsewhere and belongs in the same module as `RequiredEvidence`, not in the Evidence aggregate module.

**Violation Type**: Misplaced Function (SRP violation - Evidence module doing work for parent module)
**Severity**: Low
**Impact**: Creates unnecessary cross-module dependency; `evidence.rs` imports `RequiredEvidence` from `crate::`

### ❌ VIOLATION 2: Cross-Module Import

**Location**: Line 1
```rust
use crate::RequiredEvidence;
```

**Problem**: The Evidence aggregate imports `RequiredEvidence` from the parent `lib.rs` module. This creates a semantic coupling where the Evidence aggregate depends on a type that conceptually belongs to a different layer (policy/configuration).

**Violation Type**: Feature Envy / Boundary Blur
**Severity**: Low
**Impact**: Evidence module is coupled to policy concerns it shouldn't own

---

## 4. DDD Smell Assessment

| Smell | Present | Notes |
|-------|---------|-------|
| Misplaced factory function | ✅ | `required_evidence()` should be in `lib.rs` |
| Feature envy | ✅ | Evidence creating `RequiredEvidence` values |
| Data class | ❌ | `EvidenceSupport` has behavior (is_cited_by, etc.) |
|shotgun surgery | ❌ | Changes would be localized |
| Parallel hierarchies | ❌ | Clean inheritance |

**Overall DDD Grade**: B+ (Minor organizational drift only)

---

## 5. Positive Observations

- ✅ No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg`
- ✅ No YAML/JSON/HTTP in runtime core
- ✅ Proper `pub(crate.)` visibility boundaries
- ✅ Immutable data structures preferred
- ✅ Small, focused types with single responsibility
- ✅ Good method naming (`is_cited_by`, `is_pending_for`, `matches_sentence`)

---

## 6. Priority Assessment

| Dimension | Rating |
|-----------|--------|
| **Priority** | **LOW** |
| Correctness Impact | None (code compiles and works) |
| Maintenance Burden | Low (cosmetic organizational issue) |
| Refactor Risk | Low (single small function move) |

---

## 7. Recommended Fix

Move the `required_evidence()` function to `lib.rs` alongside `RequiredEvidence` enum (around line 87). Remove the `use crate::RequiredEvidence;` import from `evidence.rs` after this fix.

**Before** (evidence.rs):
```rust
use crate::RequiredEvidence;
// ... evidence types ...

pub(crate) fn required_evidence() -> RequiredEvidence {
    RequiredEvidence::ConcreteArtifactOrPendingMarker
}
```

**After** (lib.rs after line 87):
```rust
pub fn required_evidence() -> RequiredEvidence {
    RequiredEvidence::ConcreteArtifactOrPendingMarker
}
```

---

## 8. Files Affected

| File | Change |
|------|--------|
| `crates/vb_doc/src/evidence.rs` | Remove `required_evidence()` fn and `use crate::RequiredEvidence;` |
| `crates/vb_doc/src/lib.rs` | Add `required_evidence()` fn near `RequiredEvidence` enum |

---

**Report ID**: vb_doc_evidence_2026-05-29
**Next Action**: Low-priority cleanup; not blocking
