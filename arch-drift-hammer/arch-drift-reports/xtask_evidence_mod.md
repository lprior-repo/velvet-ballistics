# Architectural Drift Report: `xtask/src/evidence.rs`

**File:** `/home/lewis/src/velvet-ballistics/xtask/src/evidence.rs`  
**Date:** 2026-05-29  
**Analyzer:** architectural-drift agent

---

## Summary

| Metric | Value | Status |
|--------|-------|--------|
| Total Lines | 142 | ✅ PASS (< 300) |
| DDD Cohesion | LOW | ❌ VIOLATION |
| File Size Drift | None | ✅ PASS |
| Priority | **Medium** | — |

---

## DDD Cohesion Analysis

### Problem: Mixed Domain Concepts

This file violates **Single Responsibility Principle** and **DDD cohesion** by aggregating **multiple unrelated domain concepts** under one facade:

| Line | Concept | Domain Layer |
|------|---------|--------------|
| 5–11 | `Rect` struct | Geometry/Rendering |
| 14–17 | `LayoutKernelError` | Geometry/UI Layout |
| 22–26 | `SelectedIndicator` | UI State |
| 42–110 | Geometry functions (`overlap_area_px`, `rect_contains`, etc.) | Geometry/Rendering |
| 128–142 | `include!` modules (release_contract, bundle, validators, etc.) | Evidence Packaging |

### Smell: **Facade Aggregator Pattern**

The file uses 15 `include!` statements (lines 3, 128–142) to pull in disparate concerns:
- `release_contract.rs` — UI release contract
- `release_validation.rs` — Validation logic
- `bundle.rs` — Artifact bundling
- `error_profile_domain.rs` — Error profiling domain
- `parsed_documents.rs`, `raw_documents.rs` — Document handling
- `fixture_parsers.rs` — Test fixture parsing
- `profile_runner.rs` — Profile execution
- `release_model.rs`, `artifact_facts.rs` — Release modeling
- `release_validators.rs`, `release_rendering.rs` — Validation/rendering
- `negative_fixtures.rs` — Negative testing
- `persistence.rs` — Persistence concerns
- `tests.rs` — Test harness

**This is a structural violation**: A module should not be a passive conduit for `include!` statements. It should expose cohesive, named abstractions.

---

## Violations

| ID | Violation | Severity | Location |
|----|-----------|----------|----------|
| V1 | **Facade Aggregator**: 15 `include!` statements bypass module boundaries | Medium | Lines 3, 128–142 |
| V2 | **Mixed Domain Concepts**: Geometry primitives (`Rect`, `LayoutKernelError`) co-located with evidence packaging | Medium | Lines 5–126 |
| V3 | **Dead Code Allowance**: `#![allow(dead_code]` suggests incomplete integration | Low | Line 1 |
| V4 | **Inconsistent Abstraction**: Geometry domain has no clear owner in `xtask` context | Low | Lines 5–110 |

---

## DDD Smell Classification

**Smell Type:** **Kernel/Utility Confusion**

The `Rect` struct and geometry functions (`overlap_area_px`, `rect_right`, `rect_bottom`, etc.) suggest a **Layout Kernel** bounded context, but this file lives in `xtask/src/evidence.rs`, which implies **Evidence Packaging** bounded context.

**This is an architectural boundary violation**: Geometry domain concepts should live in their own crate (e.g., `vb_geometry` or `vb_layout_kernel`), not mixed into evidence tooling.

---

## Recommendations

1. **Extract Geometry Domain**: Move `Rect`, `SelectedIndicator`, `LayoutKernelError`, and all geometry functions to `crates/vb_geometry` or a dedicated `xtask/src/geometry/` submodule.

2. **Eliminate Facade Pattern**: Remove the `include!` statements. Each included module should be properly imported via `mod` declarations with public re-exports if needed.

3. **Remove Dead Code Allowance**: Investigate why `dead_code` is allowed and either integrate the code or remove it.

4. **Establish Module Ownership**: Assign clear bounded contexts:
   - **Evidence Packaging** → `xtask/src/evidence/` (release contracts, validators, artifact facts)
   - **Geometry/Layout Kernel** → `crates/vb_geometry` or `xtask/src/geometry/`

---

## Verdict

**ARCHITECTURAL DRIFT DETECTED** — The file exhibits **low cohesion** due to mixed domain concepts and a facade aggregator pattern. While file size is acceptable, the structural organization violates Scott Wlaschin's DDD principle of **"one domain per module, cohesive exports."**

**Priority: Medium** — The violations do not cause build failures but represent structural debt that will compound as the codebase grows.
