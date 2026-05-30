# Architectural Drift Report: `signals.rs`

**File**: `crates/vb_core/src/engine/signals.rs`
**Total Lines**: 552 (violates <300 line rule by 184%)
**Drift Agent**: arch-drift-hammer
**Date**: 2026-05-29

---

## Executive Summary

This file contains two domain types (`StepBudget`, `EngineSignal`) but is massively over the line limit due to an inline test module of **437 lines** (79% of file). The production code (lines 1–114) is 206 lines and largely compliant. The primary drift is **structural**: tests belong in `tests/`, not inline.

---

## 1. LINE COUNT VIOLATIONS

| Region | Lines | Limit | Status |
|--------|-------|-------|--------|
| Production (impl + type defs) | 1–114 | 300 | ⚠️ SOFT WARN (114 < 300) |
| Inline `#[cfg(test)]` module | 115–552 | 0 (should not exist inline) | 🔴 HARD VIOLATION |
| **Total file** | **552** | **300** | 🔴 **VIOLATION** |

**Remediation**: Move all tests to `crates/vb_core/src/engine/tests/signals_tests.rs` or `crates/workspace_tests/`. Inline test modules are not permitted in production source files per workspace structure rules.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 `StepBudget::try_take` returns `Result<bool, EngineError>`

**Location**: `signals.rs:50`

```rust
pub fn try_take(&mut self) -> Result<bool, EngineError>
```

The `bool` is untyped progress semantics. Callers must remember that `true` = "took one step" and `false` = "exhausted". This is primitive obsession.

**Scott Wlaschin fix**: Use a named result type.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeResult {
    Took,        // One transition consumed
    Exhausted,   // Budget depleted
}

impl StepBudget {
    pub fn try_take(&mut self) -> Result<TakeResult, EngineError> { ... }
}
```

This creates a **zero-cost abstraction** that makes call-sites self-documenting and prevents boolean parameter confusion.

### 2.2 `EngineSignal::Finished` uses unnamed tuple

**Location**: `signals.rs:104`

```rust
Finished(SlotValue, Taint),
```

The tuple payload has no field names, so every call-site must remember ordering. This is a named-field refactor away from being a proper DDD value object.

**Fix**:
```rust
Finished {
    value: SlotValue,
    taint: Taint,
},
```

### 2.3 `StepBudgetExhausted` carries no budget context

**Location**: `signals.rs:106`

```rust
StepBudgetExhausted,
```

This is a unit variant with no information about *how many steps were taken* before exhaustion. A caller who wants to know "how far did we get" cannot recover that from the signal. This is a loss of information that belongs in the signal.

**Note**: This may be intentional if the caller tracks budget independently. If so, a comment documenting this design decision should be present.

---

## 3. DOMAIN / INFRASTRUCTURE BLEED

### 3.1 `StepBudget::from_env` mixes infrastructure into domain

**Location**: `signals.rs:81–94`

```rust
pub fn from_env() -> Result<Self, EngineError> {
    match std::env::var(Self::BENCH_LATENCY_BUDGET_US) { ... }
}
```

Parsing environment variables is **infrastructure concern**, not domain logic. `StepBudget` is a pure domain value object representing step counting. `from_env` ties it directly to the process environment, making it:
- Harder to test without `ENV` manipulation
- Violates the "functional core / imperative shell" split
- Makes `StepBudget` aware of its deployment context

**Remediation**: Move env-var parsing to a thin shell in `crates/vb_core/src/engine/mod.rs` or a dedicated `budget_env.rs` adapter. `StepBudget::from_env` should be removed.

### 3.2 `StepBudget` const definitions leak env-var names into domain

**Location**: `signals.rs:69`

```rust
const BENCH_LATENCY_BUDGET_US: &'static str = "VB_BENCH_LATENCY_BUDGET_US";
```

This constant lives in the domain module but is purely an infrastructure / benchmarking concern. It should not be visible at the `StepBudget` level.

---

## 4. SINGLE RESPONSIBILITY CONCERNS

### 4.1 `StepBudget` has two reasons to change

1. Business logic changes how step budgets are counted (e.g., new clamping strategy)
2. Benchmarking infrastructure changes env-var names or parsing

These are different stakeholders and different change rhythms. The `from_env` method should not be in `StepBudget`.

### 4.2 `EngineSignal` variants conflate terminal and suspension states

**Location**: `signals.rs:100–113`

`EngineSignal` mixes:
- Terminal outcome: `Finished(SlotValue, Taint)`
- Suspension states: `AwaitingAction`, `AwaitingWait`, `AwaitingAsk`
- Budget exhaustion: `StepBudgetExhausted`
- Continue: `Continue`

These are three different semantic categories. A caller switching on `EngineSignal` must handle all categories simultaneously, which grows unwieldy as variants expand.

**Note**: This is not necessarily a violation if the enum is intentionally a flat union of all engine outcomes. But it should be documented.

---

## 5. TEST PLACEMENT VIOLATION

### 5.1 Inline `#[cfg(test)]` module

