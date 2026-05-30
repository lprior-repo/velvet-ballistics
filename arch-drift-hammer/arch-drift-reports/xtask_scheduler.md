# Architectural Drift Report: `xtask/src/scheduler.rs`

**File**: `xtask/src/scheduler.rs`  
**Total Lines**: 267  
**Status**: REFACTORED (contains active violations)

---

## 1. Line Count Assessment

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total lines | 267 | 300 | ✓ PASS |

**Note**: Within threshold, but **proptests module alone is 86 lines** — this signals cohesion violation (see §3).

---

## 2. DDD Cohesion Analysis

### Domain Structures (Cohesive)
- `ScheduleLevel` — Value object for a single scheduling level
- `Schedule` — Aggregate root for the complete build schedule
- `DepGraph` — Internal struct representing the DAG

### Domain Workflow (Cohesive)
```
build_schedule(crates, max_jobs)
  └── build_dependency_graph(crates)  → DepGraph
  └── topological_levels(&graph, max_jobs)  → Vec<ScheduleLevel>
```
Clean separation: graph construction vs. level computation.

### Verdict
**Cohesion**: WEAK due to test code density (86 lines proptests + 62 lines unit tests = 148 lines test code vs 119 lines production code = 55% test ratio).

---

## 3. Violations

### 🔴 CRITICAL — Duplicate Closing Braces (Line 251-252)

```rust
        }
    }  // ← line 252: EXTRA closing brace
}      // ← line 253: correct closing
```
**Impact**: Syntax error — code does not compile as written. This is a copy-paste artifact from `proptest_dependency_order` test closure.

---

### 🟡 MODERATE — Proptest Module Bloat

**Lines 181-267**: 86-line `proptests` module embedded in same file.

| Module | Lines | % of File |
|--------|-------|-----------|
| Production logic | 119 | 45% |
| Unit tests (`#[cfg(test)] mod tests`) | 62 | 23% |
| Proptest (`#[cfg(test)] mod proptests`) | 86 | 32% |
| **Total** | **267** | **100%** |

**Rule Violation**: `architectural-drift` enforces <300 lines AND cohesion. Proptests belong in `crates/workspace_tests/` or a sibling `tests/` directory under `xtask/`.

---

### 🟡 MODERATE — Primitive Obsession on `max_jobs`

**Line 89**: `batch.chunks(max_jobs.max(1))`

`max_jobs` is `usize` directly from function parameter. Should be wrapped:
```rust
pub struct MaxJobs(usize);
impl MaxJobs {
    pub fn new(n: usize) -> Self { Self(n.max(1)) }
    pub fn get(&self) -> usize { self.0 }
}
```

---

### 🟢 MINOR — Unclear Edge Direction in `build_dependency_graph`

**Lines 50-56**:
```rust
edges.entry(c.name.clone()).or_default().push(dep.clone());
reverse.entry(dep.clone()).or_default().push(c.name.clone());
```
- `edges[name]` = crate's dependencies (outgoing)
- `reverse[dep]` = crates that depend on `dep` (incoming)

The naming `edges`/`reverse` is backwards from convention. Consider `dependencies`/`dependents`.

---

## 4. Summary

| Category | Finding | Severity |
|----------|---------|----------|
| **Bug** | Duplicate `}` on lines 251-252 | 🔴 CRITICAL |
| **Cohesion** | 86-line proptests module in source file | 🟡 MODERATE |
| **DDD** | `max_jobs` primitive obsession | 🟡 MODERATE |
| **Clarity** | `edges`/`reverse` naming confusion | 🟢 MINOR |

---

## 5. Priority

| Priority | Action |
|----------|--------|
| **P0** | Fix duplicate brace syntax error (lines 251-252) |
| **P1** | Move proptests module to `xtask/tests/scheduler_proptests.rs` |
| **P2** | Wrap `max_jobs` in `MaxJobs` NewType |
| **P3** | Rename `edges`/`reverse` to `dependencies`/`dependents` |

---

## 6. Recommendation

**IMMEDIATE**: Fix the syntax error (P0) — the file does not compile.

**SHORT-TERM**: Extract proptests to external test file (P1) to restore cohesion.

**MEDIUM-TERM**: Apply NewType pattern for `max_jobs` (P2).

**Generated**: 2026-05-29 by architectural-drift agent
