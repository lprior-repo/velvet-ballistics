# Architectural Drift Report: `xtask/src/profiles.rs`

**File**: `xtask/src/profiles.rs`  
**Total Lines**: 107  
**Status**: ✅ UNDER 300 LINES (PERFECT)

---

## DDD Cohesion Analysis

### Cohesion Rating: **HIGH**

The file is tightly cohesive — it contains a single domain concept (`Profile`) with its associated operations:
- `Profile` enum (5 variants: Fast, Standard, Deep, ProofOnly, All)
- `lanes()` method — maps profiles to lane sets
- `parse_profile()` — parses string → Profile (Parse, don't validate ✅)
- `is_monotonic()` — validates monotonic inclusion property
- Unit tests for all functions

### Domain Concepts Identified

| Concept | Type | Status |
|---------|------|--------|
| `Profile` | Enum (Value Object) | ✅ Properly typed |
| Lane names | `&'static [&'static str]` | ⚠️ **Primitive obsession** |

---

## Violations

### 1. **Primitive Obsession — Lane Names** (MEDIUM)

**Location**: `Profile::lanes()` returns `&'static [&'static str]`

**Problem**: Lane identifiers like `"test"`, `"clippy"`, `"kani"`, `"miri"`, etc. are raw strings scattered throughout the codebase. This is classic primitive obsession — the domain concept "Lane" deserves a proper type.

**Impact**:
- No compile-time enforcement that lane names are valid
- Typos in lane strings are runtime bugs only
- Lanes appear in `parse_profile()` as string literals — adding a new lane requires string matching

**Recommendation**: Create a `Lane` enum:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lane {
    Test,
    Clippy,
    Nextest,
    Kani,
    Miri,
    Loom,
    Fuzz,
    Mutants,
    Coverage,
    Verus,
    Tla,
    Flux,
}

impl Lane {
    pub fn as_str(&self) -> &'static str { ... }
    pub fn from_str(s: &str) -> Option<Lane> { ... }
}
```

Then `Profile::lanes()` returns `&'static [Lane]` instead of `&'static [&'static str]`.

---

## DDD Smells

| Smell | Severity | Description |
|-------|----------|-------------|
| Primitive obsession | MEDIUM | Lane names as `&str` instead of `Lane` enum |

---

## Architecture Compliance

| Rule | Status |
|------|--------|
| Under 300 lines | ✅ 107 lines |
| Single responsibility | ✅ Cohesive module |
| Parse don't validate | ✅ `parse_profile` returns `Option<Profile>` |
| No `unwrap`/`expect`/`panic` | ✅ No unsafe patterns |
| No `unsafe` blocks | ✅ Clean |

---

## Priority Assessment

| Aspect | Rating |
|--------|--------|
| **Lines compliance** | ✅ PERFECT |
| **DDD smell** | ⚠️ MEDIUM (primitive obsession) |
| **Action priority** | **LOW** — file is small, cohesive, and functionally correct. The primitive obsession is a theoretical DDD concern but not causing active harm. |

---

## Conclusion

**STATUS: PERFECT** — No immediate refactoring required. The file is under 300 lines, highly cohesive, and follows functional Rust principles. The primitive obsession smell is noteworthy but low-priority given the file's simplicity and isolated use in `xtask/`.

**If refactoring**: Extract `Lane` enum to `xtask/src/lanes.rs` and update `Profile::lanes()` accordingly.
