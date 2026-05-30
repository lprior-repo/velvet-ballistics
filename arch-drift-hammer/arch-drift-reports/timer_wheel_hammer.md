# ARCH-DRIFT HAMMER REPORT
## Target: `crates/vb_runtime/src/shard/timer_wheel.rs`
## Date: 2026-05-29
## Severity: CRITICAL

---

## EXECUTIVE SUMMARY

| Violation | Count | Severity |
|-----------|-------|----------|
| Line count exceeds 300 | 348 lines (+16%) | CRITICAL |
| Primitive obsession | 4 violations | HIGH |
| Duplicate domain types | 2 entities | HIGH |
| Missing value objects | 3 types | MEDIUM |
| Test/impl co-location | 181 lines mixed | MEDIUM |

---

## 1. LINE COUNT VIOLATION (CRITICAL)

**File**: `timer_wheel.rs`
**Actual**: 348 lines
**Budget**: 300 lines
**Overflow**: 48 lines (+16%)

```
Breakdown:
  Lines 1-30:    Module docs + imports (30)
  Lines 31-37:   TimerEntry + Error type (7)
  Lines 38-46:   TimerWheel struct (9)
  Lines 48-165:  TimerWheel impl block (118)
  Lines 167-348: Tests (182)
```

**VERDICT**: OVER BUDGET BY 48 LINES. MUST SPLIT.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS (HIGH)

### 2.1 `Instant` as raw `deadline` (VIOLATION)

```rust
// Lines 27, 64, 71-73, 97, 113, 133
pub struct TimerEntry {
    pub deadline: Instant,  // RAW PRIMITIVE
}
```

**Problem**: `Instant` is a semantically-rich type used as a dumb timestamp. The domain concept "deadline" has meaning:
- A deadline in the past is semantically invalid for a *pending* timer
- Deadlines are compared, ordered, and used in arithmetic
- The domain should own this concept, not the standard library

**Scott Wlaschin Principle**: "Make illegal states unrepresentable"

**Should be**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Deadline(Instant);

impl Deadline {
    /// Creates a deadline from the given instant.
    /// Panics if the instant is in the past.
    pub fn from_instant(now: Instant, instant: Instant) -> Self { ... }

    /// Returns true if this deadline has passed relative to now.
    pub fn is_expired(self, now: Instant) -> bool { ... }

    /// Returns the underlying instant.
    pub fn as_instant(self) -> Instant { ... }
}
```

### 2.2 `u64` as raw `generation` (VIOLATION)

```rust
// Lines 25, 71, 80-87
pub struct TimerEntry {
    pub generation: u64,  // RAW PRIMITIVE
}
```

**Problem**: `generation` is a freshness token with specific semantics:
- Only increments (monotonic)
- Overflow is a domain error (`GenerationExhausted`)
- Must be compared for equality only

**Should be**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Generation(u64);

impl Generation {
    pub fn next(self) -> Result<Self, TimerWheelError> {
        self.0.checked_add(1)
            .map(Self)
            .ok_or(TimerWheelError::GenerationExhausted)
    }

    pub fn initial() -> Self { Self(1) }
}
```

### 2.3 `Vec<TimerEntry>` as raw bucket (VIOLATION)

```rust
// Lines 43, 75, 97-101, 117-125
by_deadline: BTreeMap<Instant, Vec<TimerEntry>>,  // RAW COLLECTION
```

**Problem**: The bucket is a domain concept - multiple timers can share the same deadline. The `Vec` provides no encapsulation:
- No invariant enforcement (could have duplicates)
- No empty bucket optimization hint
- No iteration semantics guarantee

**Should be**:
```rust
#[derive(Debug, Clone)]
pub struct TimerBucket {
    entries: Vec<TimerEntry>,
}

impl TimerBucket {
    pub fn insert(&mut self, entry: TimerEntry) { ... }
    pub fn remove_by_run(&mut self, run: RunId) -> bool { ... }
    pub fn is_empty(&self) -> bool { ... }
    pub fn iter(&self) -> Iter<'_, TimerEntry> { ... }
}
```

### 2.4 `#[cfg(kani)]` Map alias (ARCHITECTURAL LEAK)

```rust
// Lines 8-12
#[cfg(kani)]
use std::collections::BTreeMap as Map;
#[cfg(not(kani))]
use std::collections::HashMap as Map;
```

**Problem**: Verification infrastructure bleeds into production code. This is a build configuration concern, not a domain concern. The type selection strategy should be injected, not baked into the module.

**Should be**: The Map type should be a generic parameter or atrait with a concrete implementation selected at construction, not via module-level conditional compilation.

---

## 3. DOMAIN TYPE DUPLICATION (HIGH)

### 3.1 `PendingTimerKind` exists in `types.rs` but is redefined

