# Architectural Drift Report: `engine/types.rs`

**File**: `crates/vb_runtime/src/engine/types.rs`
**Total Lines**: 1180
**Violation**: EXCEEDS 300-line limit by 293% (3.93x over budget)

---

## Executive Summary

This file is a **CATASTROPHIC** architectural violation. At 1180 lines, it is nearly **four times** the mandated 300-line ceiling. The file violates every principle of Scott Wlaschin's DDD type-driven design and exhibits severe primitive obsession throughout.

**CRITICAL**: The test module (lines 314–1180) is **866 lines** — the actual implementation is only ~200 lines of production code hidden inside a wall of test bloat. Tests should live in a separate `tests/` directory or behind a `#[cfg(test)]` module that is **no more than 2x** the implementation it tests.

---

## Type Definitions Mapped

| Type | Lines | Category | Primitive Obsession |
|------|-------|----------|---------------------|
| `EvidenceEvent` | 16–48 | Enum (3 variants) | None — GOOD |
| `EvidenceCollector` | 55–200 | Struct + impl | `usize` for capacity/dropped |
| `RuntimeEngineResult<T>` | 202–203 | Type alias | None — GOOD |
| `RuntimeEngineError` | 205–267 | Enum (5 variants) | None — GOOD |
| `RetryPolicy` | 269–294 | Struct | `u16` for max_attempts, `u64` for base_delay_ms |
| `RuntimeSignal` | 296–312 | Enum (6 variants) | None — GOOD |

---

## Violation 1: FILE SIZE (CRITICAL)

**Rule**: No production source file may exceed 300 lines.

**Status**: FAIL — 1180 lines detected.

**Required Split**:

```
engine/types/              ← new directory
├── mod.rs                 ← ~10 lines, re-exports
├── evidence.rs            ← EvidenceEvent + EvidenceCollector (~200 lines)
├── error.rs               ← RuntimeEngineError + RuntimeEngineResult (~65 lines)
├── retry.rs               ← RetryPolicy (~30 lines)
└── signal.rs              ← RuntimeSignal (~20 lines)
```

Tests stay in `tests/engine_types_tests.rs` at workspace root **OR** each module keeps its own `#[cfg(test)]` block capped at 2x implementation size.

---

## Violation 2: PRIMITIVE OBSESSION IN `RetryPolicy`

**Current**:

```rust
pub struct RetryPolicy {
    pub max_attempts: u16,      // ← primitive obsession
    pub base_delay_ms: u64,     // ← primitive obsession
    pub exponential_backoff: bool,
}
```

**Required DDD Decomposition**:

```rust
/// Bounded attempt count (1–65535).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaxAttempts(u16);

/// Millisecond duration for retry delays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryDelayMs(u64);
```

**Impact**: Raw `u16` for `max_attempts` allows values like `0` which are semantically invalid for retry. The `MaxAttempts` wrapper enforces validity at the type level.

---

## Violation 3: PRIMITIVE OBSESSION IN `EvidenceCollector`

**Current**:

```rust
pub struct EvidenceCollector {
    events:   Vec<EvidenceEvent>,
    capacity: usize,   // ← primitive obsession
    dropped:  usize,    // ← primitive obsession
}
```

**Required DDD Decomposition**:

```rust
/// Event count, always ≤ EvidenceCapacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventCount(usize);

/// Bounded evidence buffer capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceCapacity(usize);
```

---

## Violation 4: TEST INFLATION (CRITICAL)

**Current**: 866 lines of tests for ~200 lines of implementation (4.33x ratio).

**Scott Wlaschin Rule**: Tests should prove behavior, not inflate LOC metrics.

**Required Action**:
1. Move tests to `crates/vb_runtime/tests/engine_types_tests.rs`
2. Keep `#[cfg(test)]` blocks in each module for unit tests only (< 2x impl size)
3. Integration/behavior tests belong in `crates/workspace_tests/`

**Current Test Ratio Per Module**:
| Module | Impl Lines | Test Lines | Ratio |
|--------|------------|------------|-------|
| EvidenceCollector | ~130 | ~550 | 4.2x **FAIL** |
| RuntimeEngineError | ~60 | ~200 | 3.3x **FAIL** |
| RetryPolicy | ~25 | ~50 | 2x **BORDERLINE** |
| RuntimeSignal | ~15 | ~50 | 3.3x **FAIL** |

---

## Violation 5: CONSTANTS LIVING IN SOURCE FILE

**Current**:
```rust
const REQUIRED_COLLECT_SLOT_EXTRA: &str = "collect SlotWritten extra";  // line 14
const DEFAULT_EVIDENCE_CAPACITY: usize = 3 * 1024;                         // line 53
```

**DDD Principle**: Constants that are **namespaced to a type** should live inside that type as associated constants.

**Required Refactor**:
```rust
impl EvidenceCollector {
    pub const DEFAULT_CAPACITY: usize = 3 * 1024;
}

impl RuntimeEngineError {
    pub const REQUIRED_COLLECT_SLOT_EXTRA: &str = "collect SlotWritten extra";
}
```

---

## Summary of Required Refactors

| # | Type | Violation | Priority |
|---|------|-----------|----------|
| 1 | File | 1180 lines vs 300 max | **CRITICAL** |
| 2 | `RetryPolicy` | `u16`, `u64` primitives | HIGH |
| 3 | `EvidenceCollector` | `usize` primitives | HIGH |
| 4 | Tests | 866 lines (4.3x impl) | **CRITICAL** |
| 5 | Constants | Globals not namespaced | MEDIUM |

---

## Files to Create

```
crates/vb_runtime/src/engine/types/
├── mod.rs           (re-exports)
├── evidence.rs     (EvidenceEvent + EvidenceCollector + constants)
├── error.rs        (RuntimeEngineError + RuntimeEngineResult)
├── retry.rs        (RetryPolicy)
└── signal.rs       (RuntimeSignal)

crates/vb_runtime/tests/
└── engine_types_tests.rs   (migrated tests)
```

---

## Primitive Obsession Scorecard

| Type | Field | Raw Type | Fix |
|------|-------|----------|-----|
| `RetryPolicy` | `max_attempts` | `u16` | `MaxAttempts(u16)` |
| `RetryPolicy` | `base_delay_ms` | `u64` | `RetryDelayMs(u64)` |
| `EvidenceCollector` | `capacity` | `usize` | `EvidenceCapacity(usize)` |
| `EvidenceCollector` | `dropped` | `usize` | `DroppedCount(usize)` |
| `EvidenceCollector` | `len()` | `usize` | `EventCount(usize)` |

---

## Enforcement Actions

1. **IMMEDIATE**: File MUST be split into `engine/types/` module directory
2. **HIGH**: Newtype wrappers required for all primitive obsession violations
3. **CRITICAL**: Tests MUST be migrated to `tests/` directory with ratio ≤ 2x
4. **MEDIUM**: Constants must become associated constants on their types
5. ** gate**: `moon ci` must reject any file > 300 lines after refactor

---

*Report generated by architectural-drift agent. All findings are binding.*
