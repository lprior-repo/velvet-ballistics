# Architectural Drift Report: vb_validate/control_flow.rs

**File**: `crates/vb_validate/src/control_flow.rs`  
**Analysis Date**: 2026-05-29  
**Status**: 🔴 VIOLATIONS FOUND

---

## 1. Line Count

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | **753** | 300 | 🔴 OVER LIMIT (+453) |
| Production Code | ~146 | - | - |
| Test Code | ~607 | - | - |

**Verdict**: FAIL — File exceeds 300-line limit by 151%.

---

## 2. DDD Cohesion Analysis

### Single Responsibility Violation

| Concern | Lines | Issue |
|---------|-------|-------|
| Domain Model (`WorkflowFlow`, `StepFlow`) | 129-145 | Mixed with validation |
| Validation Logic | 14-123 | Mixed with model |
| Unit Tests (basic) | 151-327 | ~176 lines |
| BDD Tests (adversarial) | 329-753 | ~424 lines |

### Domain Model Issues

**Anemic Domain Model + Feature Envy:**
```rust
#[derive(Debug, Clone, Default)]
pub struct WorkflowFlow {
    pub steps: Vec<StepFlow>,  // Public field - no encapsulation
}

#[derive(Debug, Clone, Default)]
pub struct StepFlow {
    pub id: Option<String>,           // Primitive obsession: String for ID
    pub branch_targets: Vec<usize>,   // Raw usize indices
    pub then_target: Option<usize>,   // Raw usize
}
```

| Issue | Severity | Description |
|-------|----------|-------------|
| Public fields | 🟡 Medium | Breaks encapsulation; clients can mutate state directly |
| Primitive obsession | 🟡 Medium | `usize` for step indices lacks type safety; should be `StepIndex` NewType |
| Option<String> for ID | 🟢 Low | Defensible for optional diagnostic IDs |

### Validation Logic Issues

**Feature Envy (Martin Fowler):**
The validation functions `mark_reachable()`, `push_successors()`, and `reject_unreachable()` are tightly coupled to the `WorkflowFlow` structure, violating Data-Calc-Actions layering.

---

## 3. Violations Catalog

| ID | Category | Severity | Description |
|----|----------|----------|-------------|
| V001 | Line Count | 🔴 Critical | 753 lines exceeds 300-line hard limit |
| V002 | Feature Envy | 🟡 Medium | Validation logic envies `WorkflowFlow` internals |
| V003 | Anemic Model | 🟡 Medium | `WorkflowFlow`/`StepFlow` lack behavior; all logic in standalone functions |
| V004 | Primitive Obsession | 🟡 Medium | `usize` for indices should be `StepIndex` NewType |
| V005 | Test Bloat | 🟡 Medium | 607 test lines vs 146 production lines (4:1 ratio) |
| V006 | Public Fields | 🟡 Medium | Domain objects have mutable public fields |

---

## 4. DDD Smell Assessment

**Smell**: **Feature Envy + Anemic Domain Model**

The validation functions (calc) should be methods on `WorkflowFlow` (data), but instead they're standalone functions that access internal state. The domain objects are passive data containers.

**Alternative Pattern (Wlaschin)**: Validation functions should be in a `WorkflowFlowValidator` struct implementing `Validate<WorkflowFlow>` trait, or methods returning `ValidationResult`.

---

## 5. Priority & Remediation

| Priority | Action | Effort |
|----------|--------|--------|
| P0 | **Split file**: Move tests to `control_flow_tests.rs` | Low |
| P1 | Create `StepIndex` NewType wrapper for `usize` indices | Medium |
| P2 | Move validation to `WorkflowFlow::validate()` method or `WorkflowFlowValidator` | Medium |
| P3 | Make domain model fields private; add getters | Low |

---

## 6. Recommended File Split

```
vb_validate/src/
├── control_flow.rs      # 146 lines: model + validation
├── control_flow/
│   ├── mod.rs           # Re-exports
│   ├── model.rs         # WorkflowFlow, StepFlow (encapsulated)
│   └── validator.rs    # Validation logic (Data-Calc-Actions)
└── control_flow_tests.rs  # All tests
```

---

## Summary

| Metric | Result |
|--------|--------|
| Lines | 753 (🔴 453 over limit) |
| Violations | 6 |
| DDD Smell | Feature Envy + Anemic Domain Model |
| Priority | **P0** — Split tests immediately; consider validation refactor |
