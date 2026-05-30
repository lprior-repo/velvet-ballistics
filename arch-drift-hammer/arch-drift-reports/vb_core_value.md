# Architectural Drift Report: `vb_core/src/value.rs`

## File Metrics
- **Total Lines**: 1253
- **Line Limit**: 300
- **Violation**: YES — 953 lines over limit (317% of allowed)

---

## DDD Cohesion Analysis

### Domain Types Present
| Type | Role | DDD Classification |
|------|------|-------------------|
| `Taint` | Secret propagation lattice | Value Object |
| `FiniteF64` | Finite float newtype | Value Object (NewType pattern) |
| `SlotValue` | Runtime value union | Tagged Union / Sum Type |
| `ConstValue` | Compile-time constant value | Sum Type (subset of SlotValue) |
| `SlotValueDisplay` | Lazy display formatter | Memento/DTO |
| `join_taint` | Taint lattice join operation | Domain Function |

### Cohesion Verdict: **MODERATE SMELL**
- Core domain types are cohesive — all value model concepts
- No primitive obsession detected — all primitives properly wrapped
- `Parse, don't validate` adhered to via `FiniteF64::new() -> CoreResult`
- Taint lattice properly models security propagation

---

## Violations

### 1. LINE COUNT EXCEEDED (CRITICAL)
- **Required**: ≤300 lines
- **Actual**: 1253 lines
- **Delta**: +953 lines
- **Location**: Entire file
- **Remediation**: Split into `value/types.rs` (types only) + `value/tests.rs` (inline tests)

### 2. TEST PROLIFERATION IN SOURCE MODULE (COHESION SMELL)
- **Issue**: ~950 lines of tests (lines 191–1140) co-located with production code
- **Violation**: Tests belong in `tests/` or behind `#[cfg(test)]` module in separate file
- **Pattern**: `#[cfg(test)] mod tests { ... }` inside production module is acceptable, but this is excessively large
- **Impact**: Obscures production code structure, violates single responsibility

### 3. TYPE-impl INTERLEAVING (MINOR)
- `SlotValue` impl blocks (lines 1142–1174) and `SlotValueDisplay` impl blocks (lines 1190–1253) should be adjacent
- Current structure: types → tests → impls → impls (disrupted by tests)

---

## DDD Quality Assessment

### What Works
- ✅ `Taint` is a proper value object with mathematical lattice (join_taint)
- ✅ `FiniteF64` wraps f64 with parse-not-validate semantics
- ✅ `SlotValue` and `ConstValue` are proper tagged unions
- ✅ No raw `String`, `i32` used for domain IDs — all ID types from `crate::ids`
- ✅ `#[non_exhaustive]` on public enums for future extensibility
- ✅ `#[must_use]` on pure functions
- ✅ `#![forbid(unsafe_code)]` at crate level

### Issues
- ❌ File size far exceeds single-responsibility threshold
- ❌ Tests inflate apparent complexity — production domain logic is ~200 lines

---

## Recommended Refactor

```
crates/vb_core/src/value/
├── mod.rs          # Re-exports only
├── types.rs        # Taint, FiniteF64, SlotValue, ConstValue, SlotValueDisplay (~300 lines)
└── tests.rs        # All test modules moved here (~950 lines)
```

Or alternatively, move tests to `crates/vb_core/tests/value_tests.rs` (integration tests).

---

## Priority Assessment

| Violation | Severity | Priority |
|-----------|----------|----------|
| Line count exceeded (1253 > 300) | **CRITICAL** | P0 |
| Test/code co-location | MODERATE | P1 |

**Overall**: `P0` — Must split before any further work on this module.

---

*Report generated: 2026-05-29*
*Analyzer: architectural-drift skill*
