# Architectural Drift Report: `vb_core_replay_step_tests`

## File Under Analysis
- **Path**: `crates/vb_core/src/replay/step_tests.rs`
- **Crate**: `vb_core`
- **Module**: `replay`

## Metrics

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total Lines | 2166 | 300 | ❌ VIOLATION (7.2x) |
| Test Count | 35 | N/A | INFO |

## Location Category

**Inline Unit Test** (`src/`-resident test module)

- Valid under Rust convention for unit tests testing internal implementation details
- Lives alongside `replay/step.rs` (implementation) and `replay/mod.rs` (module boundary)
- Not an integration test (those belong in `crates/workspace_tests/`)
- Not a benchmark (those belong in `benches/` or `crates/workspace_tests/`)

## Architectural Drift Findings

### 1. File Size Violation (CRITICAL)

The file is **2166 lines**, exceeding the **300-line limit** by **1866 lines** (720% over threshold).

**Impact**:
- Cognitive overload for code reviewers
-git merge conflict surface area
- Slower compilation for incremental changes
- Violates SCOTT WLASCHIN DDD cohesion principle (one concept per file)

### 2. DDD Cohesion Assessment

The file is **cohesive** — it tests a single bounded context: `ReplayStep` behavior across all `CompiledNodeKind` variants.

**Test organization by step kind** (evidence of good cohesion):
- Nop step (2 tests)
- SetConst step (4 tests)
- Copy step (4 tests)
- EvalExpr step (1 test)
- BuildObject step (5 tests)
- BuildList step (4 tests)
- Finish step (2 tests)
- Jump step (1 test)
- Suspend steps: Do, Ask, WaitUntil, WaitEvent (4 tests)
- ChooseSlot step (2 tests)
- Multi-step counter (1 test)
- Error propagation (5 tests via `replay_*_reports_*_failures`)

**No primitive obsession detected**: All IDs use newtypes (StepIdx, SlotIdx, RunId, etc.)

## Recommendation

**REFACTOR REQUIRED** — Split into multiple files by step kind:

```
src/replay/
├── step_tests.rs          # 2166 lines → DELETE
├── step/
│   ├── mod.rs
│   ├── tests_nop.rs      # ~150 lines
│   ├── tests_set_const.rs # ~200 lines
│   ├── tests_copy.rs      # ~250 lines
│   ├── tests_eval_expr.rs # ~100 lines
│   ├── tests_build_object.rs # ~300 lines
│   ├── tests_build_list.rs # ~250 lines
│   ├── tests_finish.rs    # ~100 lines
│   ├── tests_jump.rs      # ~80 lines
│   ├── tests_suspend.rs   # ~200 lines
│   ├── tests_choose_slot.rs # ~150 lines
│   └── tests_error_propagation.rs # ~250 lines
```

Each resulting file should be **< 300 lines** with tests for one `CompiledNodeKind` variant.

## Status

```
STATUS: DRIFT_DETECTED
SEVERITY: HIGH
ACTION: SPLIT_FILE_BY_STEP_KIND
```
