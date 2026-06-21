# RP-009: `enqueue` computes `_warning_was_dropped` and never uses it

- **Severity**: Low
- **Category**: simplification
- **Location**: `crates/vb_runtime/src/action_queue/queue.rs:85`
- **Confidence**: confirmed

## Description

`enqueue` records whether the backpressure warning was dropped by computing `tx.try_send(warning).is_err()` into `_warning_was_dropped`. The leading `_` silences the unused-variable lint, but the boolean is never read. The doc above the call claims the result "is still observed explicitly so fallible status is not silently discarded", which is misleading — the value is computed and discarded.

## Evidence

`crates/vb_runtime/src/action_queue/queue.rs:76-87`:

```rust
let warning = BackpressureWarning {
    depth,
    capacity: self.capacity.get(),
};
// The warning channel is best-effort by contract: enqueue must not
// stall when the backpressure receiver falls behind. The send
// result is still observed explicitly so fallible status is not
// silently discarded.
let _warning_was_dropped = tx.try_send(warning).is_err();
```

`_warning_was_dropped` is not read anywhere else in the function or in the surrounding module. `rtk grep` confirms: the identifier appears only at this assignment site.

## Adversarial Check

This is purely a clarity/maintainability nit. There is no correctness or perf impact. The comment claiming the result "is still observed explicitly" is the real problem — it implies monitoring that does not exist. A future maintainer reading the comment will assume drops are surfaced somewhere (a metric, a log, a counter) when in fact they are not.

## Suggested Fix

Either drop the binding entirely:

```rust
let _ = tx.try_send(warning);
```

or actually record the drop into a counter:

```rust
if tx.try_send(warning).is_err() {
    self.dropped_warnings = self.dropped_warnings.saturating_add(1);
}
```

with a corresponding `pub fn dropped_warnings(&self) -> u64` accessor on `BoundedActionCompletionQueue`. Either change matches the implementation to the comment.
