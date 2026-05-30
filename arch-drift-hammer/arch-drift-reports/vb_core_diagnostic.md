# Architectural Drift Report: `vb_core/src/diagnostic.rs`

**File**: `crates/vb_core/src/diagnostic.rs`
**Analysis Date**: 2026-05-29
**Agent**: architectural-drift

---

## Executive Summary

| Metric | Value | Status |
|--------|-------|--------|
| Total Lines | 2449 | ❌ EXCEEDS 300 LIMIT |
| DDD Cohesion | LOW | ❌ God Module |
| Remediation Priority | **CRITICAL** | Restructure immediately |

---

## 1. Line Count Violation

**Limit**: 300 lines
**Actual**: 2449 lines
**Overage**: 2149 lines (716% over limit)

The file is **8x larger** than the maximum allowed size.

### Line Breakdown
| Section | Lines |占比 |
|---------|-------|-----|
| CODE_REGISTRY (data) | ~1100 | 45% |
| Tests | ~390 | 16% |
| Type definitions + impls | ~959 | 39% |

---

## 2. DDD Cohesion Analysis

### God Module (Anti-pattern)

This file is a **classic God Module** containing **14 distinct domain concepts** crammed into a single file:

1. `CodeCategory` — enum for grouping codes
2. `CodeEntry` — registry entry struct
3. `CODE_REGISTRY` — 1100+ line constant data array
4. `SymbolicCode` — primary stable identifier (value object)
5. `DiagnosticCode` — numeric encoding (value object)
6. `Diagnostic` — user-facing record (entity)
7. `Severity` — diagnostic severity level (enum)
8. `SymbolicCodeParseError` — parse error
9. `DiagnosticCodeParseError` — parse error
10. `HasSymbolicCode` — trait for error types
11. Lookup functions (6+ helpers)
12. Serialization impls (serde)
13. Helper functions
14. Unit tests (390 lines)

### Bounded Context Leakage

The file aggregates diagnostic codes from **14 different bounded contexts**:

| Context | Category Range | Concerns |
|---------|---------------|----------|
| Schema | E01xx | YAML/schema validation |
| Reference | E02xx | Reference resolution |
| ControlFlow | E03xx | Workflow control |
| TypeTaint | E04xx | Type checking, secrets |
| Gate | E05xx | Verification gates |
| ContractDiscovery | E06xx | Schema versioning |
| Compilation | E10xx | Compiler internals |
| WorkflowIr | E11xx | IR validation |
| Expression | E12xx | Expression errors |
| Accessor | E13xx | Path/access errors |
| Lowering | E14xx | Compilation lowering |
| Lifecycle | E15xx/E33xx | Lifecycle transitions |
| Storage | E20xx | Journal/runtime storage |
| RuntimeBoundary | E40xx | Fjall journal boundary |

### Mixed Concerns

- **Serde serialization** (`Serialize`, `Deserialize` impls) mixed directly with domain types
- **Data (CODE_REGISTRY)** mixed with **logic (lookup functions)**
- **Tests** inline with production code (violates file-size constraint indirectly)

---

## 3. All Violations

### Critical

| # | Violation | Rule/Boundary |
|---|-----------|---------------|
| 1 | **File size 2449 >> 300** | `<300 line files` |
| 2 | **God Module** | DDD cohesion: one concept per file |
| 3 | **CODE_REGISTRY data clump** (~1100 lines) | Should be generated or in separate data module |
| 4 | **14 bounded contexts in one file** | Context boundary drift |

### Major

| # | Violation | Rule/Boundary |
|---|-----------|---------------|
| 5 | **Duplicate category mapping** — `0x15` and `0x33` both map to `Lifecycle` | `category_from_numeric` inconsistency |
| 6 | **Serde in core domain** | Infrastructure leakage |
| 7 | **Tests inline in production module** (~390 lines) | Should be in `tests/` or behind `#[cfg(test)]` gate at bottom |

### Minor

| # | Violation | Rule/Boundary |
|---|-----------|---------------|
| 8 | `category_from_numeric` fallback heuristics bypass registry | Hidden coupling |
| 9 | `Diagnostic::new` has `// SAFETY:` comment but file forbids `unsafe` | Comment inconsistency |
| 10 | `INTERNAL_INVARIANT_VIOLATION` numeric `0x1309` conflicts with Accessor range `0x1301-0x1315` | Numeric space collision |

---

## 4. DDD Smell Assessment

**Primary Smell**: **God Module / Feature Envy**

The file exhibits multiple overlapping smells:

1. **God Module**: 14 concepts should be 14 files minimum
2. **Data Clump**: CODE_REGISTRY is 1100 lines of raw data
3. ** shotgun surgery**: Changing any category requires editing this single file
4. **Parallel Inheritance**: Each category has its own code range but is not isolated
5. **Lazy Class**: Some impl blocks are thin wrappers around registry lookups

**Cohesion Score**: 2/10 (Very Low)

---

## 5. Remediation Priority

### Phase 1: Immediate (File Size Reduction)

| Action | Target |
|--------|--------|
| Extract `CODE_REGISTRY` to generated file | `-800 lines` |
| Extract tests to `tests/diagnostic_tests.rs` | `-390 lines` |
| Extract serde impls to `diagnostic_serde.rs` | `-100 lines` |
| **Resulting size** | **~1159 lines (still over)** |

### Phase 2: Structural Splitting

Split into at minimum:

```
vb_core/src/diagnostic/
├── mod.rs           # Re-exports, traits
├── code_category.rs    # CodeCategory enum
├── symbolic_code.rs     # SymbolicCode value object
├── diagnostic_code.rs  # DiagnosticCode value object
├── diagnostic.rs       # Diagnostic record + Severity
├── registry.rs         # CODE_REGISTRY + lookup functions (data + logic)
├── codes/
│   ├── schema.rs       # E01xx entries
│   ├── reference.rs    # E02xx entries
│   ├── control_flow.rs # E03xx entries
│   ├── type_*.rs       # ... (one file per category)
│   └── storage.rs      # E20xx, E40xx entries
└── parse_errors.rs     # Parse error types
```

### Phase 3: Numeric Space Cleanup

Fix `0x1309` collision between `INTERNAL_INVARIANT_VIOLATION` and Accessor range.

---

## 6. Recommendations

1. **Generate CODE_REGISTRY**: Use a build script or codegen to produce the registry from a separate data file (YAML/JSON/TOML), reducing this file by ~800 lines
2. **Move tests**: Place all tests behind `#[path = "diagnostic_tests.rs"]` mod or in integration tests
3. **Extract serde**: Create `diagnostic_serde.rs` for Serialize/Deserialize impls
4. **Enforce 300-line CI gate**: Add `cloc` or line-count check to `moon ci`
5. **Fix numeric collision**: Relocate `INTERNAL_INVARIANT_VIOLATION` to `0xFFFF` or similar non-conflicting space

---

## Verdict

**ARCHITECTURAL DRIFT: CONFIRMED**

This file is in a state of severe structural decay. It violates the `<300 line` hard limit by 8x and represents a textbook God Module anti-pattern. Immediate refactoring is required before further development can proceed on this module.

**Estimated Refactoring Effort**: 3-5 beads (medium complexity, high impact)
