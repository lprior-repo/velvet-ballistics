# Architectural Drift Report: `vb_core/src/replay/tests.rs`

## File Overview

| Metric | Value |
|--------|-------|
| **File** | `crates/vb_core/src/replay/tests.rs` |
| **Total Lines** | 4,224 |
| **Test Count** | 64 |
| **Size Category** | CRITICAL — 14x over the 300-line threshold |
| **Generated** | 2026-05-29 |

---

## Drift Analysis

### 1. File Size Violation

**Threshold:** < 300 lines (architectural-drift skill mandate)  
**Actual:** 4,224 lines  
**Violation:** Yes — **1,308% over limit**

This file is a monolith. At 4,224 lines, it violates the foundational
architectural rule that files must remain ≤ 300 lines for DDD cohesion,
test isolation, and reviewability.

### 2. Test Concentration

| Category | Count |
|----------|-------|
| Unit tests (`#[test]`) | 64 |
| Helper functions | ~20+ |
| Total module complexity | Very High |

The 64 tests are densely packed with heavy boilerplate (each test
constructs `CompiledNode` graphs via `make_plan`). This creates:

- **Cognitive overload** for reviewers
- **Long recompile cycles** on single-test edits
- **Poor test isolation** — all tests compile together regardless of
  edit scope

### 3. DDD Cohesion Violation

The file mixes multiple concerns:

| Concern | Evidence |
|---------|----------|
| Core replay logic | `replay_linear_setconst_finish`, `replay_stops_at_action` |
| Blackhat security regression | `blackhat_replay_jump_cycle_exhausts_budget` (BH-RP-01) |
| Choose expression tests | `replay_choose_expr_*` (15+ tests) |
| Collect/pagination tests | `replay_collect_*` (20+ tests) |
| Error mapping | `replay_slot_error_conversion_*` |
| Taint propagation | `blackhat_replay_taint_*` (BH-RP-02, BH-RP-05) |

These belong in separate files by DDD bounded context:
- `replay/tests_basic.rs`
- `replay/tests_choose.rs`
- `replay/tests_collect.rs`
- `replay/tests_blackhat.rs`

---

## Recommendations

### Immediate Actions (Required)

1. **Split this file** into at minimum 4 separate test modules:
   - `tests.rs` (basic replay: ~400 lines, ~10 tests)
   - `tests_choose.rs` (choose expression replay: ~600 lines, ~15 tests)
   - `tests_collect.rs` (collect/pagination: ~800 lines, ~20 tests)
   - `tests_blackhat.rs` (security regressions: ~400 lines, ~8 tests)

2. **Extract shared helpers** into `tests/helpers.rs`:
   - `make_plan()`
   - `make_expr_program()`
   - `replay_err_to_core()`
   - `collect_source_frame()`
   - `current_page_values()`
   - `replay_plan_step()`
   - `collect_page_finish_plan()`

3. **Use `#[cfg(test)]` module split** at minimum:
   ```rust
   // replay/tests.rs
   mod tests_basic;
   mod tests_choose;
   mod tests_collect;
   mod tests_blackhat;
   ```

### Architectural Compliance Target

| File | Target Lines | Notes |
|------|-------------|-------|
| `replay/tests_basic.rs` | ~350 | Basic replay engine tests |
| `replay/tests_choose.rs` | ~450 | Choose expression tests |
| `replay/tests_collect.rs` | ~650 | Collect/pagination tests |
| `replay/tests_blackhat.rs` | ~350 | Security regression tests |
| `replay/tests/helpers.rs` | ~300 | Shared test utilities |
| **Total** | **~2,100** | Still over limit; further split likely needed |

---

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Test file unmaintainable | **CRITICAL** | Immediate split required |
| CI compile time inflation | **HIGH** | Modular tests compile incrementally |
| Review difficulty | **HIGH** | Smaller files review faster |
| Blackhat tests co-mingled | **MEDIUM** | Security tests need clear ownership |

---

## Conclusion

**ARCHITECTURAL DRIFT: CRITICAL**

This file is in structural violation of the DDD cohesion mandate. It must
be shredded into bounded-context-specific test modules before any further
feature work proceeds. The current form makes the replay module untestable
at the file level and blocks effective code review.
