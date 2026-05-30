# Architectural Drift Report: `step.rs`

**File:** `crates/vb_core/src/engine/step.rs`
**Total Lines:** 1151
**Line Limit:** 300
**Violation:** CRITICAL — 383% of allowed size

---

## Executive Summary

The file is a **1,151-line monolith** that conflates two distinct responsibilities:
1. **Step execution engine** (~270 lines of production code, lines 1–270)
2. **Integration test suite** (879 lines of inline tests, lines 272–1151)

The test module alone exceeds the 300-line file limit by 193%.

---

## 1. SIZE VIOLATION

| Region | Lines | % of File |
|--------|-------|-----------|
| Production impl (step_once, execute_node, resume_*, helpers) | ~270 | 23% |
| Test module `#[cfg(test)] mod tests` | 879 | 76% |
| Blank lines / module docs | ~2 | <1% |
| **TOTAL** | **1151** | **100%** |

**Required action:** Extract the `#[cfg(test)] mod tests` block into `tests/step_tests.rs`. The production module must remain ≤300 lines.

---

## 2. STEP EXECUTION RESPONSIBILITY MAP

### 2a. Public API Surface

| Function | Lines | Responsibility |
|----------|-------|----------------|
| `step_once` | 20–48 | Top-level single-step driver. Fetches node, executes, handles error routing, marks state |
| `journal_action_suspended` | 110–125 | Constructs `ActionJournalEvent::Suspended` for Do-node suspension |
| `resume_action_completion` | 137–170 | Writes action output, marks step succeeded, advances PC |
| `resume_action_failure` | 184–211 | Marks step failed, attempts error handler routing |

### 2b. Internal Dispatch

| Function | Lines | Responsibility |
|----------|-------|----------------|
| `execute_node` | 50–74 | Route to specific node executor by `CompiledNodeKind` |
| `execute_boundary_node` | 76–104 | Handle boundary nodes (Do/Wait/Ask/Jump/Finish/ErrorHandler) |
| `mark_step_after_signal` | 213–224 | Map `EngineSignal` → `StepState` |

### 2c. Primitive Node Executors

| Function | Lines | Responsibility |
|----------|-------|----------------|
| `eval_expr_node` | 226–240 | Evaluate expression, write to output slot, advance |
| `build_object_node` | 242–255 | Build object from field descriptors, write handle |
| `build_list_node` | 257–270 | Build list from slot descriptors, write handle |

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### 3a. `ActionTicket` Construction in Tests (Line 790–798, 854–862, 882–889)

```rust
let ticket = ActionTicket {
    run: RunId::new(1),
    step: StepIdx::new(0),
    seq: SeqNo::new(1),
    action: ActionId::new(1),
    attempt: 1,
    idempotency_key: 0,   // ← bare u32
    capacity: 1,          // ← bare u32
};
```

`idempotency_key` and `capacity` are raw `u32` fields on `ActionTicket`. These should be newtypes: `IdempotencyKey(u32)` and `Capacity(u32)`. This is textbook Wlaschin primitive obsession — meaningful domain concepts stored as bare integers.

### 3b. Inline Index Construction Throughout Tests

```rust
StepIdx::new(0), SlotIdx::new(1), ExprIdx::new(0), ConstIdx::new(0)
```

Repeated ~60+ times in tests. While `StepIdx`/`SlotIdx` are newtypes (good), the *pattern* of `Type::new(literal)` with bare integers scattered through tests is a code smell — suggests no builder or fixture infrastructure.

### 3c. `finish_run` Error Case Uses Stringly-typed Resource

In `node_helpers::finish_run` (called from line 93), the result slot is `SlotIdx` but the error path in `EngineError::MissingNextStep { step }` uses a bare `StepIdx` — not the issue, but worth noting the `EngineError` enum has raw `&'static str` for resource names in several variants.

### 3d. Hardcoded Digest Arrays in Every Test

```rust
digest: WorkflowDigest::from_bytes([0x11; 32]),
digest: WorkflowDigest::from_bytes([0x22; 32]),
// ... repeated 15+ times
```

`WorkflowDigest::from_bytes([0xNN; 32])` is repeated per test. Should be a shared `fn dummy_digest(u8) -> WorkflowDigest` helper.

---

## 4. SCOTT WLASCHIN DDD OBSERVATIONS

### 4a. What Works

- **`CompiledNodeKind`** is a proper tagged union / sum type — models node variants well
- **`EngineSignal`** is a clean result type for the step engine
- **`StepIdx`, `SlotIdx`, `ExprIdx`** are proper newtypes for index roles
- **`ActionTicket`** captures a specific domain concept (a ticket for an in-flight action)
- **`resume_action_completion` / `resume_action_failure`** are pure functions with explicit inputs/outputs — no hidden state

### 4b. State Machine Is Implicit, Not Explicit

