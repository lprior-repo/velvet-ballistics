# Architectural Drift Report: `idempotency.rs`

**File**: `crates/vb_runtime/src/idempotency.rs`
**Total Lines**: 439 (exceeds 300-line limit by 46%)
**Severity**: CRITICAL
**Date**: 2026-05-29
**Enforcer**: architectural-drift agent

---

## Executive Summary

This file is a **PRIMITIVE OBSESSION HOTSPOT** and **LINE COUNT VIOLATOR**. It uses raw `u128` throughout instead of proper domain value objects, and the inline test module consumes 45% of the file's lines.

---

## Violation #1: Line Count (CRITICAL)

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 439 | 300 | **OVER BY 139 lines** |
| Production code | 232 | 300 | OK |
| Inline tests | 199 | — | **45% of file** |
| Test-to-code ratio | 86% | 50% max | **VIOLATION** |

### Root Cause
Tests are embedded inline (`#[cfg(test)]` module, lines 240-439). This is a cargo workspace — tests belong in `crates/workspace_tests/` or a dedicated `idempotency_tests.rs` file at crate root.

---

## Violation #2: Primitive Obsession (CRITICAL)

### A. `u128` Idempotency Key

**Everywhere**: `Map<u128, ActionTicket>`, `Set<u128>`, `Vec<u128>`, method signatures `fn track_for_policy(&mut self, policy: Idempotency, key: u128)`.

**Problem**: The idempotency key is a **domain concept** — it is not a raw integer. It has semantic meaning (which action instance this ticket represents) and should be wrapped in a value object.

**Current state**:
```rust
pub fn track_for_policy(&mut self, policy: Idempotency, key: u128) -> bool
pub fn is_completed_for_policy(&self, policy: Idempotency, key: u128) -> bool
pub fn mark_completed_for_policy(&mut self, policy: Idempotency, key: u128) -> Result<(), ActionError>
```

**Scott Wlaschin violation**: "Make illegal states unrepresentable." A raw `u128` can be any value including 0, MAX, or garbage. An `IdempotencyKey(u128)` with a `new()` constructor that validates (e.g., non-zero, within bounds) makes illegal values impossible to construct.

### B. Raw `usize` for Capacity and Cursor

```rust
capacity: usize,
cursor: usize,
```

**Problem**: `usize` can be 0, which is illegal for a capacity. The workaround at line 70 (`let effective_capacity = capacity.max(1)`) is a **code smell** — it patches the symptom instead of making illegal states unrepresentable at the type level.

**Fix**: Use `Capacity(NonZeroUsize)` or at minimum a `Capacity(usize)` wrapper with a private constructor that enforces invariant.

### C. Raw `Vec<u128>` for Order Tracking

```rust
order: Vec<u128>,
```

**Problem**: This is a ring-buffer implementation detail leaking into the struct. The `cursor` field combined with `order` is the eviction mechanism, but neither is encapsulated. A `RingBuffer<T>` or `EvictionTracker<T>` wrapper would separate concerns.

---

## Violation #3: DDD Cohesion Failures

### A. Policy Logic Scattered Across Multiple Methods

The `Idempotency` policy enum from `vb_core::action` is used, but the **policy-specific behavior is not encapsulated**. Compare:

**Current** (scattered policy logic):
```rust
pub fn track_for_policy(&mut self, policy: Idempotency, key: u128) -> bool {
    match policy {
        Idempotency::DeterministicPure | Idempotency::IdempotentExternal => true,
        Idempotency::AtLeastOnceExternal => { /* inline logic */ },
        _ => false,
    }
}
```

**Wlaschin DDD**: Each policy class should be a **type** or at minimum a **strategy** with a shared interface. The match arms are duplicated in `is_completed_for_policy`, `mark_completed_for_policy`, etc.

### B. Double-Bookkeeping for `AtLeastOnceExternal`

The `IdempotencyTracker` maintains TWO separate tracking structures for the same semantic concept:

1. `completed: Map<u128, ActionTicket>` — general completion
2. `at_least_once_completed: Set<u128>` — policy-specific completion

This is **redundant state** that can diverge. If `mark_completed()` is called but `mark_completed_for_policy()` is not (or vice versa), the tracker becomes inconsistent.

### C. Method Naming Confuses "Dispatched" vs "Completed" vs "Tracked"

- `mark_dispatched()` — checks if completed (not dispatched!)
- `track_for_policy()` — records a dispatch, not a completion
- `mark_completed()` — records actual completion
- `mark_completed_for_policy()` — records completion for a specific policy

**These three concepts (dispatch, track, complete) are conflated** in the API surface.

---

## Required Refactors (Priority Order)

### P0 — Split Tests Out (Line Count Fix)

Move the inline `#[cfg(test)]` module (lines 240-439) to:
- `crates/vb_runtime/src/idempotency_tests.rs` (if these are unit tests that need access to privates)
- OR `crates/workspace_tests/runtime/idempotency.rs` (if these are integration tests)

**This alone reduces the file to 239 lines** — under the 300 limit.

### P1 — Introduce `IdempotencyKey` Value Object

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdempotencyKey(u128);

impl IdempotencyKey {
    pub fn new(raw: u128) -> Self {
        Self(raw) // Future: add validation here
    }
    
    pub fn as_u128(&self) -> u128 {
        self.0
    }
}
```

Replace all `u128` key parameters with `IdempotencyKey`.

### P2 — Introduce `Capacity` Wrapper

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capacity(NonZeroUsize);

impl Capacity {
    pub fn new(n: usize) -> Option<Self> {
        NonZeroUsize::new(n).map(Self)
    }
    
    pub fn as_usize(&self) -> usize {
        self.0.get()
    }
}
```

### P3 — Extract Policy Strategies

Replace the `match policy` logic with a trait or enum-strategy pattern:

```rust
trait IdempotencyPolicy {
    fn track(&mut self, key: IdempotencyKey) -> bool;
    fn is_completed(&self, key: IdempotencyKey) -> bool;
    fn mark_completed(&mut self, key: IdempotencyKey) -> Result<(), ActionError>;
}
```

### P4 — Eliminate Double-Bookkeeping

Remove `at_least_once_completed` and use `completed` exclusively. The policy-specific check can be done by querying `completed` with the appropriate `Idempotency` context.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Key collision (raw u128) | Medium | Critical | Wrap in `IdempotencyKey` |
| Capacity=0 panic | Low | High | Use `NonZeroUsize` |
| Inconsistent state (double bookkeeping) | Medium | High | Remove `at_least_once_completed` |
| Test access to privates after split | Medium | Low | Use `#[cfg(test)]` with `super::*` in separate file |

---

## Verdict

**REFACTOR REQUIRED**. This file will not pass architectural review in its current form.

**Immediate action**: Extract tests to separate file (P0), reducing to 239 lines.

**Deferred action**: Introduce `IdempotencyKey` and `Capacity` wrappers (P1-P2) in a follow-up bead since they are API-breaking changes that require downstream coordination.

---

*Report generated by architectural-drift agent. No changes made to production code.*
