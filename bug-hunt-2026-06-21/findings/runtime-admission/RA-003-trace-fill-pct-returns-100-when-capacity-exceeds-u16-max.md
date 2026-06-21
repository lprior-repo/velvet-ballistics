# RA-003: `trace_fill_pct` reports 100 % fill when trace ring capacity exceeds `u16::MAX`

- **Severity**: Medium
- **Category**: bug (incorrect metric)
- **Location**: `crates/vb_runtime/src/runtime/runtime_metrics.rs:115-127`
- **Confidence**: confirmed

## Description

`trace_fill_pct` clamps both capacity and length through `bounded_u16`, which returns `None` for any value above `u16::MAX` (65 535). On `None` it unconditionally returns `100.0`, so an empty ring with capacity > 65 535 is reported as 100 % full. Production trace rings can be configured up to `MAX_TRACE_RING_CAPACITY = 1_048_576` (`vb_core/src/limits.rs:137`), so this is a live configuration, not a theoretical one.

## Evidence

```rust
fn trace_fill_pct(shard: &Shard) -> f32 {
    let capacity = shard.trace_ring().capacity();
    if capacity == 0 {
        return 0.0;
    }
    let Some(capacity_u16) = bounded_u16(capacity) else {
        return 100.0;
    };
    let Some(len_u16) = bounded_u16(shard.trace_ring().len()) else {
        return 100.0;
    };
    (f32::from(len_u16) / f32::from(capacity_u16)) * 100.0
}

fn bounded_u16(value: usize) -> Option<u16> {
    u16::try_from(value).ok()
}
```

`MAX_TRACE_RING_CAPACITY = 1_048_576` (`vb_core/src/limits.rs:137`) and `TraceRing::new` clamps capacity into `1..=MAX_TRACE_RING_CAPACITY` (`trace/ring.rs:45`), so any operator that configures `trace_capacity` between 65 536 and 1 048 576 receives a permanently misleading 100 % fill metric regardless of actual ring occupancy.

## Adversarial Check

One could argue that since `len ≤ capacity`, the `len_u16` branch is unreachable once `capacity_u16` succeeds. That is true. The bug is the capacity branch: returning `100.0` for "capacity too large to fit in u16" is a non-sequitur — capacity exceeding u16::MAX says nothing about fill ratio. A 0-length ring with capacity 100 000 should report 0 %, not 100 %. The f32 computation itself does not require u16; `usize -> f32` is available via `as` (the runtime already permits `as` casts in `saturating_u32`/`bounded_u16` patterns) or via intermediate u32/u64 casts.

## Suggested Fix

Drop the u16 bounding entirely and compute the percentage in a wider numeric type, e.g.:

```rust
fn trace_fill_pct(shard: &Shard) -> f32 {
    let capacity = shard.trace_ring().capacity();
    if capacity == 0 {
        return 0.0;
    }
    let len = shard.trace_ring().len();
    (len as f64 / capacity as f64 * 100.0) as f32
}
```

Or expose `TraceRing::fill_pct()` directly so the metric is computed at the source.
