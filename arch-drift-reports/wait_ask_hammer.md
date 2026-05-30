# Architectural Drift Report: `wait_ask.rs`

**File**: `crates/vb_runtime/src/primitives/wait_ask.rs`
**Total Lines**: 726
**Line Limit**: 300
**Violation**: YES — 426 lines over limit (142% excess)

---

## 1. LINE COUNT VIOLATION

| Region | Lines | Status |
|--------|-------|--------|
| Production functions (1-106) | 106 | OK |
| Test module (108-726) | 619 | OVER LIMIT BY 319 |
| **Total** | **726** | **FAIL** |

The test block alone (619 lines) exceeds the 300-line file limit. This file must be split into two:
- `wait_ask.rs` — production primitives only (~110 lines)
- `wait_ask_tests.rs` — moved to `crates/workspace_tests/` or behind `#[cfg(test)]` in a separate test file

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Raw `SlotIdx` for All Semantic Roles

Every function takes `SlotIdx` without domain wrapping:

```rust
// All these use raw SlotIdx — indistinguishable without reading context
pub fn wait_until(run: &mut RunFrame, deadline_slot: SlotIdx, ...) 
pub fn wait_event(run: &mut RunFrame, event: SlotIdx, timeout_slot: Option<SlotIdx>, ...)
pub fn ask(run: &mut RunFrame, prompt: SlotIdx, timeout_slot: Option<SlotIdx>, ...)
pub fn ask_resume(run: &mut RunFrame, answer: SlotIdx, output: Option<SlotIdx>, next: Option<StepIdx>, step: StepIdx, ...)
```

**DDD Fix**: Create distinct newtypes:
```rust
struct DeadlineSlot(SlotIdx);
struct EventSlot(SlotIdx);
struct PromptSlot(SlotIdx);
struct AnswerSlot(SlotIdx);
struct OutputSlot(SlotIdx);
struct TimeoutSlot(SlotIdx);
struct ResumeTargetStep(StepIdx);
```

This makes illegal states unrepresentable — you cannot accidentally pass an `EventSlot` where a `DeadlineSlot` is expected.

### 2.2 Raw `SlotValue` for Domain Values

`deadline`, `event`, `timeout`, `prompt`, and `answer` are all unvalidated `SlotValue`. These should be lifted to value objects:
- `Deadline(i64)` — validated positive integer
- `Event(i64)` — validated event ID
- `Timeout(i64)` — validated non-negative duration
- `Prompt(SlotValue)` — validated non-Bool
- `Answer(SlotValue)` — any value including Null

### 2.3 `StepIdx` Passed as Raw in `ask_resume`

`next: Option<StepIdx>` and `step: StepIdx` are primitive types. The `step` parameter is only used for error construction (`MissingNextStep { step }`). This should be `ResumeTargetStep` or the error construction should be lifted out.

---

## 3. VALIDATION GAPS (Exposed by Adversarial Tests)

The file's own adversarial tests (lines 505-568) prove validation is incomplete:

### 3.1 No Deadline Sign Validation
```rust
// Lines 505-518: Negative deadline accepted silently
run.write_slot(deadline, SlotValue::I64(-1))  // BUG documented in test
let result = wait_until(&mut run, deadline);
assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingWait)); // Passes!
```
```rust
// Lines 521-532: Zero deadline accepted silently
run.write_slot(deadline, SlotValue::I64(0))  // epoch — likely unintended
```

**Fix**: `validate_numeric` should check `> 0` for deadline, or a `Deadline` newtype should enforce this at construction.

### 3.2 No Timeout Sign Validation
```rust
// Lines 553-568: Negative timeout accepted silently
run.write_slot(timeout, SlotValue::I64(-999))
let result = wait_event(&mut run, event, Some(timeout));
assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingWait)); // Passes!
```
```rust
// Lines 535-550: Zero timeout accepted silently
run.write_slot(timeout, SlotValue::I64(0))
let result = ask(&mut run, prompt, Some(timeout));
assert_eq!(result, Ok(vb_core::EngineSignal::AwaitingAsk)); // Passes!
```

**Fix**: `Timeout` newtype should enforce `>= 0`, or validation added.

