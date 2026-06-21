# CV-102: Idempotency-key validation ignores unreadable key slots

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_core/src/action/validate.rs:31`
- **Confidence**: confirmed

## Description

`validate_idempotency_key_ingredients` treats out-of-bounds or uninitialized key slots as a successful validation path. An action requiring an idempotency key can pass with a non-empty key slot list that contains no readable deterministic ingredient.

## Evidence

Unreadable slots are skipped:

```rust
let Ok(slot_taint) = frame.read_taint(slot) else {
    i = match i.checked_add(1) {
        Some(next) => next,
        None => break,
    };
    continue;
};
```

`verify_idempotency` only checks that `key_slots` is non-empty before delegating:

```rust
RetrySafety::RequiresIdempotencyKey => {
    if key_slots.is_empty() {
        return Err(IdempotencyViolation::MissingKey(action.side_effect));
    }
    validate_idempotency_key_ingredients(key_slots, frame)
}
```

## Adversarial Check

The skip is not a defensive fallback. A key ingredient that cannot be read cannot prove determinism, secrecy, or reproducibility. Because `verify_idempotency` accepts any non-empty `key_slots`, a single invalid slot is enough to bypass the MissingKey gate.

## Suggested Fix

Add an `IdempotencyViolation` variant for invalid/unreadable key slots, or reuse `MissingKey` for unreadable ingredients. Return an error immediately when `frame.read_taint(slot)` fails.
