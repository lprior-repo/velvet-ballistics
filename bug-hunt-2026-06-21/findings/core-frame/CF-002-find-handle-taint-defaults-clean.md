# CF-002: `find_handle_taint` defaults to `Taint::Clean` when handle is not found

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_core/src/frame/slots.rs:136` (also 152)
- **Confidence**: confirmed

## Description

When `find_handle_taint` cannot find a matching `SlotValue::Object(id)` or
`SlotValue::List(id)` in any slot, it returns `Ok(Taint::Clean)`. A handle
that the frame cannot account for is treated as completely untainted —
the least restrictive element of the taint lattice. This inverts the
fail-safe direction expected of a taint lattice: an unknown handle should
be treated as `Secret` (most restrictive) or surfaced as an
invariant-violation error, never as `Clean`.

## Evidence

```rust
SlotValue::Object(id) => {
    let mut idx = 0usize;
    while idx < usize::from(self.slot_count) {
        if let Some(Some(SlotValue::Object(vid))) = self.slots.get(idx)
            && vid == id
        {
            return self.taint.get(idx).copied().ok_or(...);
        }
        idx = idx.saturating_add(1);
    }
    Ok(Taint::Clean)            // <-- unknown handle defaults to Clean
}
SlotValue::List(id) => {
    ...
    Ok(Taint::Clean)            // <-- unknown handle defaults to Clean
}
_ => Ok(Taint::Clean),
```

(`crates/vb_core/src/frame/slots.rs:120-156`)

## Adversarial Check

A defender might argue "the only way to obtain a `SlotValue::Object(id)`
is by inserting it into this frame's ValueStore, so the lookup can never
fail." But cross-frame value flow (e.g. parent → child run, journal
replay, snapshot hydration) can absolutely produce a handle that was
allocated in a different store. Even within a single frame, a buggy
caller could pass an `id` from a stale snapshot. Defaulting to `Clean`
means a derived-from-secret computation that touches an unknown handle
will be marked Clean, leaking the secret across the taint boundary.

The task brief explicitly lists "taint lattice join picking wrong
direction" as a Critical-class finding; defaulting to the *least*
restrictive element is the canonical wrong direction.

## Suggested Fix

Return `Ok(Taint::Secret)` for unknown handles (fail-closed), or —
better — return `Err(CoreError::InternalInvariantViolation { reason:
"handle_taint_unknown" })` so the caller is forced to handle the
mismatch explicitly.
