# RE-003: `read_attempt_from_slot` returns `0` for uninitialized slots

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_runtime/src/engine/handlers/util.rs:16-33`
- **Confidence**: likely

## Description

`read_attempt_from_slot` maps an `EngineError::SlotUninitialized` to `Ok(0)`. The caller `handle_retry_check` then passes that zero into `execute_retry_check`, which compares `current_attempt < policy.max_attempts` — a value of 0 is *always* less than `max_attempts` (which is `≥ 1` by validation), so the engine always routes to the retry body. If the body does not write back the incremented attempt counter, the engine loops against the same uninitialized slot until the step budget is exhausted.

## Evidence

`crates/vb_runtime/src/engine/handlers/util.rs:16-33`:

```rust
pub(crate) fn read_attempt_from_slot(run: &RunFrame, slot: SlotIdx) -> RuntimeEngineResult<u16> {
    match run.read_slot(slot) {
        Ok(value) => match *value {
            SlotValue::I64(v) => u16::try_from(v).map_err(|_| ...),
            _ => Err(RuntimeEngineError::Core(EngineError::TypeMismatch { ... })),
        },
        Err(EngineError::SlotUninitialized { .. }) => Ok(0),
        Err(e) => Err(RuntimeEngineError::Core(e)),
    }
}
```

`crates/vb_runtime/src/engine/handlers/action.rs:55-59` then uses the value to pick a branch:

```rust
let current_attempt = read_attempt_from_slot(run, policy_slot)?;
let target = execute_retry_check(current_attempt, retry_policy, body, exhausted);
run.set_pc(target)?;
```

`execute_retry_check` (engine/action.rs:108-119):

```rust
pub fn execute_retry_check(
    current_attempt: u16,
    policy: RetryPolicy,
    body: StepIdx,
    exhausted: StepIdx,
) -> StepIdx {
    if current_attempt < policy.max_attempts {
        body
    } else {
        exhausted
    }
}
```

Crucially, `handle_retry_check` never writes back to `policy_slot`. There is no increment in the handler. So if the body also does not write to the slot, every call returns 0, `0 < max_attempts` is always true, and the body is re-entered forever. Only the step budget prevents runaway.

## Adversarial Check

1. *"The body always writes the attempt slot."* — I found no proof of this. The contract is implicit and undocumented in `handle_retry_check`. The retry primitive (`primitives/retry.rs::retry_on_failure`) *does* write the slot, but that is a separate code path used by `TryAgain`, not by `RetryCheck`. `RetryCheck` is wired to `handle_retry_check`, which does not.
2. *"This is a stub for legacy callers."* — The handler comment in `engine/action.rs:107` calls `execute_retry_check` "Backward-compatible". Legacy or not, it is reachable from `CompiledNodeKind::RetryCheck` via `execute.rs:197-201`. Reachable code that depends on an unwritten contract is a bug waiting to happen.
3. *"Returning 0 is safer than erroring."* — The opposite is true. Returning 0 routes execution into the body indefinitely; returning an `EngineError::SlotUninitialized` would surface the misconfiguration immediately. Defaulting to a value that satisfies the "retry" predicate is the worst possible default.

Severity Medium: no panic, but the contract is implicit and the default behavior is "loop until step budget dies", which is the opposite of fail-fast.

## Suggested Fix

Either:

(a) Return `Err(SlotUninitialized)` from `read_attempt_from_slot` when the slot is uninitialized, and let `handle_retry_check` decide whether to treat that as "first attempt" with an explicit write-back, or

(b) Have `handle_retry_check` write `(attempt + 1)` back to the slot itself so the counter advances even if the body does not write to it, mirroring what `repeat_check` does at `primitives/repeat.rs:94-117`.

Option (b) is consistent with the `repeat` primitive's in-handler increment and removes the implicit contract.
