# ARCHITECTURAL DRIFT HAMMER REPORT

**File**: `crates/vb_compile/src/kani_lower_control.rs`
**Total Lines**: 376
**Line Limit**: 300
**Violation**: YES — 76 lines over limit (25.3% excess)

---

## EXECUTIVE SUMMARY

This file is a Kani proof harness collection for control-flow lowering operations (`lower_repeat`, `lower_ask`, `lower_choose`). It suffers from **severe structural violations**: primitive obsession, scattered domain logic, feature intersection, and inline business rules. The 376-line monster violates the <300 line mandate by 76 lines.

---

## 1. LINE COUNT VIOLATION

| Metric | Value |
|--------|-------|
| Actual lines | 376 |
| Maximum allowed | 300 |
| Excess | 76 lines |
| % Over limit | 25.3% |

---

## 2. RESPONSIBILITY MAPPING

### 2.1 Proof Groups

| Lines | Proof Group | Responsibility |
|-------|-------------|----------------|
| 179–204 | `lower_repeat_accepts_non_max_id_and_uses_id_plus_one_slot` | Verify non-max ID → id+1 slot mapping |
| 206–225 | `lower_repeat_rejects_max_id_without_overflow` | Verify max ID rejection |
| 227–251 | `lower_ask_accepts_non_max_id_and_uses_id_plus_one_resume` | Verify non-max ID → id+1 resume |
| 253–278 | `lower_ask_rejects_max_id_without_overflow` | Verify max ID rejection |
| 287–340 | `lower_choose_fanout_bound` | PO-001 H1: 64/65 fanout limit enforcement |
| 342–374 | `lower_choose_live_api_has_fanout_check` | PO-001 H2: Public API fanout check |

### 2.2 Helper Assertion Functions

| Lines | Function | Domain Concept |
|-------|----------|----------------|
| 57–73 | `assert_repeat_nodes` | Repeat node tri-plate assertion |
| 75–95 | `assert_repeat_start` | RepeatStart kind verification |
| 97–120 | `assert_repeat_attempt` | RepeatAttempt kind verification |
| 122–130 | `assert_repeat_finish` | RepeatFinish kind verification |
| 132–147 | `assert_ask_nodes` | Ask node bi-plate assertion |
| 149–166 | `assert_ask_start` | Ask kind verification |
| 168–177 | `assert_ask_resume` | AskResume kind verification |

### 2.3 Symbolic Generators

| Lines | Function | Purpose |
|-------|----------|---------|
| 10–19 | `symbolic_non_max_step_raw` | Generate non-max u16 step raw |
| 21–29 | `expected_successor` | Compute id+1 with overflow guard |
| 31–39 | `max_step_plus_one` | Compute u16::MAX + 1 as usize |
| 41–47 | `symbolic_step`, `symbolic_slot` | Generate arbitrary StepIdx/SlotIdx |
| 49–55 | `symbolic_timeout` | Generate optional SlotIdx |

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### PO-001: Magic Number - Fanout Limit 64/65

**Location**: Lines 292–306, 346–351

```rust
(0..65).map(...).collect()  // 65 = rejected
(0..64).map(...).collect()  // 64 = accepted
```

**Problem**: The fanout limit is a **business rule** encoded as raw integer literals. No named constant explains why 64 is the limit and 65 is rejected.

**Fix**: Extract to a domain constant:
```rust
/// Maximum allowed branches in a choose expression.
/// 64 is the architectural limit for the fanout tree.
const MAX_CHOOSE_BRANCHES: usize = 64;
```

---

### PO-002: Magic Number - MAX_NON_OVERFLOWING_STEP_RAW

**Location**: Line 8

```rust
const MAX_NON_OVERFLOWING_STEP_RAW: u16 = 65_534; // u16::MAX - 1
```

**Problem**: `65_534` is `u16::MAX - 1` but the meaning is opaque. Why `-1`? This is `StepIdx`'s domain boundary for id+1 safety.

**Fix**: Derive from domain type:
```rust
const MAX_NON_OVERFLOWING_STEP_RAW: u16 = StepIdx::MAX.raw() - 1;
```

---

### PO-003: Inline Overflow Arithmetic in Symbolic Generation

**Location**: Lines 21–29

```rust
fn expected_successor(raw: u16) -> u16 {
    match raw.checked_add(1) {
        Some(value) => value,
        None => {
            kani::assert(false, "non-max step id must have an id + 1 successor");
            0
        }
    }
}
```

**Problem**: The id+1 successor relationship is a **domain invariant** that should be encapsulated in a type, not scattered across symbolic generators.

**Fix**: Add `StepIdx::successor()` method that preserves the domain contract.

---

### PO-004: Scattered kani::assert Without Domain Wrappers

**Location**: Throughout lines 82–177 (all `assert_*` functions)

Every assertion uses raw `kani::assert(condition, "message")` with stringly-typed messages. No domain-specific assertion combinators.

**Problem**: Violates "make illegal states unrepresentable" — the string messages are not machine-checkable invariants.

---

### PO-005: Raw u16 in Symbolic Generation

**Location**: Lines 41–47

```rust
fn symbolic_step() -> StepIdx {
    StepIdx::new(kani::any::<u16>())
}
```

**Problem**: `kani::any::<u16>()` bypasses `StepIdx`'s domain constraints. A valid `StepIdx` should be constructed through a domain-safe constructor that guarantees validity.

---

### PO-006: id+1 Slot Mapping Without Domain Type

**Location**: Lines 183–185

```rust
let successor_raw = expected_successor(id_raw);
let expected_slot = SlotIdx::new(successor_raw);
```

