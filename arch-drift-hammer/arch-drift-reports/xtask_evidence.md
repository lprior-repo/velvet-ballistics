# Architectural Drift Report: `xtask/src/evidence.rs`

**File**: `/home/lewis/src/velvet-ballistics/xtask/src/evidence.rs`  
**Total Lines**: 142  
**Status**: ⚠️ ATTENTION REQUIRED

---

## 1. Line Count Check

| Metric | Value | Threshold | Result |
|--------|-------|-----------|--------|
| Total lines | 142 | 300 | ✅ PASS |

---

## 2. DDD Cohesion Analysis

### Domain Boundary: `LayoutKernel`

The file defines a coherent **LayoutKernel** domain with:
- **Types**: `Rect`, `LayoutKernelError`, `LayoutKernelResult`, `SelectedIndicator`
- **Functions**: `overlap_area_px`, `rect_right`, `rect_bottom`, `rect_contains`, `is_clipped`, `is_out_of_bounds`, `chip_is_readable`, `selected_state_is_visible`

This is a **well-defined geometric domain** (rectangle operations for UI layout).

### ⚠️ CRITICAL: Modular Boundary Drift

The file uses **14 consecutive `include!` macros** at the end:

```rust
include!("evidence/release_contract.rs");
include!("evidence/release_validation.rs");
include!("evidence/tooling_and_gate_types.rs");
include!("evidence/bundle.rs");
include!("evidence/error_profile_domain.rs");
include!("evidence/parsed_documents.rs");
include!("evidence/raw_documents.rs");
include!("evidence/fixture_parsers.rs");
include!("evidence/profile_runner.rs");
include!("evidence/release_model.rs");
include!("evidence/artifact_facts.rs");
include!("evidence/release_validators.rs");
include!("evidence/release_rendering.rs");
include!("evidence/negative_fixtures.rs");
include!("evidence/persistence.rs");
include!("evidence/tests.rs");
```

**Problem**: This file acts as a **facade aggregator** for 16 unrelated modules. This violates DDD principles:
- `release_contract`, `release_validation`, `release_model`, `release_validators`, `release_rendering` are release/domain concerns
- `tooling_and_gate_types` is infrastructure
- `bundle`, `artifact_facts` are packaging concerns
- `parsed_documents`, `raw_documents`, `fixture_parsers` are parsing concerns
- `profile_runner` is execution
- `error_profile_domain` is error handling
- `negative_fixtures`, `tests` are test support
- `persistence` is storage

**These should be proper `mod` statements in `mod.rs`, not `include!` macros.**

---

## 3. Violations

| # | Violation | Type | Severity | Description |
|---|-----------|------|----------|-------------|
| 1 | **Facade Aggregator** | Modular Boundary | 🔴 HIGH | 16 `include!` macros collecting unrelated domains into one file |
| 2 | **Primitive Obsession** | DDD | 🟡 MEDIUM | `Rect { x: u32, y: u32, width: u32, height: u32 }` should use NewTypes: `X`, `Y`, `Width`, `Height` |
| 3 | **Missing Module Hierarchy** | Modular Boundary | 🟡 MEDIUM | `include!` used where `mod` + `pub mod` would provide proper visibility control |

---

## 4. DDD Smell Assessment

| Smell | Present | Evidence |
|-------|---------|----------|
| Primitive Obsession | ✅ Yes | Raw `u32` for all coordinate values |
| Feature Envy | ❌ No | Functions operate on `Rect` as intended |
| Data Class | ⚠️ Partial | `Rect` is a data class with no behavior beyond constructor validation |
| Large Class | ❌ No | File is only 142 lines |
| Shotgun Surgery | ⚠️ Yes | 16 includes suggest scattered concerns |

---

## 5. Priority & Recommendation

| Priority | Level |
|----------|-------|
| **Priority** | 🟡 MEDIUM |
| **Refactor Scope** | Modular boundary only |

### Recommended Actions

1. **Replace `include!` with proper `mod` declarations** in `xtask/src/evidence/mod.rs` or the appropriate parent module
2. **Create NewTypes** for coordinate values:
   ```rust
   struct X(u32);
   struct Y(u32);
   struct Width(u32);
   struct Height(u32);
   ```
3. **Audit the `include!` pattern** across the xtask crate to determine if this is a systemic issue

---

## 6. Evidence

```
File: xtask/src/evidence.rs
Lines: 142
Domain: LayoutKernel (geometric calculations)
Aggregated concerns: 16 include! statements
```

**Generated**: 2026-05-29  
**Agent**: architectural-drift
