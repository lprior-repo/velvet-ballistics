# Architectural Drift Report: `vb_core/src/workflow/tests.rs`

**File:** `crates/vb_core/src/workflow/tests.rs`
**Analyzed:** 2026-05-29
**Status:** ⚠️ DRIFT DETECTED

---

## 1. File Size Analysis

| Metric | Value |
|--------|-------|
| **Total Lines** | 4,990 |
| **Test Count** | 212 `#[test]` functions |
| **Helper Functions** | ~20 (`load`, `construction_parts`, `resource_contract`, etc.) |
| **Proptest Blocks** | 8 (`#[cfg(test)] mod proptests` with 8 parameterized tests) |
| **Size Threshold** | 300 lines (architectural rule) |
| **Drift Ratio** | **16.6× over limit** |

---

## 2. Test Organization

### Inline vs External Classification

| Criterion | Finding |
|-----------|---------|
| **Location** | `crates/vb_core/src/workflow/tests.rs` (inline) |
| **Module Declaration** | `#[cfg(test)] mod tests { ... }` inside parent `workflow` module |
| **Access Pattern** | `use super::super::validate_budget_result;` — correct superchain |
| **External Test Crate** | NO — not in `crates/workspace_tests/` |

**Verdict:** Tests are **inline**, properly accessing parent module internals via `super::` chain. This is architecturally valid but **structurally inappropriate for this size**.

---

## 3. Test Categories (Internal Structure)

| Section | Lines | Tests | Description |
|---------|-------|-------|-------------|
| `expr_program_*` | 32–81 | ~6 | Expression stack validation |
| `workflow_parts_*` | 84–413 | ~30 | Resource contract validation |
| `workflow_error_*_exact_variant` | 710–891 | ~10 | Error variant exhaustiveness |
| **Adversarial BDD tests** | 899–1413 | ~40 | Attack vectors: empty nodes, OOB, cycles |
| **Phase 46 IR structural** | 1458–2204 | ~25 | Reachability, backward edges, nesting |
| **Phase 46 adversarial** | 2206–2872 | ~30 | Symbol bounds, accessor paths, depth |
| **CompiledNodeKind 34-variant** | 2878–3323 | 35 | All `CompiledNodeKind` variant construction |
| **ExprOp construction** | 3329–3402 | ~15 | Expression operator variant tests |
| **ExprProgram construction** | 3407–3511 | ~10 | Expression program validity |
| **AccessorProgram/PathSegment** | 3517–3582 | ~8 | Accessor path construction |
| **CompiledWorkflow accessors** | 3625–3741 | ~8 | In-bounds accessor lookup |
| **Budget validation** | 3821–4063 | ~20 | Budget error mapping |
| **`mod proptests`** | 4065–4565 | 8 | Property-based: chain, slot OOB, duplicate idx, unreachable, resource bounds |
| **ResourceContract defaults** | 4527–4565 | ~8 | DEFAULT value sanity |
| **blake3 digest coherence** | 4876–4989 | 5 | B25–B27 digest tests |

---

## 4. Drift Findings

### 🚨 VIOLATION 1: File Size Exceeds 300-line Limit (16.6×)

**Rule:** Files must not exceed 300 lines (architectural-drift skill, Scott Wlaschin DDD).

**Finding:** 4,990 lines is **16.6× the limit**. This is a structural violation regardless of test quality.

**Impact:**
- Cannot be effectively reviewed in a single screen
- Violates Single Responsibility — this file does too much
- Binds validation knowledge into a single monolithic artifact

### ⚠️ VIOLATION 2: Test File is Inline Rather Than External

**Rule:** Per AGENTS.md workspace structure, tests should be in `crates/workspace_tests/` when they exceed ~1000 lines or represent a significant testing surface.

**Finding:** This is an **inline `#[cfg(test)] mod tests`** inside `workflow/mod.rs`. For a file of this size with 212 tests covering complex domain invariants, an **external integration test crate** would be more appropriate.

**Rationale:**
- External tests in `crates/workspace_tests/vb_core_workflow/` would allow parallel CI execution
- Easier to maintain separate test documentation
- Clearer boundary between "unit test" (inline) and "integration validation" (external)

---

## 5. Recommendations

### Immediate Actions

1. **Split this file** into external tests at `crates/workspace_tests/vb_core_workflow/`:
   - `tests_resource_contract.rs` — resource validation (lines 84–217)
   - `tests_ir_structural.rs` — Phase 46 graph validation (lines 1458–2204)
   - `tests_compiled_node_kinds.rs` — 34-variant construction (lines 2878–3323)
   - `tests_expr_program.rs` — expression compilation (lines 32–81, 3407–3511)
   - `tests_budget_validation.rs` — budget error mapping (lines 3821–4063)
   - `tests_blake3_digest.rs` — digest coherence B25–B27 (lines 4876–4989)

2. **Retain inline minimal tests** in `workflow/tests.rs`:
   - Keep only a representative subset (~20-30 tests) for fast inline validation
   - Move property-based tests to `proptest!` in external files

### Long-term

3. **Enforce file-size gates in CI** — reject PRs with files >300 lines (except generated code)
4. **Consider moving all 212 tests to external** — given the volume and adversarial nature, external tests in `workspace_tests/` would enable:
   - Parallel test execution
   - Separate clippy/lint lanes
   - Independent test documentation

---

## 6. Positive Observations

- ✅ Tests correctly use `use super::super::` to access parent module internals
- ✅ Comprehensive adversarial test coverage (empty nodes, OOB, cycles, nesting)
- ✅ Property-based tests (`proptest!`) for critical invariants (slot OOB, duplicate idx, unreachable nodes)
- ✅ Error variant exhaustiveness checked (`WorkflowError` display, equality)
- ✅ BDD-style naming (`workflow_parts_reject_*`, `phase46_accepts_*`, `phase46_rejects_*`)
- ✅ Helper functions properly encapsulate `WorkflowParts` construction
- ✅ No `unwrap`/`expect`/`panic` in test code — proper error propagation

---

## 7. Summary

| Attribute | Value |
|-----------|-------|
| **Lines** | 4,990 |
| **Test Count** | 212 |
| **Location Category** | INLINE (within `workflow` module) |
| **Drift Severity** | **HIGH** — 16.6× size limit exceeded |
| **Recommendation** | **MIGRATE to external tests** in `crates/workspace_tests/vb_core_workflow/` |

**Action Required:** File should be split into 6–8 external test modules; retain minimal inline subset.
