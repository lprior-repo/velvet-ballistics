# ARCHITECTURAL DRIFT HAMMER REPORT
## Target: `crates/vb_runtime/src/engine/action.rs`
## Size: 674 lines | Limit: 300 lines | Over: 374 lines (125% violation)

---

## EXECUTIVE SUMMARY

This file is a **PRIMITIVE OBSESSION CASINO** married to a **SINGLE RESPONSIBILITY INCEST**. Seven distinct behavioral domains are crammed into 674 lines, with 451 of those lines (67%) being verbose inline tests that belong in a test crate.

**Verdict: GUILTY. Hammer recommended.**

---

## VIOLATION #1: <300 LINE RULE (CRITICAL)

| Metric | Value |
|--------|-------|
| Actual lines | 674 |
| Limit | 300 |
| Violation | 374 lines (125% over) |
| Test lines | 451 (67% of file) |
| Production logic | 223 lines |

**Structural breakdown:**
```
Lines 1-18:   Module header + imports (18 lines)
Lines 20-74:  execute_do function (55 lines)
Lines 76-106: execute_do_without_contract (31 lines)
Lines 109-120: execute_retry_check (12 lines)
Lines 123-131: execute_error_handler (9 lines)
Lines 134-200: resume_action_outcome (67 lines)
Lines 203-208: compute_idempotency_key (6 lines)
Lines 211-221: resolve_contract (11 lines)
Lines 223-674: INLINE TEST MODULE (451 lines — 67% of file)
```

**Required surgery:**
- Extract ALL tests to `crates/vb_runtime/src/engine/tests/action_tests.rs`
- Split production code into: `action/execution.rs`, `action/retry.rs`, `action/outcome.rs`, `action/idempotency.rs`, `action/contract.rs`

---

## VIOLATION #2: PRIMITIVE OBSESSION (SEVERE)

### 2.1 Raw `u16` for Attempt Count

```rust
// Line 110 — raw primitive, no domain semantics
pub fn execute_retry_check(
    current_attempt: u16,   // ← PRIMITIVE OBSESSION
    policy: RetryPolicy,
    body: StepIdx,
    exhausted: StepIdx,
) -> StepIdx {
    if current_attempt < policy.max_attempts {  // ← raw u16 comparison
        body
    } else {
        exhausted
    }
}
```

**Problem:** `u16` carries zero domain meaning. What is an "attempt"? It should be `Attempt(u16)` or better `RetryAttempt(u16)` with bounded validity.

**Fix:** Create `struct Attempt(u16)` with `impl TryFrom<u16>` and `fn is_exhausted(&self, policy: &RetryPolicy) -> bool`.

---

### 2.2 Unbounded `StepIdx::new()` Calls

```rust
// Lines 309-310 — raw construction with no validation
let target = execute_retry_check(0, policy, StepIdx::new(5), StepIdx::new(10));
assert_eq!(target, StepIdx::new(5));
```

Every `StepIdx::new()` in the tests is unchecked construction. If a bug exists in `execute_retry_check` using wrong step indices, these tests won't catch it because the domain doesn't validate construction.

**Fix:** Add `StepIdx::try_new(val)` that returns `Result<StepIdx, StepIdxError>`.

---

### 2.3 Raw `u16` in `ActionFailure`

```rust
// Lines 370-376 — test constructs ActionFailure with raw primitives
let failure = ActionFailure {
    code: ActionFailureCode::Timeout,
    retry_policy: vb_core::action::RetryPolicy::Retryable,
    taint: Taint::Clean,
    detail: None,
    encoded_len: 0,   // ← raw u32
};
```

`encoded_len: 0` is a magic number with no domain semantics. What does 0 mean? Max? Default?

**Fix:** `encoded_len` should be `EncodedLen(u32)` newtype with constants like `EncodedLen::ZERO`.

---

### 2.4 Raw `u128` for Idempotency Key

