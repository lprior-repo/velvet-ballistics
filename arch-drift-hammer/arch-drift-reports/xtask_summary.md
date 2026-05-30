# Architectural Drift Report: `xtask/src/summary.rs`

**File**: `xtask/src/summary.rs`  
**Total Lines**: 175  
**Status**: PERFECT (no refactoring required for line count)

---

## 1. Line Count Check

| Metric | Value | Threshold | Result |
|--------|-------|-----------|--------|
| Total Lines | 175 | 300 | ✅ PASS |

---

## 2. DDD Cohesion Analysis

### Domain Entities
| Entity | Fields | Assessment |
|--------|--------|------------|
| `LaneResult` | `crate_name`, `lane`, `status`, `duration_ms` | Data carrier (anemic) |
| `RunSummary` | `run_id`, `results` | Aggregate root with query methods |

### Behavior Boundary
- `format_summary`, `format_text_summary`, `format_json_summary` → Presentation layer (output formatting)
- `group_by_crate`, `status_icon` → Pure presentation utilities
- `RunSummary::{pass_count, fail_count, skip_count, total_duration_ms, has_failures}` → Domain queries on aggregate

### Cohesion Verdict
Cohesion is **acceptable**. The file has a single, clear responsibility: human-readable summary output for xtask proof runs. All functions are related to this purpose.

---

## 3. Violations

### ❌ Primitive Obsession: `status: String` (HIGH)

The field `status: String` encodes a closed union type (`"pass"`, `"fail"`, `"skip"`, `"timeout"`, `"dry-run"`) as a raw string.

**Problems**:
1. Invalid strings like `"pas"` or `"failed"` are accepted at compile time
2. `status_icon()` performs validation via string matching with no compiler enforcement
3. `pass_count()`, `fail_count()`, `skip_count()` all use string comparison: `r.status == "pass"`

**Refactor Recommendation**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneStatus {
    Pass,
    Fail,
    Skip,
    Timeout,
    DryRun,
}
```

### ⚠️ Primitive Obsession: `crate_name`, `lane`, `run_id` as `String` (LOW)

These are identifier strings that could be newtypes, but the violation is less severe since they're not union types. Acceptable for this boundary layer.

---

## 4. DDD Smell Assessment

| Smell | Severity | Present |
|-------|----------|---------|
| String typing for closed union | HIGH | Yes (`status`) |
| Anemic domain model | LOW | Partial (`LaneResult`) |
| Validation at boundary | MEDIUM | Yes (`status_icon` fallback to "?") |

**Primary Smell**: String-typed enum for `LaneStatus` — the most common DDD smell in Rust.

---

## 5. Priority

| Issue | Priority | Effort |
|-------|----------|--------|
| Convert `status: String` to `LaneStatus` enum | MEDIUM | Low (refactor in place) |

**Recommendation**: The file is well-structured at 175 lines. The primary improvement would be replacing the `status: String` field with a proper `LaneStatus` enum. This would:
- Make illegal states unrepresentable
- Eliminate runtime string comparison in query methods
- Improve `status_icon` to a simple `match` on the enum

However, since this is an output/display module (xtask), the current implementation is **functional and acceptable**. No mandatory refactoring required.

---

**Report Generated**: 2026-05-29  
**Analyzer**: architectural-drift skill
