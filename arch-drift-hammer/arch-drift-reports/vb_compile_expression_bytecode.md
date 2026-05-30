# Architectural Drift Report: `expression_bytecode.rs`

**File**: `crates/vb_compile/src/expression_bytecode.rs`  
**Analyzer**: architectural-drift skill  
**Date**: 2026-05-29

---

## 1. Line Count

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | **2533** | 300 | **FAIL** (744% over) |
| Production code | ~552 | 300 | **FAIL** (184% over) |
| Test code | ~1981 | N/A (should be separate) | **FAIL** |

---

## 2. DDD Cohesion Analysis

**Domain Concept**: Expression bytecode lowering (postfix VM instruction generation)

**Filename Verdict**: `expression_bytecode.rs` ✓ correctly reflects a single domain concept

**Cohesion Smell**: **YES** — The module violates single-responsibility by mixing:
- Core bytecode compilation logic (lines 1–551)
- Inline test suite (lines 553–2533)

---

## 3. Violations

### V-01: FILE SIZE EXCEEDED (CRITICAL)
- **Lines**: 2533 total (requirement: <300)
- **Production code**: ~552 lines
- **Test code**: ~1981 lines
- **Location**: Entire file
- **Remediation**: Split into `expression_bytecode.rs` (prod) + `expression_bytecode_tests.rs` (tests)

### V-02: INLINE TESTS MODULE (CRITICAL)
- **Lines**: 553–2533 (1981 lines of `#[cfg(test)] mod tests`)
- **Problem**: Tests are embedded within the production module rather than in a separate file
- **Impact**: Violates module separation principle; test code pollutes production module visibility
- **Remediation**: Move to `expression_bytecode/tests/` subdirectory or `expression_bytecode_tests.rs`

### V-03: PRODUCTION CODE STILL EXCEEDS SIZE LIMIT
- **Lines**: ~552 lines (still 184% of 300-line limit)
- **Location**: Lines 1–551 (excluding test module)
- **Problem**: Even with tests extracted, production code alone exceeds 300-line limit
- **Functions exceeding 30 lines**:
  - `lower_expr` (lines 353–370): 18 lines — acceptable
  - `lower_literal` (lines 381–400): 20 lines — acceptable
  - `lower_unary` (lines 402–417): 16 lines — acceptable
  - `lower_binary` (lines 432–444): 13 lines — acceptable
  - `lower_helper` (lines 446–459): 14 lines — acceptable
  - `parse_field_path_segments` (lines 253–273): 21 lines — marginal
- **Remediation**: Split production code into submodules:
  - `expression_bytecode/resolver.rs` — resolver traits and implementations
  - `expression_bytecode/lowering.rs` — core lowering functions
  - `expression_bytecode/reference.rs` — reference parsing helpers

### V-04: MISSING MODULE SEPARATION
- **Problem**: No subdirectory structure under `expression_bytecode`
- **Expected structure**:
  ```
  expression_bytecode/
    mod.rs           # Re-exports
    lowering.rs      # Core bytecode lowering
    resolver.rs      # Reference resolvers  
    reference.rs     # Reference parsing
    tests.rs         # OR tests/ subdirectory
  ```
- **Current**: Single monolithic file

---

## 4. Function Complexity Analysis

| Function | Lines | Complexity | Status |
|----------|-------|------------|--------|
| `compile_expr_to_bytecode` | 5 | O(1) | ✓ |
| `compile_expr_to_bytecode_with_accessors` | 9 | O(1) | ✓ |
| `compile_expr_to_bytecode_with_step_slots` | 14 | O(1) | ✓ |
| `compile_expr_to_bytecode_with_resolver` | 9 | O(1) | ✓ |
| `lower_slot_reference` | 12 | Low | ✓ |
| `lower_step_reference` | 30 | Low | ✓ |
| `lower_expr` | 18 | Low | ✓ |
| `lower_literal` | 20 | Low | ✓ |
| `lower_helper` | 14 | Low | ✓ |
| `parse_field_path_segments` | 21 | Low | ✓ |
| `validate_helper_arity` | 11 | Low | ✓ |

**No oversized functions detected** — all functions are under 30 lines.

---

## 5. Remediation Priority

| Priority | Violation | Effort | Impact |
|----------|-----------|--------|--------|
| **P0** | Move inline tests to separate file | Medium | Unblocks CI, restores module hygiene |
| **P0** | Split production code into submodules | Medium | Reduces each module to <300 lines |
| **P1** | Create `expression_bytecode/resolver.rs` | Low | Extracts resolver trait + 3 implementations |
| **P1** | Create `expression_bytecode/reference.rs` | Low | Extracts 5 parsing helpers |
| **P2** | Create `expression_bytecode/lowering.rs` | Low | Core lowering functions |

---

## 6. Recommended File Structure

```
crates/vb_compile/src/
├── expression_bytecode/
│   ├── mod.rs              # Re-exports + public API
│   ├── lowering.rs         # ~180 lines: lower_expr, lower_literal, lower_unary, lower_binary, lower_helper
│   ├── resolver.rs         # ~120 lines: ExpressionReferenceResolver trait + 3 implementations
│   ├── reference.rs        # ~150 lines: slot/step reference parsing + lowering
│   └── tests/
│       ├── lowering_tests.rs        # ~650 lines
│       ├── resolver_tests.rs        # ~200 lines  
│       └── reference_tests.rs       # ~200 lines
└── expression_bytecode.rs   # Removed (code moved to submodules)
```

**Note**: Update `lib.rs` to use the new module structure.

---

## 7. Summary

| Check | Status |
|-------|--------|
| Total lines < 300 | ❌ FAIL (2533 lines) |
| DDD cohesion | ⚠️ SMELL (mixed concerns) |
| Function sizes | ✓ PASS |
| Module separation | ❌ FAIL (inline tests) |
| No primitive obsession | ✓ PASS (uses typed indices) |
| Parse don't validate | ✓ PASS |

**DDD Smell Detected**: YES

**Files Needing Creation**: 2–4 new files  
**Files Needing Modification**: `lib.rs` (module updates) + `expression_bytecode.rs` (deletion)  
**Estimated Refactor Time**: 2–3 hours

---

*Report generated by architectural-drift skill*