```rust
// Line 206 — pure function returning raw u128
pub fn compute_idempotency_key(run: RunId, seq: SeqNo, action: ActionId) -> u128 {
```

A `u128` carries no domain meaning. This should be `struct IdempotencyKey(u128)` with a `Display` impl and possibly a `LowerHex` formatting.

**Fix:** `pub struct IdempotencyKey(u128)` — wraps the raw u128.

---

## VIOLATION #3: SINGLE RESPONSIBILITY PRINCIPLE (SEVERE)

### 3.1 Seven Behaviors in One Module

This file conflates **seven distinct behavioral domains**:

| Behavior | Lines | Domain |
|----------|-------|--------|
| `execute_do` | 55 | Action execution with taint + capability enforcement |
| `execute_do_without_contract` | 31 | Conservative action execution without contract |
| `execute_retry_check` | 12 | Retry routing decision |
| `execute_error_handler` | 9 | Error handler routing |
| `resume_action_outcome` | 67 | Retry ticket construction |
| `compute_idempotency_key` | 6 | Pure cryptographic key derivation |
| `resolve_contract` | 11 | Contract lookup |

**These belong in separate modules:**
```
engine/action/
├── mod.rs           (re-exports only)
├── execution.rs     (execute_do, execute_do_without_contract)
├── retry.rs         (execute_retry_check, execute_error_handler)
├── outcome.rs       (resume_action_outcome)
├── idempotency.rs   (compute_idempotency_key, IdempotencyKey)
└── contract.rs      (resolve_contract)
```

---

### 3.2 `resume_action_outcome` Does Too Much

Lines 138-200 show a function that:
1. Pattern matches on `ActionOutcome`
2. Constructs new `ActionTicket` on retry
3. Does arithmetic overflow checking on `seq` and `attempt`
4. Computes idempotency key
5. Returns different `RuntimeSignal` variants

This is a **workflow function** masquerading as a helper. It should be a method on `ActionTicket`:

```rust
impl ActionTicket {
    pub fn retry_with(&self, outcome: &ActionOutcome) -> RuntimeEngineResult<RuntimeSignal> { ... }
}
```

---

## VIOLATION #4: TEST LOCATION (CRITICAL)

**Lines 223-674 (451 lines, 67% of file) are inline tests.**

According to the repository architecture:
- `tests/` and `benches/` must NOT be at the repository root
- Integration tests belong in `crates/workspace_tests/`

**But within a crate**, tests in `#[cfg(test)] mod tests` are acceptable IF they are under 100 lines and focused. At 451 lines, this is a **test module that ate the production module**.

**Evidence of the problem:**
- Test helpers like `make_contract()` and `make_contract_with_capability()` are defined within the test module (lines 424-437, 505-518)
- These helpers have production-level complexity but are hidden in tests
- The test module is so large it effectively doubles the file's cognitive load

**Fix:** Move to `crates/vb_runtime/src/engine/tests/action_tests.rs`

---

## VIOLATION #5: CONDITIONAL LOGIC THAT SHOULD BE METHOD DISPATCH

### 5.1 The `#[allow(unreachable_code)]` Landmine

```rust
// Lines 192-199
#[allow(unreachable_code)]
_ => Err(RuntimeEngineError::Core(
    EngineError::InternalInvariantViolation {
        reason: "unknown_action_outcome",
    },
)),
```

This catch-all `_` pattern with `#[allow(unreachable_code)]` is a code smell. It means:
1. Someone fears future `ActionOutcome` variants being added
2. The pattern match isn't closed for evolution
3. There's no `fn handle_outcome(outcome: ActionOutcome) -> RuntimeSignal` dispatch table

**Fix:** Use exhaustive matching without the allow, or better, make `ActionOutcome` an enum that implements `TryInto<RuntimeSignal>`.

---

## VIOLATION #6: TYPE SAFETY VIOLATIONS IN CONTRACT LOOKUP

### 6.1 Dual-Index Validation

