# ARCHITECTURAL DRIFT REPORT: kani_choose_replay.rs

**File**: `crates/vb_core/src/kani_choose_replay.rs`
**Total Lines**: 340
**Line Limit**: 300
**Violation**: YES — 40 lines over limit

---

## 1. LINE COUNT VIOLATION

| Metric | Value |
|--------|-------|
| Actual | 340 lines |
| Limit | 300 lines |
| Overflow | +40 lines |

---

## 2. DUPLICATE FILE ANOMALY

**IDENTICAL COPY EXISTS AT**: `crates/vb_core/src/verification/kani/kani_choose_replay.rs`

Both files have the same 340-line content. This is architectural waste and a maintenance hazard. One copy must be canonical; the other is dead weight.

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### 3.1 Raw Numeric Primitives in Proof Setup

| Location | Primitive | Domain Meaning |
|----------|-----------|----------------|
| Lines 60-61, 123-124, 170-171, etc. | `u16 = 16` | Slot count bound |
| Lines 60-61, 123-124, 170-171, etc. | `u16 = 200` | Step count bound |
| Line 70 | `u16` | Slot index before wrapping |
| Line 311 | `u8` | Branch count |
| Line 307 | `128u16` | Hardcoded slot initialization bound |
| Line 318 | `u16::from(i)` | Branch index construction |

### 3.2 Magic Numbers Not Using Domain Constants

The file uses hardcoded literals instead of `limits.rs` constants:

```rust
// SHOULD use: crate::limits::{MAX_SLOTS_PER_STEP, MAX_STEPS_PER_WORKFLOW}
const SLOT_COUNT: u16 = 16;           // Magic number
const STEP_COUNT: u16 = 200;           // Magic number  
const MAX_BRANCHES: u8 = 64;          // Magic number
const SLOT_INIT_BOUND: u16 = 128;     // Magic number
```

### 3.3 Bounded Generators Take Raw Primitives

```rust
// VIOLATION: Takes raw u16 max, should take a bounded domain type
fn any_slot_idx(max: u16) -> SlotIdx {
    let raw = kani::any::<u16>();
    kani::assume(raw < max);
    SlotIdx::new(raw)
}
```

---

## 4. DDD VIOLATIONS (Scott Wlaschin)

### 4.1 No Value Objects for Bounds

The file manipulates raw `u16` and `u8` primitives for:
- Slot counts
- Step counts  
- Branch counts

**Should exist**: `SlotCount(u16)`, `StepCount(u16)`, `BranchCount(u8)` — wrapped with validation invariants.

### 4.2 Primitive Construction in Test Harnesses

```rust
// VIOLATION: Raw primitive construction scattered across proofs
let true_slot_idx: u16 = kani::any();
kani::assume(true_slot_idx < slot_count);
let _ = frame.write_slot(SlotIdx::new(true_slot_idx), SlotValue::Bool(true));
```

### 4.3 Branch Index Arithmetic

```rust
// VIOLATION: u16::from(i) instead of proper typed index
branches.push(SlotBranch {
    condition: SlotIdx::new(u16::from(i)),  // Primitive obsession
    target: StepIdx::new(u16::from(100 + i)), // Magic arithmetic
});
```

---

## 5. REPETITION PATTERNS

Each proof function contains identical boilerplate:

```rust
let slot_count: u16 = 16;
let step_count: u16 = 200;
let mut frame = any_run_frame(slot_count, step_count);

for i in 0..slot_count {
    let _ = frame.write_slot(SlotIdx::new(i), SlotValue::Bool(false));
}
```

This appears 6 times with only minor variations. A shared `TestFixtures` helper could reduce this.

---

## 6. EVIDENCE COMMANDS

```bash
# Verify line count
wc -l crates/vb_core/src/kani_choose_replay.rs
# Expected: 340

# Check for duplicates
rg -l "kani_choose_replay_true_branch" crates/vb_core/src/
# Expected: ONLY ONE FILE

# Check primitive usage
rg "u16\s*=\s*(16|200|128)" crates/vb_core/src/kani_choose_replay.rs
# Should use limits.rs constants instead
```

---

## 7. RECOMMENDED REFACTORS

### 7.1 Create Bounded Types for Kani Harness

```rust
// In a new kani_helpers module
pub struct SlotCountBounds(u16);
pub struct StepCountBounds(u16);
pub struct BranchCountBounds(u8);

impl SlotCountBounds {
    pub const DEFAULT: Self = Self(16);
    pub fn new(raw: u16) -> Self { Self(raw) }
    pub fn get(&self) -> u16 { self.0 }
}
```

### 7.2 Extract Shared Fixtures

```rust
pub struct HarnessFixtures {
    pub slot_count: SlotCountBounds,
    pub step_count: StepCountBounds,
    pub frame: RunFrame,
}

impl HarnessFixtures {
    pub fn with_slots(slot_count: u16, step_count: u16, value: SlotValue) -> Self { ... }
}
```

### 7.3 Use Limits Constants

```rust
use crate::limits::{MAX_SLOTS_PER_STEP, MAX_STEPS_PER_WORKFLOW};

const DEFAULT_SLOT_COUNT: u16 = MAX_SLOTS_PER_STEP as u16; // Instead of 16
const DEFAULT_STEP_COUNT: u16 = 200; // Reasonable test bound
```

---

## 8. SUMMARY

| Category | Status |
|----------|--------|
| Line Count | ❌ VIOLATION (+40) |
| Duplicate File | ❌ EXISTS |
| Primitive Obsession | ❌ HIGH |
| DDD Cohesion | ❌ WEAK |
| Domain Constant Usage | ❌ NONE |

---

**DRIFT SCORE**: SEVERE

**MANDATORY ACTIONS**:
1. Delete duplicate at `verification/kani/kani_choose_replay.rs`
2. Extract 40+ lines into shared test fixtures
3. Replace all magic numbers with domain constants from `limits.rs`
4. Create bounded value object types for harness parameters
5. Re-verify line count after refactor