**Location**: `signals.rs:115–552` (437 lines)

Per the workspace structure:
> `crates/workspace_tests/`: Contains all cross-crate integration tests and benchmarks. Do not place `tests/` or `benches/` at the repository root.

And implicit in DDD / clean architecture: **unit tests belong adjacent to modules but not inside them**. An inline `mod tests { ... }` bloats the production artifact (in terms of human reading) and mixes test concerns with domain concerns.

**Exceptions**: Trivial accessor tests that verify `#[test] fn it_works() { assert_eq!(x.foo(), 42) }` are sometimes acceptable inline. But 437 lines of tests — including proptest property tests — is not trivial.

**Remediation**:
```
crates/vb_core/src/engine/tests/
  signals_property_tests.rs   (proptest blocks)
  signals_unit_tests.rs      (non-property unit tests)
```

---

## 6. FINDINGS SUMMARY

| # | Category | Severity | Issue |
|---|----------|----------|-------|
| 1 | Line Count | 🔴 CRITICAL | 552 lines total; inline test module is 437 lines |
| 2 | Primitive Obsession | 🟡 MODERATE | `try_take` returns bare `bool` instead of `TakeResult` enum |
| 3 | Primitive Obsession | 🟡 MODERATE | `Finished(SlotValue, Taint)` uses unnamed tuple |
| 4 | Domain/Infrastructure Bleed | 🟡 MODERATE | `from_env` on `StepBudget` pulls std::env into domain |
| 5 | Single Responsibility | 🟡 MODERATE | `StepBudget` has two responsibilities: step counting + env parsing |
| 6 | Test Placement | 🟡 MODERATE | 437-line inline test module violates workspace structure |
| 7 | Information Loss | ⚪ INFO | `StepBudgetExhausted` carries no budget exhaustion metadata |
| 8 | Documentation | ⚪ INFO | `StepBudgetExhausted` design intent undocumented |

---

## 7. RECOMMENDED REFACTOR SEQUENCE

1. **Immediate**: Move inline `#[cfg(test)]` module to `crates/vb_core/src/engine/tests/signals.rs`
2. **P0**: Extract `from_env` from `StepBudget` into an adapter in `engine/mod.rs` or a dedicated `budget_env.rs`
3. **P0**: Replace `Result<bool, EngineError>` with `Result<TakeResult, EngineError>` (new `TakeResult` enum)
4. **P1**: Convert `Finished(SlotValue, Taint)` to `Finished { value: SlotValue, taint: Taint }`
5. **P1**: Add doc comment on `StepBudgetExhausted` explaining why it carries no budget metadata

---

## 8. COMPLIANCE SCORE

| Dimension | Score |
|-----------|-------|
| Line Count | 0 / 100 (552 vs 300 limit) |
| Primitive Obsession | 60 / 100 (partial violations) |
| Domain Purity | 70 / 100 (env-var bleed) |
| SRP Compliance | 75 / 100 (two responsibilities) |
| Test Placement | 0 / 100 (tests inline) |
| **Overall** | **41 / 100** |

**Verdict**: 🔴 **ARCHITECTURAL DRIFT CONFIRMED** — requires refactor before merge.

---

*Report generated by arch-drift-hammer. Next action: `bd ready` to create refactor beads.*
