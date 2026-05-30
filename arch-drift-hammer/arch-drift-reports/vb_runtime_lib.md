# Architectural Drift Report: vb_runtime/src/lib.rs

**File Analyzed:** `crates/vb_runtime/src/lib.rs`  
**Date:** 2026-05-29  
**Status:** PERFECT (file-level), DRIFT DETECTED (crate-level)

---

## 1. Line Count

| File | Lines | Limit | Status |
|------|-------|-------|--------|
| `lib.rs` | **98** | 300 | ✅ PASS |

**Verdict:** File is under 300 lines. No refactoring required at this level.

---

## 2. DDD Cohesion Analysis

### Module Exposition Map
The `lib.rs` exposes **20+ modules** in a single crate:

```
action, action_queue, admission, counters, durability_matrix, 
engine, error, frame_pool, idempotency, ipc_refinement, journal, 
primitives, recovery, runtime, shard, taint, trace
```

### Bounded Context Violations
| Smell | Evidence | Severity |
|-------|----------|----------|
| **Low Cohesion** | Unrelated domains bundled: `action`, `admission`, `counters`, `taint`, `trace` share no ubiquitous language | HIGH |
| **Primitive Obsession** | `primitives.rs` module suggests raw types still in use; no evidence of NewType wrappers | MEDIUM |
| **Workflow Diffusion** | State machine logic scattered across `runtime`, `shard`, `action`, `admission` | HIGH |
| **Anemic Domain Model** | `error.rs` only re-exported, no domain error types with state transitions | MEDIUM |

### Coupling Analysis
```
lib.rs (98 lines)
├── re-exports: RuntimeError, RuntimeResult, AskAnswer, AskTicket, ResumeError, ResumeResult, ResumeStatus
└── module declarations: 20+ modules
```

The re-exports create a **facade** but the underlying modules lack clear DDD grouping.

---

## 3. Violations

| ID | Violation | Location | Type |
|----|-----------|----------|------|
| V1 | Low Cohesion: Multiple bounded contexts in one crate | `lib.rs:50-90` | ARCHITECTURAL |
| V2 | Primitive Obsession: `primitives.rs` suggests raw types | `primitives.rs` | DOMAIN |
| V3 | Workflow Diffusion: State logic spread across modules | `runtime.rs`, `shard/`, `action.rs` | DOMAIN |
| V4 | Anemic Error Model: No domain-rich error types | `error.rs` | DOMAIN |

---

## 4. Recommendations

1. **Split into bounded contexts** (suggested):
   - `vb_runtime_core` (runtime, shard, action)
   - `vb_runtime_admission` (admission, durability_matrix, counters)
   - `vb_runtime_trace` (trace, taint, journal)

2. **NewType wrappers** for primitive IDs/handles in `primitives.rs`

3. **Domain events** instead of raw error propagation

---

## 5. Priority Assessment

| Metric | Value |
|--------|-------|
| **Priority** | **LOW** (lib.rs itself is compliant) |
| **Bead Recommendation** | Create follow-up bead for crate-level refactoring |
| **Risk** | Medium (cohesion issues may cause maintenance burden) |

---

**STATUS:** PERFECT (lib.rs), DRIFT DETECTED (crate architecture)
