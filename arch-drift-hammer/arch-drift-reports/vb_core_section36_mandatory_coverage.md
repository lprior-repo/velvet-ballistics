# Architectural Drift Report: `section36_mandatory_coverage.rs`

## File Overview

| Property | Value |
|----------|-------|
| **Path** | `crates/vb_core/tests/section36_mandatory_coverage.rs` |
| **Total Lines** | 2607 |
| **Test Functions** | 123 |
| **Size Violation** | **YES** — 2607 >> 300 lines (8.7x over limit) |

---

## Drift Classification

```
SEVERITY: CRITICAL
STATUS:   REFACTOR REQUIRED
```

### Rule Violation

The architectural-drift skill enforces a **300-line hard limit** per `.rs` file.

- Lines: **2607**
- Limit: **300**
- Violation ratio: **8.7x**

---

## Structural Analysis

### Section Breakdown (22 test groups)

| Section | Topic | Est. Tests |
|---------|-------|------------|
| 1 | FiniteF64 arithmetic | 7 |
| 2 | SlotValue type_name stability | 7 |
| 3 | ConstValue::to_slot_value mapping | 6 |
| 4 | StepBudget exhaustion | 5 |
| 5 | RunFrame bounds checking | 8 |
| 6 | CompiledWorkflow::try_from_parts | 10 |
| 7 | Engine invariants | 12 |
| 8 | Resource contract validation | 7 |
| 9 | Node bounds validation | 4 |
| 10 | Transition target validation | 14 |
| 11 | CompiledWorkflow round-trip | 2 |
| 12 | Budget enforcement | 3 |
| 13 | Expression stack depth | 2 |
| 14 | Comparison operators | 6 |
| 15 | Arithmetic operators | 4 |
| 16 | Entry validation | 2 |
| 17 | Reachability | 2 |
| 18 | Forward edge validation | 1 |
| 19 | Branch route validation | 2 |
| 20 | Expression op count validation | 1 |
| 21 | Display formatters | 4 |
| 22 | Accessor/expression validation | 3 |
| 23 | Accessor root slot validation | 1 |
| 24 | Kind edges validation | 1 |
| 25 | Expression final depth validation | 2 |
| **TOTAL** | | **~100+** |

### DDD Assessment

**POSITIVE:**
- Tests are organized by domain concept (FiniteF64, SlotValue, StepBudget, RunFrame, etc.)
- Each test is independently executable
- Helper functions (`eval_expr_value`, `valid_parts`, `parts_with_contract`, `budget_parts_with_steps`) reduce duplication
- No use of `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg`

**CONCERNS:**
- File is too large for a single test module — should be split by section

---

## Recommendation

### Required Actions

1. **SPLIT THE FILE** — Divide into 22+ separate test modules:
   - `section36_finite_f64.rs`
   - `section36_slot_value.rs`
   - `section36_step_budget.rs`
   - `section36_run_frame.rs`
   - `section36_workflow_parts.rs`
   - `section36_engine_invariants.rs`
   - `section36_resource_contract.rs`
   - `section36_node_bounds.rs`
   - `section36_transition_target.rs`
   - `section36_expression_ops.rs`
   - `section36_comparison_ops.rs`
   - `section36_arithmetic_ops.rs`
   - `section36_entry_validation.rs`
   - `section36_reachability.rs`
   - `section36_branch_route.rs`
   - `section36_display_formatters.rs`
   - `section36_accessor_validation.rs`

2. **Create a integration test module** at `crates/vb_core/tests/section36_mandatory_coverage.rs` that:
   - Contains only module declarations
   - Uses `#[tokio::test]` or `#[test]` as appropriate
   - Re-exports tests from child modules

3. **Keep helper functions** in a shared `helpers.rs` module within the test directory

### Example Structure

```
crates/vb_core/tests/
  section36_mandatory_coverage.rs   # ~50 lines: module declarations only
  section36_finite_f64.rs          # ~200 lines
  section36_slot_value.rs          # ~200 lines
  section36_step_budget.rs         # ~200 lines
  section36_run_frame.rs           # ~300 lines
  ...
  section36_helpers.rs             # Shared helper functions
```

---

## Verification

After refactoring:
- Each new file must be **< 300 lines**
- Total test count must remain **123**
- All tests must pass with `cargo test --test section36_mandatory_coverage`

---

**Generated**: 2026-05-29
**Agent**: architectural-drift
