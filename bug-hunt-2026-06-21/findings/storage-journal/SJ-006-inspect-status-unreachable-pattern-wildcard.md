# SJ-006: `lifecycle_state_to_inspect_status` carries an unreachable wildcard + `unreachable_patterns` allow

- **Severity**: Info
- **Category**: simplification
- **Location**: `crates/vb_storage/src/journal/incident/lifecycle.rs:12`
- **Confidence**: confirmed

## Description

`lifecycle_state_to_inspect_status` annotates the function with
`#[allow(unreachable_patterns)]` and ends with `_ => "running"`, but the
preceding arms already exhaust every `LifecycleState` variant. The wildcard
and the allow are both dead, and the wildcard silently masks future
`LifecycleState` variants as `"running"`.

## Evidence

```rust
#[must_use]
#[allow(unreachable_patterns)]
pub fn lifecycle_state_to_inspect_status(state: LifecycleState) -> &'static str {
    match state {
        LifecycleState::Cancelled => "cancelled",
        LifecycleState::Completed => "finished",
        LifecycleState::Failed => "failed",
        LifecycleState::Pending | LifecycleState::Active | LifecycleState::WaitingAnswer => {
            "running"
        }
        _ => "running",
    }
}
```

If `LifecycleState` ever gains a new variant (e.g. `Paused`, `Suspended`),
this match will silently map it to `"running"` instead of forcing the author
to decide its inspect-status mapping.

## Adversarial Check

The `#[allow(unreachable_patterns)]` is the giveaway: the author already
knows the wildcard is unreachable against the current enum, and added the
allow to suppress the warning. The combination of "allow + wildcard" means
(1) today's code is misleading (looks defensive but cannot fire), and (2)
tomorrow's enum additions are silently absorbed.

## Suggested Fix

Delete both the wildcard and the allow:
```rust
#[must_use]
pub fn lifecycle_state_to_inspect_status(state: LifecycleState) -> &'static str {
    match state {
        LifecycleState::Cancelled => "cancelled",
        LifecycleState::Completed => "finished",
        LifecycleState::Failed => "failed",
        LifecycleState::Pending
        | LifecycleState::Active
        | LifecycleState::WaitingAnswer => "running",
    }
}
```
The compiler will then require explicit handling when new variants are added.
