# Architectural Drift Report: `execute.rs`

**File**: `crates/vb_runtime/src/engine/execute.rs`
**Total Lines**: 1910
**Violation Status**: ❌ CATASTROPHIC - 536% over the 300-line limit

---

## Executive Summary

This file violates the **<300 line rule** by a factor of 6.3x. It is a monolithic god-function that mixes dispatch logic, domain execution, and 1537 lines of inline tests. The file must be shattered into specialized, single-responsibility modules.

---

## Violation Breakdown

### Primary Violation: File Size (1910 >> 300)

| Section | Lines | Purpose |
|---------|-------|---------|
| Module header + imports | 1-19 | Bootstrap |
| `read_attempt_from_slot` helper | 20-41 | Utility |
| `execute_node_full` function | 43-371 | **CORE GATE** - 328 lines of dispatch match |
| Test module | 373-1910 | 1537 lines of inline tests |

**The production code (execute_node_full) is 328 lines alone — already 109% over the limit.**

---

## Responsibility Map of `execute_node_full`

`execute_node_full` is a **7-domain god dispatcher**:

```
execute_node_full(node: &CompiledNode)
│
├── ForEach Domain (lines 56-102)
│   ├── ForEachStart
│   ├── ForEachNext
│   └── ForEachJoin
│
├── Together Domain (lines 104-144)
│   ├── TogetherStart
│   ├── TogetherBranch
│   └── TogetherJoin
│
├── Collect Domain (lines 146-210)
│   ├── CollectStart
│   ├── CollectPage
│   ├── CollectNext
│   └── CollectFinish
│
├── Reduce Domain (lines 212-261)
│   ├── ReduceStart
│   ├── ReduceNext
│   └── ReduceFinish
│
├── Repeat Domain (lines 263-289)
│   ├── RepeatStart
│   ├── RepeatAttempt
│   ├── RepeatCheck
│   └── RepeatFinish
│
├── Wait/Ask Domain (lines 291-315)
│   ├── WaitUntil
│   ├── WaitEvent
│   ├── Ask
│   └── AskResume
│
├── Action Domain (lines 317-354)
│   ├── Do (with/without contract)
│   ├── RetryCheck
│   └── ErrorHandler
│
└── Fallback (lines 365-369)
    └── step_once (core engine)
```

---

## Scott Wlaschin DDD Violations

### 1. Primitive Obsession (Multiple Instances)

#### Instance A: Attempt Count as Raw `u16` wrapped in `I64`
```rust
// Lines 23-40: read_attempt_from_slot
fn read_attempt_from_slot(run: &RunFrame, slot: SlotIdx) -> RuntimeEngineResult<u16> {
    match run.read_slot(slot) {
        Ok(value) => match *value {
            SlotValue::I64(v) => u16::try_from(v)...  // ← Primitive conversion in domain logic
            _ => Err(...)
        },
        Err(_) => Ok(0),  // ← Magic default
    }
}
```
**Problem**: The domain concept "attempt count" is a raw `u16`. The conversion from `SlotValue::I64` to `u16` leaks throughout the codebase.

**Fix**: Introduce `AttemptCount(SlotValue)` wrapper with `fn read_attempt(&self) -> RuntimeEngineResult<u16>`.

#### Instance B: `FanoutLimit` Wrapping at Call Site
```rust
// Line 67
FanoutLimit::new(*limit),  // raw u16 limit dereferenced at call site
```
**Problem**: `limit` is a raw `u16` in `CompiledNodeKind::ForEachStart`. The `FanoutLimit::new()` wrapper is called inline in the match arm, mixing dispatch with type construction.

#### Instance C: `branch_count: u16` Primitive in TogetherJoin
```rust
// Line 243
branch_count: 2,  // raw primitive in struct literal
```
**Problem**: `branch_count` is a `u16` representing "number of parallel branches". No type enforces that this is always > 0.

#### Instance D: `max_attempts: u16` in RepeatStart
```rust
// Line 265
max_attempts: 1,  // raw u16 in struct literal
```
**Problem**: `max_attempts` can be 0, which is semantically invalid for a retry loop.

#### Instance E: `page_size: u16` and `limit: u16` in CollectStart
```rust
// Lines 148-149
limit: 10,  // raw u16
page_size: 5,  // raw u16
```
**Problem**: Both can be 0, which may not be the intended semantics for pagination.

---

### 2. God Function / Feature Envy

`execute_node_full` (lines 43-371) exhibits **classic God Function behavior**:

- **8 parameters** passed on every call: `plan`, `run`, `store`, `node`, `contracts`, `retry_policy`, `collect_states`, `granted`
- **328 lines** of dispatch logic
- **Envies the data** of its callers — it reaches into `CompiledNodeKind` variants and extracts fields
- **Duplicates error mapping**: `.map_err(RuntimeEngineError::Core).map(runtime_from_core)` appears ~20 times

**This function should be a 10-line router that delegates to domain-specific executors.**

---

### 3. Switch Statement Smell

The match on `node.kind` (lines 55-370) is a **316-line switch** with nearly identical arms:

```rust
CompiledNodeKind::ForEachStart { input, item_slot, limit, body, done } =>
    crate::primitives::for_each::for_each_start(
        run, store, *input, *item_slot, FanoutLimit::new(*limit), *body, *done, node.output
    )
    .map_err(RuntimeEngineError::Core)
    .map(runtime_from_core),
```

Each arm follows the same pattern:
1. Destructure the variant
2. Call a primitive module function
3. Map errors
4. Convert signals

**This is a code smell indicating missing abstraction.** Each domain should have its own `execute_*` function.

---