### 3.3 `validate_prompt` is Too Permissive
```rust
// Lines 98-106
fn validate_prompt(value: SlotValue) -> Result<(), EngineError> {
    match value {
        SlotValue::Bool(_) => Err(...),  // Only rejects Bool
        _ => Ok(()),  // Accepts everything else: Null, Symbol, I64, F64!
    }
}
```
The test at line 708 confirms Bool is rejected, but Null/I64/F64 pass through as valid prompts. The comment says "prompt-compatible" but no actual compatibility check exists beyond "not Bool". If prompts should be Symbols, validate that explicitly.

---

## 4. WORKFLOW / STATE MACHINE OBSERVATIONS

The four public functions model suspension states of a workflow:

| Function | Signal | PC Change | Semantic |
|----------|--------|-----------|----------|
| `wait_until` | `AwaitingWait` | No change | Suspend until wall-clock deadline |
| `wait_event` | `AwaitingWait` | No change | Suspend until event OR timeout |
| `ask` | `AwaitingAsk` | No change | Suspend for user answer |
| `ask_resume` | `Continue` | Jump to `next` | Resume after answer received |

**Issue**: These are free functions, not an explicit state machine. The valid state transitions are not machine-checked:
- No guarantee that `ask_resume` is only called after `ask`
- No guarantee that `wait_until`/`wait_event` are only called when in `AwaitingWait` state
- No typestate enforcing "must be in AwaitingAsk before calling ask_resume"

Consider a `WaitAskWorkflow` state machine with explicit transitions if the domain requires this constraint.

---

## 5. TEST BLOATING

The `#[cfg(test)]` block (lines 108-726) is 619 lines. This is:
- Larger than the entire 300-line limit
- Mixes unit tests (RunFrame-only) with BDD-style Given/When/Then commentary
- The adversarial tests (503-705) document bugs rather than verify fixed behavior

**Recommendation**: Move to `crates/workspace_tests/vb_runtime_wait_ask.rs` or a `wait_ask_bdd.rs` integration test file. Keep only a minimal smoke test in the module.

---

## 6. SUMMARY OF VIOLATIONS

| Rule | Severity | Description |
|------|----------|-------------|
| **Line Count** | CRITICAL | 726 > 300 (142% excess) |
| **Primitive Obsession** | HIGH | Raw `SlotIdx`, `StepIdx`, `SlotValue` for all domain concepts |
| **No Value Objects** | HIGH | Missing `Deadline`, `Timeout`, `Event`, `Prompt`, `Answer` types |
| **Validation Gaps** | MEDIUM | Negative/zero deadlines and timeouts silently accepted |
| **Weak `validate_prompt`** | MEDIUM | Only rejects Bool; no positive validation |
| **Implicit State Machine** | LOW | Workflow states not modeled as explicit transitions |
| **Test Bloat** | CRITICAL | 619-line test block doubles as its own file |

---

## 7. REQUIRED REFACTORS

### 7.1 Immediate (Line Count Fix)
1. Create `crates/vb_runtime/src/primitives/wait_ask.rs` with production code only (~110 lines)
2. Move tests to `crates/workspace_tests/wait_ask_bdd.rs` or `wait_ask_tests.rs`

### 7.2 Short Term (DDD Fix)
3. Add newtype wrappers in `vb_core` or `vb_runtime::primitives`:
   - `DeadlineSlot`, `EventSlot`, `PromptSlot`, `AnswerSlot`, `OutputSlot`, `TimeoutSlot`
   - `Deadline(i64)`, `Timeout(i64)`, `Event(i64)`, `Answer(SlotValue)`
4. Refactor functions to use domain types instead of raw indices
5. Add deadline/timeout sign validation at construction time

### 7.3 Medium Term (Workflow Fix)
6. Consider `WaitAskState` enum with `AwaitingWaitUntil(DeadlineSlot)`, `AwaitingEvent { event: EventSlot, timeout: Option<TimeoutSlot> }`, `AwaitingAsk { prompt: PromptSlot, timeout: Option<TimeoutSlot> }`, `AwaitingAnswer(AnswerSlot)` — with explicit transition functions

---

**STATUS: MUST REFACTOR**

File is 2.4x the line limit and has documented primitive obsession throughout. The adversarial tests explicitly document bugs (negative/zero values accepted). Refactor into separate production + test files first, then apply DDD newtypes.
