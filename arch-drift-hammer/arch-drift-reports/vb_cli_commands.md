# Architectural Drift Report: `vb_cli/src/commands.rs`

**File:** `crates/vb_cli/src/commands.rs`  
**Date:** 2026-05-29  
**Agent:** architectural-drift  

---

## 1. Line Count

| Metric | Value | Status |
|--------|-------|--------|
| Total lines | **22** | ✅ PASS (< 300) |

---

## 2. DDD Cohesion Analysis

### Domain Concept Assessment

| Question | Answer | Notes |
|----------|--------|-------|
| Does filename reflect a single domain concept? | ⚠️ **PARTIAL** | `commands` is a facade module, not a domain concept |
| Is the module's responsibility cohesive? | ⚠️ **FRAGMENTED** | Only 3 of 9 command modules are re-exported |
| Does filename match actual contents? | ✅ Yes | Contents are purely re-exports |

### Facade Completeness Check

`commands.rs` re-exports from **3 modules**:
- `run.rs` → `cmd_compile`, `cmd_run`, `cmd_run_compiled`, `cmd_validate`
- `storage.rs` → `cmd_events`, `cmd_inspect`, `cmd_ipc_serve`, `cmd_replay`
- `bench.rs` → `cmd_bench_run`, `cmd_doctor`

**Total sibling `commands_*` modules in `vb_cli/src/`:**

| Module | Re-exported via `commands.rs`? | Lines |
|--------|-------------------------------|-------|
| `commands_ai_context.rs` | ❌ No | 699 |
| `commands_diff.rs` | ❌ No | 964 |
| `commands_incident.rs` | ❌ No | 227 |
| `commands_journal.rs` | ❌ No | 1157 |
| `commands_status.rs` | ❌ No | 215 |
| `commands_system_status.rs` | ❌ No | 161 |
| `commands_verify.rs` | ❌ No | 214 |
| `commands_workflow.rs` | ❌ No | 504 |
| *(facade) `commands.rs`* | ✅ Yes | 22 |
| `run.rs` | ✅ Yes | 204 |
| `storage.rs` | ✅ Yes | 295 |
| `bench.rs` | ✅ Yes | 131 |

**Conclusion:** Only 33% of command modules are surfaced through the `commands` facade.

---

## 3. Violations

### Violations in `commands.rs` (22 lines)

| Violation | Severity | Details |
|-----------|----------|---------|
| None | — | File contains only re-exports, no implementation |

### Architectural Smells (Broader vb_cli Context)

| Smell | Severity | Location | Description |
|-------|----------|----------|-------------|
| **Incomplete Facade** | 🔴 High | `commands.rs` | Facade exposes only 3/9 command modules; 6 `commands_*` modules are orphaned from the facade |
| **Fragmented Bounded Context** | 🔴 High | `commands_*.rs` files | `commands_journal.rs` (1157 lines), `commands_diff.rs` (964 lines) are monolithic; these should be decomposed |
| **Oversized Files** | 🔴 High | `commands_journal.rs`, `commands_diff.rs`, `commands_ai_context.rs` | 3 files exceed 300 lines (1157, 964, 699 respectively) |
| **Missing Module Hierarchy** | 🟡 Medium | `vb_cli/src/` | Flat structure; `commands_journal`, `commands_diff`, etc. suggest sub-domain grouping that isn't reflected in module organization |

---

## 4. Detailed Findings

### `commands.rs` Contents (22 lines)
```rust
// Pure facade - re-exports only
pub use crate::args::EmitTarget;
pub use crate::run::{cmd_compile, cmd_run, ...};
pub use crate::storage::{cmd_events, cmd_inspect, ...};
pub use crate::bench::{cmd_bench_run, cmd_doctor};
```

**No violations within the file itself.**

### Sibling Modules Referenced by Facade

| Module | Lines | Violations |
|--------|-------|------------|
| `run.rs` | 204 | None (under 300) |
| `storage.rs` | 295 | None (under 300) |
| `bench.rs` | 131 | None (under 300) |

### Orphaned Command Modules (NOT in Facade)

| Module | Lines | Concern |
|--------|-------|---------|
| `commands_journal.rs` | **1157** | Exceeds 300 line limit by 285% |
| `commands_diff.rs` | **964** | Exceeds 300 line limit by 221% |
| `commands_ai_context.rs` | **699** | Exceeds 300 line limit by 133% |
| `commands_workflow.rs` | 504 | Exceeds 300 line limit by 68% |
| `commands_incident.rs` | 227 | Under limit |
| `commands_status.rs` | 215 | Under limit |
| `commands_verify.rs` | 214 | Under limit |
| `commands_system_status.rs` | 161 | Under limit |

---

## 5. DDD Smell Assessment

| Smell Type | Detected (Yes/No) | Details |
|------------|-------------------|---------|
| **Bounded Context Fragmentation** | **Yes** | `commands.rs` facade is incomplete; 6 command modules exist outside it |
| **Primitive Obsession** | No | Not applicable (facade only) |
| **Missing Module Separation** | **Yes** | Monolithic `commands_journal.rs` (1157 lines), `commands_diff.rs` (964 lines) suggest missing sub-domain modules |
| **Siloed Domains** | **Yes** | `commands_*` modules are not integrated into the `commands` facade, suggesting disconnected CLI concerns |

---

## 6. Remediation Priority

| Priority | Action | Files Affected | Effort |
|----------|--------|----------------|--------|
| 🔴 **P0 - Critical** | Decompose `commands_journal.rs` (>300% over limit) into `commands_journal/events.rs`, `commands_journal/replay.rs`, `commands_journal/inspect.rs` | `commands_journal.rs` (1157 lines) | High |
| 🔴 **P0 - Critical** | Decompose `commands_diff.rs` (>220% over limit) into sub-modules | `commands_diff.rs` (964 lines) | High |
| 🟡 **P1 - High** | Decompose `commands_ai_context.rs` (>130% over limit) | `commands_ai_context.rs` (699 lines) | Medium |
| 🟡 **P1 - High** | Extend `commands.rs` facade to include all `commands_*` re-exports for unified public API | `commands.rs`, `lib.rs` | Low |
| 🟢 **P2 - Medium** | Decompose `commands_workflow.rs` (>68% over limit) | `commands_workflow.rs` (504 lines) | Medium |

---

## 7. Summary

| Metric | Value |
|--------|-------|
| `commands.rs` lines | 22 ✅ |
| DDD smell detected | **Yes** (Incomplete facade + Fragmented bounded context) |
| Files violating 300-line limit | **3** (`commands_journal.rs`, `commands_diff.rs`, `commands_ai_context.rs`) |
| Remediation priority | **P0** (Critical) for decomposition of 3 oversized modules |

---

**STATUS: DRIFT DETECTED — Facade incomplete, oversized files need decomposition**