### 4. Temporal Coupling: Error Handling Repeated Per-Arm

```rust
// Lines 91-101: Typical Join finish pattern
let step = node.id;
match crate::primitives::for_each::for_each_join(...) {
    Ok(signal) => Ok(runtime_from_core(signal)),
    Err(e) => Err(RuntimeEngineError::Core(e)),
}
```

The `step = node.id` extraction and the `node.next`/`node.output` passing is repeated across **all Join-type nodes** (ForEachJoin, TogetherJoin, CollectFinish, ReduceFinish).

**Fix**: Join nodes could implement a `JoinExecution` trait with a `fn finish(run, output, next, step)` method.

---

### 5. Inappropriate Intimacy

`execute_node_full` knows too much about the internals of:
- `CompiledNodeKind` variants (all fields extracted)
- `RunFrame` internals (slot read/write, PC manipulation)
- `CollectStates` mutation
- `RetryPolicy` semantics

**This is a clear violation of Demeter's Law.** The function should not need to know this much about the data structures it operates on.

---

## Test Module Bloat (1537 Lines)

Lines 373-1910 contain **29 test functions** all defined inline with the production code.

**Problems**:
1. Tests should be in `crates/vb_runtime/src/engine/tests/` or a separate `execute_tests.rs`
2. Each test re-implements `make_workflow`, `make_run`, `finish_node`, `nop_forward` — **duplicated code**
3. Test helpers (`make_workflow`, `make_run`) are 30+ lines each and repeated verbatim

**Evidence of helper duplication**:
- `finish_node`: lines 385-396
- `nop_forward`: lines 399-408
- `make_workflow`: lines 412-442
- `make_workflow_with_constants`: lines 417-442
- `make_run`: lines 444-452

---

## Required Refactoring

### Phase 1: Extract Tests (Mandatory)

**Action**: Move lines 373-1910 to `crates/vb_runtime/src/engine/execute_tests.rs`

**Rationale**: Separating 1537 lines of tests reduces the file to 372 lines — still over 300, but a massive improvement.

---

### Phase 2: Extract Domain Executors (Mandatory)

**Action**: Create one file per domain under `crates/vb_runtime/src/engine/primitives/`:

```
engine/
├── execute.rs          # 10-line router + read_attempt_from_slot
├── execute_tests.rs    # Moved from execute.rs
└── primitives/
    ├── for_each_execute.rs   # ForEachStart, ForEachNext, ForEachJoin
    ├── together_execute.rs    # TogetherStart, TogetherBranch, TogetherJoin
    ├── collect_execute.rs     # CollectStart, CollectPage, CollectNext, CollectFinish
    ├── reduce_execute.rs     # ReduceStart, ReduceNext, ReduceFinish
    ├── repeat_execute.rs      # RepeatStart, RepeatAttempt, RepeatCheck, RepeatFinish
    ├── wait_ask_execute.rs    # WaitUntil, WaitEvent, Ask, AskResume
    └── action_execute.rs     # Do, RetryCheck, ErrorHandler
```

**Each executor file target**: <80 lines

---

### Phase 3: Value Object Extraction (Required for Primitive Obsession)

**Action**: Create domain types that wrap primitives:

```rust
// engine/domain/attempt_count.rs
pub struct AttemptCount(u16);
impl AttemptCount {
    pub fn read_from_slot(run: &RunFrame, slot: SlotIdx) -> RuntimeEngineResult<Self>;
    pub fn increment(&self) -> Self;
    pub fn value(&self) -> u16;
}
```

```rust
// engine/domain/fanout_limit.rs
pub struct FanoutLimit(u16);
impl FanoutLimit {
    pub fn new(limit: u16) -> RuntimeEngineResult<Self>;
}
```

```rust
// engine/domain/execution_context.rs
pub struct ExecutionContext<'a> {
    pub run: &'a mut RunFrame,
    pub store: &'a mut ValueStore,
    pub collect_states: &'a mut CollectStates,
    pub granted: &'a CapabilitySet,
    pub node: &'a CompiledNode,
    pub plan: &'a CompiledWorkflow,
    pub contracts: &'a [ActionContract],
    pub retry_policy: RetryPolicy,
}
```

---

### Phase 4: Router Signature (Target)

```rust
pub fn execute_node_full(ctx: &mut ExecutionContext) -> RuntimeEngineResult<RuntimeSignal> {
    match &ctx.node.kind {
        CompiledNodeKind::ForEachStart { .. } => for_each::execute_start(ctx),
        CompiledNodeKind::ForEachNext { .. } => for_each::execute_next(ctx),
        // ...
    }
}
```

**Target `execute.rs` size**: ~50 lines (router + `read_attempt_from_slot`)

---

## Summary of Violations

| Violation | Severity | Count |
|-----------|----------|-------|
| File size > 300 lines | **CRITICAL** | 1 (1910 lines) |
| Primitive obsession | **HIGH** | 5 instances |
| God function | **HIGH** | 1 (`execute_node_full`) |
| Switch statement smell | **MEDIUM** | 1 (316-line match) |
| Temporal coupling | **MEDIUM** | ~20 repeated error mapping blocks |
| Inappropriate intimacy | **MEDIUM** | 1 function with 8 parameters |
| Test module inline | **LOW** | 1537 lines misplaced |

---

## Verdict

**ARCHITECTURAL REJECTION**: This file must not land in the codebase in its current form.

**Minimum Viable Fix**: Extract test module immediately. This alone brings the file to 372 lines.

**Acceptable Fix**: Complete Phase 1 + Phase 2 + Phase 3 refactoring.

**Deadline**: Next agent session must not touch this file until it is <300 lines of production code with all tests extracted.
