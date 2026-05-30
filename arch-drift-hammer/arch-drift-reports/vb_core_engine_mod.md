# Architectural Drift Report: vb_core engine module

**File Analyzed:** `crates/vb_core/src/engine/mod.rs`  
**Status:** FILE NOT FOUND  
**Actual File:** `crates/vb_core/src/engine.rs` (50 lines) + `engine/` directory

---

## Line Count Summary

| File | Lines | Status |
|------|-------|--------|
| `engine.rs` (root) | 50 | ✅ PASS |
| `engine/step.rs` | 1151 | ❌ VIOLATION (>300) |
| `engine/validate.rs` | 1164 | ❌ VIOLATION (>300) |
| `engine/signals.rs` | 552 | ❌ VIOLATION (>300) |
| `engine/error_routing.rs` | 485 | ❌ VIOLATION (>300) |
| `engine/object_list.rs` | 442 | ❌ VIOLATION (>300) |
| `engine/choose.rs` | 403 | ❌ VIOLATION (>300) |
| `engine/node_helpers.rs` | 321 | ❌ VIOLATION (>300) |
| `engine/run_loop.rs` | 202 | ❌ VIOLATION (>300) |

**Total engine source lines (non-test): 4,720**

---

## Violations

### 1. Line Count Violations (8 files)
Every file in `engine/` exceeds the 300-line limit:
- `step.rs` - 1151 lines (282% over limit)
- `validate.rs` - 1164 lines (288% over limit)
- `signals.rs` - 552 lines (84% over limit)
- `error_routing.rs` - 485 lines (62% over limit)
- `object_list.rs` - 442 lines (47% over limit)
- `choose.rs` - 403 lines (34% over limit)
- `node_helpers.rs` - 321 lines (7% over limit)
- `run_loop.rs` - 202 lines (under limit)

### 2. DDD Cohesion Smells

**Primitive Obsession Observations:**
- `step.rs` uses raw `usize`, `i32` for indices without newtype wrappers
- `validate.rs` uses raw integer comparisons against `MAX_*` constants
- `signals.rs` has `EngineSignal` enum with implicit state machine transitions

**Workflow Modeling:**
- State transitions in `step.rs` are implicit in match statements rather than modeled as explicit state transition functions
- `execute_node` function is 24 lines but handles too many node kinds

**Cohesion Concern:**
- `node_helpers.rs` (321 lines) is a "god module" that mixes concerns:
  - `jump_to_next`, `jump_to` (control flow)
  - `set_const`, `copy_slot` (data operations)
  - `finish_run` (terminal state)

---

## Priority Assessment

| Priority | Item | Effort |
|----------|------|--------|
| **P0-CRITICAL** | `step.rs` (1151 lines) - Must split into smaller modules | High |
| **P0-CRITICAL** | `validate.rs` (1164 lines) - Must split | High |
| **P1-HIGH** | `signals.rs` (552 lines) - Consider split | Medium |
| **P1-HIGH** | `error_routing.rs` (485 lines) - Consider split | Medium |
| **P2-MEDIUM** | Remaining files need refactoring once core split done | Medium |

---

## Recommended Splitting Strategy

### step.rs (1151 lines) Suggested Split:
1. `step/execute.rs` - `execute_node`, `execute_boundary_node`
2. `step/actions.rs` - action handling, journal operations
3. `step/signals.rs` - signal generation logic

### validate.rs (1164 lines) Suggested Split:
1. `validate/bounds.rs` - node bounds validation
2. `validate/resources.rs` - resource contract validation  
3. `validate/transitions.rs` - transition target validation
4. `validate/branches.rs` - branch target validation helpers

---

## DDD Smell Rating

**Overall DDD Cohesion:** ⚠️ MODERATE SMELL

The engine module captures a clear domain (state-machine execution), but:
- Files are too large (violates 300-line rule)
- `node_helpers.rs` lacks single responsibility
- Primitive obsession in index handling

---

*Report generated: 2026-05-29*
