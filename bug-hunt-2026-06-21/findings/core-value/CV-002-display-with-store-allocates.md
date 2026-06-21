# CV-002: `SlotValue::display_with_store` allocates a `String` despite the documented allocation-free hot path

- **Severity**: Low
- **Category**: perf
- **Location**: `crates/vb_core/src/value/slot.rs:78`
- **Confidence**: confirmed

## Description

`SlotValue::display_with_store` is documented as deferring all formatting
to `SlotValueDisplay` so "the hot-path value module [stays]
allocation-free" (comment at line 73-77). The implementation, however,
calls `.to_string()` on the `SlotValueDisplay`, which allocates a
`String`. Any caller that invokes this method on a hot path pays a heap
allocation per call, contradicting the doc.

## Evidence

```rust
/// Resolves arena handles against the store and returns a human-readable
/// string.  Falls back to the bare `Display` representation when the
/// handle cannot be resolved (out-of-bounds, missing field, etc.).
///
/// # Performance Note
/// This method allocates only when formatting output. The `SlotValueDisplay`
/// type defers all formatting to its `Display` implementation, keeping the
/// hot-path value module allocation-free.
pub fn display_with_store(&self, store: &ValueStore) -> String {
    super::display::SlotValueDisplay::new(self, store).to_string()
}
```

(`crates/vb_core/src/value/slot.rs:70-80`)

The docstring technically admits the allocation, but its framing ("this
method allocates *only when formatting output*") downplays the fact that
*every call formats output*. The `SlotValueDisplay` newtype exists
precisely so callers can write to a formatter without going through
`String`; routing back through `.to_string()` defeats the purpose.

## Adversarial Check

A defender might say "this is a convenience method for callers that want
a `String`, not a hot-path method." Then it should not be on the
`SlotValue` impl in the `value` module — it should be a free function in
`display` or `diagnostic`, and the doc should not advertise
"allocation-free" anywhere in its vicinity. As written, the comment and
the code contradict each other.

## Suggested Fix

Either delete the method (forcing callers to use `SlotValueDisplay`
directly with `write!(f, "{d}")`), or move it to a `diagnostic` module
where the allocation is expected.
