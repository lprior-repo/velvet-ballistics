# SR-013: `legacy_slot_taint` asymmetric Bool heuristic leaks Bool(false) secret slots as Clean

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_storage/src/recovery/replay/summary/slots/taint.rs:60`
- **Confidence**: confirmed

## Description

`legacy_slot_taint` maps `SlotValue::Bool(false)` to `Taint::Clean` while
mapping `SlotValue::Bool(true)` to `Taint::DerivedFromSecret`. The asymmetry
means that any secret-derived boolean predicate that happens to evaluate
`false` is recovered as `Clean`, allowing downstream consumers to expose it
in logs, CLI output, or redaction-stripped debug dumps.

## Evidence

```rust
fn legacy_slot_taint(value: SlotValue) -> Taint {
    match value {
        SlotValue::Bool(false) => Taint::Clean,
        SlotValue::Bool(true) | SlotValue::Null => Taint::DerivedFromSecret,
        _ => Taint::Secret,
    }
}
```

This is invoked when recovering legacy `SlotWrittenEvent` records that lack
the modern `extra` taint envelope. The heuristic assumes:

1. Booleans are always predicate outputs from comparison/logical ops.
2. A `false` result implies no secret dependence.
3. A `true` result implies a secret was involved.

(1) is sometimes true. (2) is unsafe: the negation of a secret-derived
`true` is `false`, and `slot == SECRET_VALUE` evaluating to `false` leaks
the secret by exhaustion (one-bit leak per slot). (3) is unsafe for the
inverse reason.

The fail-safe default for an unknown taint should be `Taint::Secret`
(over-classification is safe; under-classification is a leak). This is
exactly what the accumulator's `recovered_event_slots` does in
`slots/recovery.rs:107` when taint is missing:
```rust
taint: accumulator
    .slot_taint
    .get(slot)
    .copied()
    .map_or(Taint::Secret, |taint| taint),
```

So the crate already knows the safe default; the legacy helper contradicts
it.

## Adversarial Check

One might argue "this only runs on legacy snapshots that predate the taint
envelope, and we accept best-effort recovery for those." But the function is
on the recovery code path today — `recovered_slot_taint` (line 18) calls it
whenever `extra` is `None`, and `SlotWrittenEvent.extra` is `Option<Vec<u8>>`
so any event without an explicit envelope triggers it. An adversary who can
inject a `SlotWrittenEvent { value: Some(encode(Bool(false))), extra: None }`
into a recovered run gets the result classified as `Clean`. The asymmetric
heuristic is also surprising to readers — there is no comment justifying
why `false` is "cleaner" than `true`.

## Suggested Fix

The legacy fallback should fail closed, matching the rest of the crate:
```rust
fn legacy_slot_taint(_value: SlotValue) -> Taint {
    Taint::Secret
}
```
If the legacy tier *really* needs to distinguish "known-clean" slots (e.g.
frame counters), encode that as an explicit `SlotWrittenEvent.extra` going
forward and leave the legacy fallback uniformly Secret.
