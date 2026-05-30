# Architectural Drift Report: `xtask/src/loom.rs`

**File**: `xtask/src/loom.rs`  
**Total Lines**: 106  
**Threshold**: 300  
**Status**: ✓ PASS (under threshold)

---

## 1. Line Count

| Metric | Value |
|--------|-------|
| Total lines | 106 |
| Threshold | 300 |
| Status | **PASS** |

---

## 2. DDD Cohesion Analysis

### Cohesion Score: HIGH (for xtask infrastructure)

**Domain Concept**: Loom model execution command (build-time concurrency testing)

**Functions**:
- `cmd_loom(model: &str)` - orchestrates running a loom model test
- `find_model(model: &str)` - resolves model name to filesystem path
- `list_models()` - displays available loom models

**Static Data**:
- `LOOM_MODELS` - catalogue of `[(&str, &str)]` mapping model names to descriptions
- `VB_RUNTIME_PATH` - hardcoded workspace path

**Cohesion Assessment**: All functions operate on the single domain concept of "loom models". The module is tightly focused.

---

## 3. Violations

### Violation 1: Magic String Constant for Workspace Path
- **Severity**: LOW
- **Location**: Line 15, `VB_RUNTIME_PATH`
- **Issue**: `const VB_RUNTIME_PATH: &str = "crates/vb_runtime";` couples this xtask to a specific workspace layout
- **Rationale**: For xtask build tooling, this is acceptable. Production code would warrant a config type.

### Violation 2: Primitive Obsession - Model Names as `&str`
- **Severity**: LOW
- **Location**: Line 29, `find_model(model: &str)`
- **Issue**: Model names are raw `&str` instead of a `ModelName` newtype
- **Rationale**: For xtask CLI wrapper, this is acceptable. Production domain code would require a newtype.

### Violation 3: No Workflow State Machine
- **Severity**: INFORMATIONAL
- **Location**: `cmd_loom` function
- **Issue**: The function is straight-through procedural code with no explicit state transitions
- **Rationale**: For a simple command wrapper, this is appropriate. Complex orchestration would warrant state modeling.

### Violation 4: Direct Process Spawning
- **Severity**: INFORMATIONAL
- **Location**: Lines 39-44
- **Issue**: `std::process::Command` spawning is inline with no abstraction
- **Rationale**: Acceptable for xtask; production would use a command executor trait

---

## 4. DDD Smell Assessment

| Smell | Level | Notes |
|-------|-------|-------|
| Primitive obsession | MILD | Acceptable for xtask infrastructure |
| Magic strings | MILD | `VB_RUNTIME_PATH` is workspace coupling |
| Anemic domain | LOW | Functions are actually cohesive, just thin |
| Workflow clarity | N/A | Simple command wrapper, state machine overkill |

**Overall DDD Smell**: **LOW**

This is build infrastructure (`xtask`), not production domain code. The simplicity is appropriate.

---

## 5. Priority

| Priority | Rationale |
|----------|-----------|
| **LOW** | File is small (106 lines), cohesive, and is xtask infrastructure |

---

## 6. Recommendation

**No refactoring required.** This file is appropriate xtask infrastructure code. It:

1. Is under the 300-line threshold
2. Has clear, single-responsibility functions
3. Uses acceptable primitives for CLI/build tooling
4. Would not benefit from DDD rich types given its purpose

**Status**: `PERFECT`