**Problem**: The id→slot relationship (id+1 mapping) is a **workflow contract** but encoded as raw arithmetic. No type expresses "this slot is the successor of that step."

---

## 4. SCOTT WLASCHIN DDD VIOLATIONS

### SW-001: Feature Intersection — One File, Three Concerns

This file attempts to be:
1. A **Kani proof harness** for `lower_repeat`
2. A **Kani proof harness** for `lower_ask`
3. A **Kani proof harness** for `lower_choose`

**Violation**: DDD Bounded Context principle. Each lowering operation belongs in its own verification context.

**Fix**: Split into three files:
```
vb_compile/src/
  kani_repeat_lower.rs   # lower_repeat proofs only
  kani_ask_lower.rs      # lower_ask proofs only  
  kani_choose_lower.rs   # lower_choose proofs only
```

---

### SW-002: Data Clump — Repeat Parameter Clusters

**Location**: Lines 57–73

```rust
fn assert_repeat_nodes(
    nodes: &[CompiledNode],
    id: StepIdx,
    max_attempts: u16,
    body: StepIdx,
    done: StepIdx,
    expected_slot: SlotIdx,
)
```

`id`, `body`, `done`, `expected_slot` travel together but are not unified into a domain concept.

**Fix**: Create a `RepeatControl` value object:
```rust
struct RepeatControl {
    id: StepIdx,
    body: StepIdx,
    done: StepIdx,
    expected_slot: SlotIdx,
    max_attempts: u16,
}
```

---

### SW-003: Feature Intersection — Assertion Functions Mix Levels

**Lines 57–177**: Assertion helpers mix:
- Node structure verification (`assert_repeat_nodes`)
- Kind discrimination (`assert_repeat_start`)
- Field equality checks (`assert_repeat_start` inside match)

These should be separate: structural tests vs. domain contract tests.

---

### SW-004: Hidden Business Rule — Fanout Limit

**Location**: Lines 289–340

The fanout limit of 64 is a **domain rule** but appears only as integer literals. No comment explains why 64.

**Fix**: Add module-level documentation:
```rust
/// Fanout limit for `lower_choose`.
/// Architectural constraint: branch indices encoded in 6-bit payload.
/// Exceeding this limit returns `CompileError::PrimitiveLoweringLimitExceeded`.
const MAX_CHOOSE_BRANCHES: usize = 64;
```

---

## 5. FILE STRUCTURE VIOLATION

### 5.1 Module Organization

```
Lines 1–8:     Module header + imports
Lines 10–55:  Symbolic generator helpers
Lines 57–177: Assertion helper functions  ← 120 lines of helpers
Lines 179–278: lower_repeat + lower_ask proofs
Lines 280–374: lower_choose proofs + EOF reference
```

The 120-line assertion helper block (lines 57–177) is a **god module** smell. These helpers support three different lowering operations but are co-located.

---

## 6. PRESCRIPTIVE REFACTORING

### 6.1 Split Thresholds

| File | Target Lines | Content |
|------|--------------|---------|
| `kani_repeat_lower.rs` | ~150 | Repeat proof + helpers |
| `kani_ask_lower.rs` | ~150 | Ask proof + helpers |
| `kani_choose_lower.rs` | ~130 | Choose proof + helpers |
| `kani_control_shared.rs` | ~80 | Shared symbolic generators + domain constants |

### 6.2 Domain Constants to Extract

```rust
// In kani_control_shared.rs
use vb_core::{CompiledNode, CompiledNodeKind, SlotIdx, StepIdx};

/// Maximum non-overflowing step raw value (StepIdx::MAX - 1).
/// Exists because StepIdx::MAX cannot produce StepIdx::MAX + 1.
const MAX_NON_OVERFLOWING_STEP_RAW: u16 = StepIdx::MAX.raw() - 1;

/// Fanout limit for choose expressions.
/// Architectural constraint: branch index fits in 6 bits.
const MAX_CHOOSE_BRANCHES: usize = 64;

/// Architectural fanout limit + 1 for rejection testing.
const CHOOSE_BRANCHES_OVER_LIMIT: usize = 65;
```

### 6.3 Domain Types to Introduce

```rust
/// Domain marker: this slot is the successor (id+1) of a step.
/// Embodies the "slot index = step index + 1" workflow invariant.
struct StepSuccessor(SlotIdx);

/// Domain value object for repeat control flow parameters.
struct RepeatControl {
    id: StepIdx,
    body: StepIdx,
    done: StepIdx,
    expected_slot: StepSuccessor,
    max_attempts: u16,
}

/// Domain value object for ask control flow parameters.
struct AskControl {
    id: StepIdx,
    prompt: SlotIdx,
    answer: SlotIdx,
    timeout_slot: Option<SlotIdx>,
    expected_resume: StepSuccessor,
}
```

---

## 7. VERDICT

| Violation | Severity | Count |
|-----------|----------|-------|
| Line count | **CRITICAL** | 1 (76 lines over) |
| Primitive obsession | **HIGH** | 6 |
| Feature intersection | **HIGH** | 2 |
| DDD data clump | **MEDIUM** | 1 |
| Hidden business rules | **MEDIUM** | 1 |

**Overall**: This file requires aggressive refactoring. The 376-line monolith must be split into at least 4 focused files with proper domain types extracted. The fanout limit business rule must be named and documented.

**Recommended Action**: Immediate refactor into `kani_repeat_lower.rs`, `kani_ask_lower.rs`, `kani_choose_lower.rs`, and `kani_control_shared.rs`. Extract `StepSuccessor` and control parameter value objects. Introduce named domain constants for all magic numbers.

---

*Report generated by arch-drift-hammer | velvet-ballistics | 2026-05-29*
