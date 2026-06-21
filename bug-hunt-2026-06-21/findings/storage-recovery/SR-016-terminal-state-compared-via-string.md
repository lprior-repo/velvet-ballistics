# SR-016: `recover_runtime_summary_with_expected` compares terminal states via string

- **Severity**: Info
- **Category**: simplification
- **Location**: `crates/vb_storage/src/recovery/recover.rs:222`
- **Confidence**: confirmed

## Description

`recover_runtime_summary_with_expected` compares the recovered terminal
state against the expected one by formatting both to `String` via
`terminal_state_to_string` and comparing the strings. This is fragile: a
future `RecoveryTerminalState` variant whose `Display` representation
collides with another (or whose payload is meaningful, like `Finished`'s
`result` slot) will silently mismatch or falsely match.

## Evidence

```rust
let found_str = terminal_state_to_string(hydration.summary().terminal);
let expected_str = terminal_state_to_string(Some(expected));

if found_str != expected_str {
    return Err(RecoveryError::TerminalStateMismatch {
        expected: expected_str,
        found: found_str,
    });
}
```

```rust
fn terminal_state_to_string(terminal: Option<crate::recovery::RecoveryTerminalState>) -> String {
    match terminal {
        None => "NoTerminal".to_owned(),
        Some(crate::recovery::RecoveryTerminalState::Cancelled) => "Cancelled".to_owned(),
        Some(crate::recovery::RecoveryTerminalState::Killed) => "Killed".to_owned(),
        Some(crate::recovery::RecoveryTerminalState::Failed) => "Failed".to_owned(),
        Some(crate::recovery::RecoveryTerminalState::Finished { .. }) => "Finished".to_owned(),
    }
}
```

Note that `Finished { result }` discards the `result` payload during the
string conversion — two `Finished` states with different `result` slots
compare as equal. The function is supposed to verify "the recovered
terminal state matches the expected", but the comparison ignores the
result slot.

## Adversarial Check

The docstring says the function "verifies terminal state", and today the
only fielded callers compare against outer `Cancelled`/`Killed`/`Failed`
variants, so the `Finished { result }` collapse does not yet cause a
problem. But the function is `pub` and the API contract is "matches
expected" — not "matches expected variant kind". A caller that recovers a
`Finished { result: SlotIdx(7) }` and expects `Finished { result:
SlotIdx(7) }` will pass even if the actual recovered result is
`SlotIdx(99)`, because both stringify to `"Finished"`.

## Suggested Fix

Derive `PartialEq` on `RecoveryTerminalState` (it is already `Copy` and
`Eq`) and compare directly:
```rust
if hydration.summary().terminal != Some(expected) {
    return Err(RecoveryError::TerminalStateMismatch {
        expected: format!("{:?}", Some(expected)),
        found: format!("{:?}", hydration.summary().terminal),
    });
}
```
The `Debug`-based stringification for the error message is fine — the
*comparison* should be on the typed value.
