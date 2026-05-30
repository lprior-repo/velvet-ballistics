# Architectural Drift Report: `vb_expr/stack_ops.rs`

## File
- **Path**: `crates/vb_expr/src/stack_ops.rs`
- **Lines**: 56 (LIMIT: 300) ✅

## DDD Cohesion Analysis

### Cohesion Score: HIGH
This module exhibits **high cohesion** — all 5 functions serve a single, well-defined purpose: stack operation primitives for a bounded stack-based expression evaluator.

| Function | Responsibility | Boundary |
|----------|----------------|----------|
| `push_value` | Push value onto evaluation stack | Stack Ops |
| `pop_value` | Pop single value from stack | Stack Ops |
| `pop_pair` | Pop pair (left, right order) | Stack Ops |
| `expect_bool` | Type-check SlotValue → bool | Type Coercion |
| `expect_i64` | Type-check SlotValue → i64 | Type Coercion |

### Domain Alignment
- Stack operations are the **core domain primitive** for expression evaluation
- Error types (`ExprError::StackOverflow`, `StackUnderflow`, `TypeMismatch`) are well-typed and domain-appropriate
- Bounded stack enforced via `ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>`
- No leakage of infrastructure concerns

## Violations

| Rule | Status |
|------|--------|
| Line count ≤ 300 | ✅ PASS (56 lines) |
| No unsafe code | ✅ PASS (forbid unsafe_code) |
| No unwrap/expect/panic | ✅ PASS |
| No unchecked indexing | ✅ PASS (uses safe ArrayVec) |
| No YAML/JSON/HTTP | ✅ PASS |
| DDD cohesion | ✅ PASS |
| Parse don't validate | ✅ PASS (typed extraction with errors) |

## DDD Smells

**NONE DETECTED**

- No primitive obsession violations (SlotValue is a proper NewType)
- No anemic domain model (functions carry behavior)
- No boundary violations (stays within stack-ops boundary)
- No God module symptoms (56 lines, 5 focused functions)

## Summary

| Metric | Value |
|--------|-------|
| Lines | 56 |
| Violations | 0 |
| DDD Smells | 0 |
| Priority | **NONE** — module is architecturally clean |

**STATUS: PERFECT**
