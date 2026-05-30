# Architectural Drift Report: repeat.rs

**File**: `crates/vb_runtime/src/primitives/repeat.rs`
**Total Lines**: 968
**Limit**: 300 lines
**Status**: VIOLATION — MUST SPLIT

---

## 1. Repeat Primitive Responsibilities

| Responsibility | Current Location | Problem |
|---|---|---|
| Encode (max_attempts, current_attempt) → i64 | Free function `encode_repeat_state` | Primitive obsession — raw i64 bit-packing |
| Decode i64 → (max_attempts, current_attempt) | Free function `decode_repeat_state` | Primitive obsession — raw i64 bit-packing |
| Initialize repeat loop | `repeat_start` | Leaks i64 encoding to callers |
| Read current attempt | `repeat_attempt` | No encapsulation |
| Increment and route | `repeat_check` | Buried in function body, not in state type |
| Copy result to output | `repeat_finish` | Simple copy, no state abstraction |

---

## 2. Primitive Obsession Violations

### VIOLATION 1: Naked i64 Bit-Packing
```rust
// Lines 13, 19-34, 37-57
const REPEAT_SHIFT: u32 = 32;
fn encode_repeat_state(max_attempts: u16, current_attempt: u16) -> Result<i64, EngineError>
fn decode_repeat_state(packed: i64) -> Result<(u16, u16), EngineError>
```

**Problem**: Two u16 values are manually bit-shifted into a single i64. This is:
- No type safety — anywhere an i64 is used, you can't tell if it's a repeat state
- Bit-shifting logic duplicated across encode/decode
- Encoding invariants (max>0, current<=max) not enforced by a type

**Fix**: Extract `RepeatState` newtype:
```rust
pub struct RepeatState { max_attempts: u16, current_attempt: u16 }
impl RepeatState {
    pub fn new(max_attempts: u16, current_attempt: u16) -> Result<Self, EngineError>
    pub fn encode(&self) -> i64
    pub fn decode(packed: i64) -> Result<Self, EngineError>
    pub fn increment(&self) -> Result<Self, EngineError>  // for repeat_check
}
```

### VIOLATION 2: Magic Constant `REPEAT_SHIFT`
```rust
const REPEAT_SHIFT: u32 = 32; // Line 13
```

**Problem**: Magic number with no semantic binding to `RepeatState`. Anyone can use this constant.

**Fix**: `RepeatState::SHIFT` or encapsulate entirely.

### VIOLATION 3: SlotValue::I64 Leakage
The public API functions accept/return raw `i64` stored in slots:
```rust
run.write_slot(attempt_output, SlotValue::I64(state))?;  // line 72
run.write_slot(attempt_slot, SlotValue::I64(updated))?;   // line 107
```

**Problem**: Callers must know the encoding scheme. No `RepeatState` type to wrap this.

### VIOLATION 4: Validation Logic Not in Type
The encode/decode functions validate invariants but this validation is not owned by a type:
```rust
if max_attempts == 0 || current_attempt > max_attempts {
    return Err(invalid_repeat_state());
}
```
This appears in BOTH `encode_repeat_state` (line 20) AND `decode_repeat_state` (line 53) — duplication.

---

## 3. File Size Violation

**968 lines >> 300 line limit**

Structure (estimated):
- Production code (lines 1-148): ~148 lines
- Test code (lines 150-968): ~819 lines

**The tests MUST be moved to a separate file**: `repeat_tests.rs` or `tests/repeat.rs`

---

## 4. Required Refactoring

### New File Structure

```
primitives/
├── repeat.rs          # ~150 lines: RepeatState newtype + thin API delegates
├── repeat_tests.rs    # ~819 lines: all tests
└── helpers.rs         # shared helpers (jump_to, etc.)
```

### Step 1: Create `RepeatState` type (in repeat.rs)

```rust
/// Encapsulates retry-loop attempt counter.
/// Avoids primitive obsession by wrapping (max_attempts, current_attempt)
/// with encoded i64 storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepeatState {
    max_attempts: u16,
    current_attempt: u16,
}

const REPEAT_STATE_SHIFT: u32 = 32;

impl RepeatState {
    /// Creates a new RepeatState, validating that max_attempts > 0
    /// and current_attempt <= max_attempts.
    pub fn new(max_attempts: u16, current_attempt: u16) -> Result<Self, EngineError> {
        if max_attempts == 0 || current_attempt > max_attempts {
            return Err(EngineError::InternalInvariantViolation {
                reason: "invalid_repeat_state",
            });
        }
        Ok(Self { max_attempts, current_attempt })
    }

    /// Encode into an i64 for slot storage.
    pub fn encode(self) -> i64 {
        (i64::from(self.max_attempts) << REPEAT_STATE_SHIFT) | i64::from(self.current_attempt)
    }

    /// Decode from a slot i64. Validates reserved bits and bounds.
    pub fn decode(packed: i64) -> Result<Self, EngineError> {
        // ... full validation (see current decode_repeat_state)
    }

    /// Increment attempt counter, clamping at u16::MAX.
    pub fn increment(self) -> Result<Self, EngineError> {
        let next = self.current_attempt.saturating_add(1);
        Self::new(self.max_attempts, next)
    }

    /// Returns true if all attempts are exhausted.
    pub fn is_exhausted(self) -> bool {
        self.current_attempt >= self.max_attempts
    }
}
```

### Step 2: Slim down repeat.rs to delegate to RepeatState

```rust
pub fn repeat_start(...) -> Result<EngineSignal, EngineError> {
    let state = RepeatState::new(max_attempts, 0)?;
    run.write_slot(attempt_output, SlotValue::I64(state.encode()))?;
    jump_to(run, body)
}

pub fn repeat_check(...) -> Result<EngineSignal, EngineError> {
    let packed = expect_i64(*run.read_slot(attempt_slot)?)?;
    let state = RepeatState::decode(packed)?;
    let next_state = state.increment()?;
    run.write_slot(attempt_slot, SlotValue::I64(next_state.encode()))?;
    if next_state.is_exhausted() {
        jump_to(run, done)
    } else {
        jump_to_body(run, body_entry)
    }
}
```

### Step 3: Move all tests to `repeat_tests.rs`

---

## 5. Summary of Violations

| # | Violation | Severity | Fix |
|---|---|---|---|
| 1 | 968 lines >> 300 | CRITICAL | Split tests to separate file |
| 2 | i64 bit-packing without type | HIGH | Create `RepeatState` newtype |
| 3 | `REPEAT_SHIFT` magic constant | MEDIUM | Encapsulate in `RepeatState` |
| 4 | Duplicate validation in encode/decode | MEDIUM | Move to `RepeatState::new()` and `RepeatState::decode()` |
| 5 | `SlotValue::I64` leak in public API | MEDIUM | Hide behind `RepeatState` |

---

## 6. DDD Assessment (Scott Wlaschin)

- **Primitive obsession**: YES — raw i64 bit manipulation exposed in public API
- **NewType needed**: YES — `RepeatState` to encapsulate attempt counter
- **Workflow as state machine**: PARTIAL — the repeat_start/attempt/check/finish IS a state machine but not modeled as such
- **Parse, don't validate**: NO — validation happens in two places (encode, decode) instead of one consolidated constructor

---

**MANDATORY ACTIONS**:
1. Extract `RepeatState` newtype with encode/decode/increment/is_exhausted
2. Slim `repeat.rs` to ~150 lines (production code only)
3. Move all 29 tests to `repeat_tests.rs`
4. Update `mod.rs` / `primitives` module to export correctly
