# V82 manual_inspect Clippy Fix

## File
`crates/vb_storage/src/batch/artifact_sequence.rs`

## Change
Line 48: replaced `map_err` pattern with idiomatic `inspect_err`.

**Before:**
```rust
.map_err(|error| {
    self.aborted = true;
    error
})?;
```

**After:**
```rust
.inspect_err(|_| {
    self.aborted = true;
})?;
```

## Rationale
`map_err` is for transforming errors. When the intent is only to perform a side effect (marking the batch as aborted) while passing the same error through, `inspect_err` is the correct combinator. It takes a closure that receives the error for inspection but returns it unchanged automatically.

## Behavior
Identical to before: on error from `compute_put_artifact_sequence`, `self.aborted` is set to `true` and the original error is propagated via `?`. No change to error values, semantics, or control flow.