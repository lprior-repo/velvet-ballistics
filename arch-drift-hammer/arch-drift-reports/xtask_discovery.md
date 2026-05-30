# Architectural Drift Report: `xtask/src/discovery.rs`

**File**: `xtask/src/discovery.rs`  
**Analyzed**: 2026-05-29

---

## 1. Line Count

| Metric | Value | Status |
|--------|-------|--------|
| Total Lines | 125 | ✅ PASS (< 300) |

---

## 2. DDD Cohesion Analysis

### Module Purpose
Single-responsibility discovery module for workspace crate enumeration via `cargo metadata`.

### Public API Surface

| Function | Responsibility | Cohesion |
|----------|---------------|----------|
| `CrateInfo` (struct) | Value object representing a discovered crate | ✅ Cohesive |
| `discover_crates()` | Entry point: run metadata + parse | ✅ Cohesive |
| `filter_crates()` | Filter crates by include/exclude patterns | ✅ Cohesive |

### Internal Helpers

| Function | Responsibility |
|----------|---------------|
| `run_cargo_metadata()` | Executes `cargo metadata` command |
| `parse_crates()` | Filters workspace members from metadata |
| `pkg_to_crate_info()` | Transforms `Package` → `CrateInfo` |
| `matches_any()` | Pattern matching for filtering |

### Cohesion Verdict
**HIGH** — All functions serve the single "crate discovery" bounded context. No mission creep.

---

## 3. Violations

### Violation 1: Primitive Obsession (Low Severity)
**Location**: `CrateInfo` fields, `filter_crates` signatures

```rust
// CURRENT (primitive obsession)
pub struct CrateInfo {
    pub name: String,           // Raw String for crate name
    pub manifest_path: PathBuf,
    pub dependencies: Vec<String>,
}

pub fn filter_crates(
    crates: &[CrateInfo],
    include: Option<&[String]>,  // Raw String slices
    exclude: Option<&[String]>,
) -> Vec<CrateInfo>
```

**Issue**: `String` is used directly instead of domain newtypes like `CrateName`, `FilterPattern`.

**Recommendation** (non-blocking for xtask):
```rust
#[derive(Debug, Clone)]
pub struct CrateName(String);

#[derive(Clone)]
pub struct FilterPattern(Vec<String>);
```

### Violation 2: Anemic Domain Model (Low Severity)
**Location**: `CrateInfo`

`CrateInfo` is a pure data bucket with no behavior. In a true DDD model, value objects should encapsulate validation/logic.

**Current**: Fields are `pub` with no invariants enforced.

### Violation 3: Procedural Filtering (Informational)
**Location**: `filter_crates` + `matches_any`

The filtering logic is procedurally composed rather than modeled as a domain filter type. Acceptable for xtask utility.

---

## 4. DDD Smell Assessment

| Smell | Severity | Present |
|-------|----------|---------|
| Primitive Obsession | Low | Yes (`String` for names) |
| Anemic Domain Model | Low | Yes (CrateInfo is data-only) |
| Feature Envy | None | No |
| Shotgun Surgery | None | No |
| Long Method | None | No (max 15 lines) |
| God Object | None | No |

**Overall DDD Smell**: MINIMAL — File is a well-structured utility, not a rich domain model. Acceptable for xtask scaffolding.

---

## 5. Priority Assessment

| Issue | Priority | Effort |
|-------|----------|--------|
| Primitive obsession | **LOW** | High (API change) |
| Anemic model | **LOW** | Medium |
| Report cleanliness | **N/A** | — |

**Recommendation**: No refactoring required. File is compliant with architectural rules.

---

## Summary

| Check | Result |
|-------|--------|
| Line count < 300 | ✅ 125 lines |
| DDD cohesion | ✅ HIGH |
| Violations | 2 minor (non-blocking) |
| DDD smell | MINIMAL |
| **Priority** | **NONE — PERFECT** |

**STATUS**: PERFECT