**In `types.rs` (line 29-34)**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PendingTimerKind {
    Wait,
    Ask,
}
```

**In `timer_wheel.rs` (line 18)**:
```rust
use super::types::PendingTimerKind;  // ← Already imported
```

**BUT** `TimerEntry` duplicates the *concept* of `PendingTimer` from `types.rs`:

**In `types.rs` (line 36-42)**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingTimer {
    pub step: StepIdx,
    pub kind: PendingTimerKind,
    pub generation: u64,
    pub deadline: Instant,
}
```

**In `timer_wheel.rs` (line 19-30)**:
```rust
pub struct TimerEntry {
    pub run: RunId,
    pub generation: u64,
    pub deadline: Instant,
    pub kind: PendingTimerKind,
}
```

**Problem**: `PendingTimer` and `TimerEntry` are the same domain concept with different names. `PendingTimer` (from `types.rs`) has `step`; `TimerEntry` has `run`. These are two views of the same aggregate.

**Scott Wlaschin Principle**: "One concept, one name" - unify under a single type or clearly differentiate with distinct bounded contexts.

**VERDICT**: DOMAIN TYPE FRAGMENTATION. `TimerEntry` should either:
1. Be removed and use `PendingTimer` directly, or
2. Be clearly designated as the *view* of a timer for the timer wheel's specific indexing needs

---

## 4. MISSING VALUE OBJECTS (MEDIUM)

### 4.1 No `Deadline` value object
As analyzed in 2.1 - `Instant` is used raw.

### 4.2 No `Generation` value object
As analyzed in 2.2 - `u64` is used raw.

### 4.3 No `TimerFired` event type
When timers fire, the domain event is just returned as raw `Vec<TimerEntry>`. Should be:
```rust
#[derive(Debug, Clone)]
pub struct TimerFired {
    pub run: RunId,
    pub generation: Generation,
    pub deadline: Deadline,
    pub kind: PendingTimerKind,
}
```

---

## 5. TEST/IMPL CO-LOCATION (MEDIUM)

**Lines 167-348**: 181 lines of tests (52% of file)

**Problem**: Tests are interleaved with implementation. This violates the single-responsibility principle:
- The file is 52% test code
- Readers must scroll past 167 lines to reach tests
- Refactoring the module requires editing this file

**Scott Wlaschin Principle**: "Functions should do one thing" - files should have one concern.

**VERDICT**: MOVE TESTS TO `timer_wheel_tests.rs` in the same directory.

---

## 6. DUAL-INDEX LEAK (ARCHITECTURAL)

```rust
pub struct TimerWheel {
    by_deadline: BTreeMap<Instant, Vec<TimerEntry>>,  // Exposed
    by_run: Map<RunId, TimerEntry>,                   // Exposed
}
```

**Problem**: The `TimerWheel` aggregate exposes its internal indexing strategy. External code could bypass the domain methods and manipulate the indices directly.

**VERDICT**: Indices should be `pub(crate)` or encapsulated behind an invariant-enforcing interface.

---

## 7. RECOMMENDED REFACTORING

### Phase 1: Extract types to domain module
```
vb_runtime/src/shard/timer/
├── mod.rs           (re-exports)
├── wheel.rs         (TimerWheel impl - TARGET: <300 lines)
├── types.rs         (Deadline, Generation, TimerBucket, TimerEntry)
└── wheel_tests.rs   (moved tests)
```

### Phase 2: Introduce value objects
- Replace `Instant` with `Deadline`
- Replace `u64` with `Generation`
- Wrap `Vec<TimerEntry>` with `TimerBucket`

### Phase 3: Unify domain types
- Resolve `PendingTimer` vs `TimerEntry` overlap
- Either consolidate or establish clear bounded context boundary

### Phase 4: Remove cfg-gated Map alias
- Use a trait or generic parameter for map implementation selection

---

## 8. SUMMARY SCORECARD

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Lines | 348 | 300 | 🔴 FAIL (+48) |
| Value Objects | 0 | 3 | 🔴 FAIL |
| Primitive Fields | 4 | 0 | 🔴 FAIL |
| Domain Duplication | 2 types | 0 | 🔴 FAIL |
| Test Co-location | 181 lines | 0 lines | 🔴 FAIL |
| Index Encapsulation | Public | Private | 🔴 FAIL |

**OVERALL ARCH-DRIFT SCORE**: 🔴 SEVERE

---

## 9. MANDATORY ACTIONS

1. **SPLIT FILE**: Move tests to `timer_wheel_tests.rs` (immediate -16% reduction)
2. **CREATE** `Deadline` wrapper type (eliminates 1 primitive obsession)
3. **CREATE** `Generation` wrapper type (eliminates 1 primitive obsession)
4. **CREATE** `TimerBucket` type (eliminates 1 primitive obsession)
5. **RESOLVE** `PendingTimer` vs `TimerEntry` duplication
6. **PRIVATIZE** internal indices
7. **ELIMINATE** `#[cfg(kani)]` Map alias

---

*Report generated by arch-drift-hammer*
*Enforcer: architectural-drift skill*
