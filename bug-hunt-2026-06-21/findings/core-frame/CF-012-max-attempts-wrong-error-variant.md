# CF-012: `MaxAttempts::try_new` returns wrong error variant for invalid user input

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_core/src/ids/domain_values.rs:97`
- **Confidence**: confirmed

## Description

`MaxAttempts::try_new` is documented to return
`EngineError::InvalidRepeatState` when `value == 0`, but the
implementation returns `EngineError::InternalInvariantViolation {
reason: "max_attempts_cannot_be_zero" }`. The two variants have very
different semantics: `InvalidRepeatState` is a user-facing validation
error, while `InternalInvariantViolation` is reserved for "should never
happen" runtime invariant failures.

## Evidence

```rust
/// Creates a max attempts value, validating that it is non-zero.
///
/// # Errors
/// Returns `EngineError::InvalidRepeatState` if value is 0.
pub fn try_new(value: u16) -> Result<Self, EngineError> {
    if value == 0 {
        return Err(EngineError::InternalInvariantViolation {
            reason: "max_attempts_cannot_be_zero",
        });
    }
    Ok(Self(value))
}
```

(`crates/vb_core/src/ids/domain_values.rs:92-104`)

The doc says `InvalidRepeatState`; the code returns
`InternalInvariantViolation`.

## Adversarial Check

One might argue "the variant name doesn't matter as long as it's an
error." But `EngineError::InternalInvariantViolation` typically triggers
different observability — paged alerts, retry-suppression, etc. — than a
validation error. A user who configures `max_attempts: 0` (a perfectly
ordinary configuration mistake) will trip the "internal invariant"
telemetry, polluting dashboards and potentially paging on-call.

## Suggested Fix

Either change the return to `EngineError::InvalidRepeatState` (matching
the doc), or update the doc to match the code. Prefer the former: this
*is* a user-input validation error.