```rust
// Lines 215-220 — index AND id must both match
let index = usize::from(action.get());
contracts
    .get(index)
    .filter(|c| c.id == action)  // ← redundant validation after index lookup
    .ok_or(ActionError::UnknownAction { action })
```

**Problem:** If `action.get()` returns a value larger than the registry, `contracts.get(index)` returns `None`. The `filter` adds a second validation that `c.id == action`. This is defensive programming but it reveals that `ActionId` is not being trusted correctly.

**This suggests** either:
1. The index is unreliable (broken invariant)
2. The dual-check is a hack to paper over an ID generation bug

**Fix:** Trust the index. If there's a bug, fix the ID generation, don't add redundant checks.

---

## SCOTT WLASCHIN DDD ASSESSMENT

### ✓ Passing
- **Void octagon:** No functions return `!` (good — panic-free)
- **Error pipeline:** Uses `Result` throughout (good railway)
- **Type-enforced invariants:** `RetryPolicy::NEVER`, `RetryPolicy::DEFAULT` are constrained

### ✗ Failing
- **Primitive obsession:** `u16` for attempts, `u128` for keys, `u32` for encoded_len
- **Feature envy:** `resume_action_outcome` reaches into `ActionTicket` internals to build new tickets
- **Data class:** `ActionTicket` is passive — only has fields, no behavior. Methods like `retry_with()` belong on it.
- **Inconsistent abstraction:** `compute_idempotency_key` is a bare function, not a method on a domain type

---

## RECOMMENDED REFACTORING

### Phase 1: Extract Tests (Safety net)
```bash
# Create test module
mkdir -p crates/vb_runtime/src/engine/tests/
# Move action tests to tests/action_tests.rs
# Keep inline #[cfg(test)] mod tests but empty or minimal
```

### Phase 2: Split by Responsibility
```
engine/action/
├── mod.rs          (5 lines — re-exports)
├── execution.rs    (action execution — execute_do variants)
├── retry.rs        (retry routing — execute_retry_check, execute_error_handler)  
├── outcome.rs      (resume logic — resume_action_outcome)
├── idempotency.rs  (key computation — IdempotencyKey, compute_idempotency_key)
└── contract.rs     (resolve_contract)
```

### Phase 3: Newtype Wrappers
```rust
pub struct Attempt(u16);
pub struct IdempotencyKey(u128);
pub struct EncodedLen(u32);

impl Attempt {
    pub fn is_exhausted(&self, policy: &RetryPolicy) -> bool {
        self.0 >= policy.max_attempts
    }
}
```

### Phase 4: Method Lift
```rust
impl ActionTicket {
    /// Retry this ticket given an action outcome
    pub fn retry_with(&self, outcome: ActionOutcome) -> RuntimeEngineResult<RuntimeSignal> {
        match outcome {
            ActionOutcome::Ready(_) => Ok(RuntimeSignal::Continue),
            ActionOutcome::Suspended(ticket) => Ok(RuntimeSignal::AwaitingAction(ticket)),
            ActionOutcome::Failed(failure) => self.handle_failure(failure),
        }
    }
}
```

---

## EFFORT ESTIMATE

| Phase | Lines Removed | Files Created | Risk |
|-------|---------------|---------------|------|
| Phase 1: Tests | -451 | +1 | Low |
| Phase 2: Split | -223 | +5 | Medium |
| Phase 3: Newtypes | ~50 | +1 | Medium |
| Phase 4: Methods | ~30 | 0 | Low |

**Total: -674 production lines → ~150 production lines across 6 files**

---

## FINAL VERDICT

**ARCHITECTURAL DRIFT: CONFIRMED**
**HAMMER STATUS: ARMED**

This file is a textbook case of "it fit in one file, so we kept adding to it." The 674-line monster demonstrates:
1. 125% line limit violation
2. 7 behavioral domains conflated
3. Primitive obsession on 4+ types
4. 67% inline tests obscuring production logic

The `action` module should be a directory with 6 focused files, not a 674-line monolith.
