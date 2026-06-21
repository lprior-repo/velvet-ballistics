# CV-106: Span::new accepts inverted byte ranges

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_core/src/span.rs:20`
- **Confidence**: confirmed

## Description

`Span` is documented as an inclusive-start, exclusive-end byte range, but `Span::new` allows `start > end`. That creates invalid source locations that are not empty and cannot represent a real slice.

## Evidence

The constructor stores unchecked offsets:

```rust
pub const fn new(start: u32, end: u32) -> Self {
    Self { start, end }
}
```

The only predicate checks equality, not ordering:

```rust
pub const fn is_empty(self) -> bool {
    self.start == self.end
}
```

## Adversarial Check

This is not a request for richer source maps. A byte span with `start > end` violates the type's own range contract and can poison diagnostics, sorting, overlap checks, or downstream source slicing. Since the fields are public, a smart constructor alone cannot enforce the invariant globally, but the current constructor gives callers an official invalid construction path.

## Suggested Fix

Introduce a fallible constructor such as `Span::try_new(start, end) -> Result<Span, SpanError>` that rejects `start > end`. Keep `new` only if it clamps, swaps, or is explicitly documented as unchecked.
