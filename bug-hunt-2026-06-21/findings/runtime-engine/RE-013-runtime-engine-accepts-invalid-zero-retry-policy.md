# RE-013: Runtime engine accepts zero-attempt retry policies despite retry math rejecting them

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/engine/drive.rs:46-56`
- **Confidence**: likely

## Description

`RetryPolicy::validate_against` defines `max_attempts == 0` as invalid, but the runtime drive and action dispatch path accepts a raw `RetryPolicy` and never applies that validation before issuing action tickets or evaluating retry checks.

## Evidence

`crates/vb_runtime/src/engine/retry_math.rs:61-63` explicitly rejects zero-attempt policies:

```rust
if self.max_attempts == 0 {
    return Err(RetryPolicyMathError::ZeroMaxAttempts);
}
```

But `drive_deterministic_full` accepts the raw policy and passes it through unchanged at `crates/vb_runtime/src/engine/drive.rs:46-56` and `crates/vb_runtime/src/engine/drive.rs:63-72`:

```rust
pub fn drive_deterministic_full(
    ...
    retry_policy: RetryPolicy,
    ...
) -> RuntimeEngineResult<RuntimeSignal> {
    ...
    let signal = execute_node_full(
        ...
        retry_policy,
        ...
    )?;
```

`execute_do` then publishes that invalid capacity into an action ticket at `crates/vb_runtime/src/engine/action.rs:59-67`:

```rust
let ticket = ActionTicket {
    ...
    attempt: 1,
    ...
    capacity: retry_policy.max_attempts,
    ..Default::default()
};
```

With `max_attempts: 0`, the engine can issue a ticket whose `attempt` is already greater than `capacity`.

## Adversarial Check

An upstream caller might validate today, but this API does not encode that fact. `RetryPolicy` is a public struct with public fields, and both `drive_deterministic_full` and `drive_with_actions` accept it by value. If the runtime requires a validated policy, it should accept a validated type or validate at the boundary. Relying on every caller to remember `validate_against` defeats the point of the retry math module.

## Suggested Fix

Validate `RetryPolicy` at the runtime boundary before dispatch, or replace the public raw policy parameter with a validated newtype. At minimum, reject `max_attempts == 0` before constructing `ActionTicket` so the ticket invariant `attempt <= capacity` cannot be violated.
