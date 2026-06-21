# CF-004: `RunFrame::new` defaults `max_parallel_in_flight` to `u16::MAX`

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/frame/lifecycle.rs:38` (also 72)
- **Confidence**: confirmed

## Description

`RunFrame::new` initializes `max_parallel_in_flight: u16::MAX` and
`reinitialize` resets it to the same value. Until a caller explicitly
invokes `set_max_parallel_in_flight`, the field claims the run is allowed
65 535 parallel branches. Given that the field is meant to be the
configured ceiling (per CF-001), seeding it to the type maximum means any
read of the field before configuration returns a misleading "everything
is allowed" value.

## Evidence

```rust
Ok(Self {
    ...
    max_parallel_in_flight: u16::MAX,
    ...
})
```

(`crates/vb_core/src/frame/lifecycle.rs:32-43`)

Same on the reinitialize path (`lifecycle.rs:72`).

## Adversarial Check

One might argue the value gets overwritten by `set_max_parallel_in_flight`
before any real branch is spawned, so the default does not matter. But
nothing in the lifecycle enforces that ordering. A run that admits a
TogetherStart branch before the limit is configured would silently use
`u16::MAX` as the ceiling. Combined with CF-001 (no enforcement), the
default is doubly dangerous: it both advertises an unbounded ceiling and
fails to constrain.

## Suggested Fix

Default to `0` (reject everything until explicitly configured), or make
the field `Option<u16>` so an unconfigured ceiling is a hard error.