The step execution state machine (Running → Succeeded/Waiting/Asking/Failed) is encoded in `mark_step_after_signal` as a `match` on `EngineSignal`. This is the Wlaschin "state machine as data" pattern but buried inside a function rather than being a named `enum StepStateTransition { ... }` with explicit transitions.

**Current:**
```rust
fn mark_step_after_signal(run: &mut RunFrame, step: StepIdx, signal: &EngineSignal) -> Result<(), EngineError> {
    match signal {
        EngineSignal::AwaitingWait => run.mark_waiting(step),
        EngineSignal::AwaitingAsk  => run.mark_asking(step),
        EngineSignal::AwaitingAction | EngineSignal::StepBudgetExhausted => Ok(()),
        EngineSignal::Continue | EngineSignal::Finished(_, _) => run.mark_succeeded(step),
    }
}
```

**Wlaschin improvement:** Define a `StepStateTransition` enum with variants `Wait`, `Ask`, `Run`, `Succeed`, and derive the state machine explicitly. The current version mixes signal semantics with state machine transitions.

### 4c. Error Handling Is Good — `route_error_handler` Is Explicit

The error routing in `step_once` (lines 33–44) is clean two-phase: first mark step failed, then route to handler. No hidden control flow.

### 4d. `execute_boundary_node` Catch-All Is Suspicious

```rust
_ => Err(EngineError::UnsupportedPrimitive {
    primitive: "not_yet_implemented",
}),
```

This catch-all returns an error for any unknown `CompiledNodeKind` variant. This is appropriate, but the string `"not_yet_implemented"` is primitive obsession — should be `UnknownNodeVariant` or similar.

---

## 5. DUPLICATION ANALYSIS

### 5a. Test Workflow Construction — Extreme Duplication

Each test (20+ tests) independently builds a complete `CompiledWorkflow` via `WorkflowParts {...}`. Pattern:

```rust
let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
    name: Box::<str>::from("some_name"),
    digest: WorkflowDigest::from_bytes([0xNN; 32]),
    nodes: vec![CompiledNode { id: StepIdx::new(0), output: None, next: Some(StepIdx::new(1)),
        on_error: None, error_slot: None, kind: CompiledNodeKind::Nop }, /* ... */]
    .into_boxed_slice(),
    expressions: Box::new([]),
    accessors: Box::new([]),
    constants: Box::new([]),
    slot_count: 1,
    symbols_count: 0,
    entry: StepIdx::new(0),
    resource_contract: ResourceContract::DEFAULT,
    step_names: Box::new([]),
})?;
```

**Lines per test for boilerplate:** ~35 lines of identical/near-identical boilerplate per test.

**Duplication estimate:** ~600 lines of boilerplate across the test module.

### 5b. Existing Helpers Are Underused

Only 2 helpers exist:
- `nop_then_finish_workflow()` — used in 3 tests
- `test_frame(workflow)` — used in all tests

The remaining ~18 tests build full workflows from scratch despite identical patterns.

---

## 6. REFACTOR PRESCRIPTION

### Phase 1: Extract Tests (880 lines → 0 lines in step.rs)

Create `crates/vb_core/src/engine/tests/step_tests.rs`:

```rust
// Extract entire #[cfg(test)] mod tests { ... } block
// from step.rs lines 272–1151
```

Update `crates/vb_core/src/engine.rs` to add:
```rust
#[cfg(test)]
mod tests {
    mod step_tests;
}
```

### Phase 2: Add Test Fixtures (in the new test file)

```rust
fn dummy_digest(seed: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([seed; 32])
}

fn simple_single_node_workflow(kind: CompiledNodeKind) -> CompiledWorkflow { ... }

fn do_node_workflow(action: ActionId, input: SlotIdx) -> CompiledWorkflow { ... }

// etc.
```

### Phase 3: Consolidate `StepStateTransition` (optional, not blocking)

Extract `mark_step_after_signal` logic into a `StepStateTransition` enum. Not strictly required but would improve Wlaschin compliance.

### Phase 4: Address `idempotency_key` / `capacity` Primitive Obsession

File a separate bead for `ActionTicket` newtype refinement. Do not fix in this file — fix at the type definition in `action.rs`.

---

## 7. VERDICT

| Check | Result |
|-------|--------|
| File size ≤ 300 lines | **FAIL** — 1151 lines |
| Test code in separate file | **FAIL** — 879 lines inline |
| Primitive obsession | **WARN** — `idempotency_key`, `capacity`, `not_yet_implemented` |
| DDD state machine explicitness | **PARTIAL** — implicit in `mark_step_after_signal` |
| Parse don't validate | **PASS** — `step_once` uses `ok_or` for PC validation |
| No `unwrap`/`panic`/`todo` | **PASS** — clean error handling |

**STATUS: REFACTOR REQUIRED**

The file MUST be split. The production implementation (~270 lines) is acceptable; the test module (879 lines) must move to a separate file. After extraction, `step.rs` will be ~275 lines including module docs and blank lines — within the 300-line limit.
