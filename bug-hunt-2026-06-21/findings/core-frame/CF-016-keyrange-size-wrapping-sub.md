# CF-016: `KeyRange::size` uses `wrapping_sub`, contradicting the file's "checked operations" contract

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/shard/partition/mod.rs:104`
- **Confidence**: confirmed

## Description

The module docstring (lines 7-12) commits: "All arithmetic uses checked
operations; no panics." Yet `KeyRange::size` uses `self.end.wrapping_sub(self.start)`,
which silently wraps on underflow. The correctness of the function
depends on the `start <= end` invariant — but that invariant is itself
violated if a caller ever constructs a `KeyRange` via
`KeyRange { start, end }` (which the pub fields would permit in any
future refactor) or via the const constructors `from_single_key` /
`full_keyspace` (which bypass `new`).

## Evidence

```rust
#[must_use]
pub const fn size(self) -> u64 {
    self.end.wrapping_sub(self.start)
}
```

(`crates/vb_core/src/shard/partition/mod.rs:103-106`)

The `count` method on line 108-110 uses `checked_sub` + `checked_add`
correctly; `size` is the outlier.

## Adversarial Check

A defender might say "the `start <= end` invariant is enforced by `new`,
so `wrapping_sub` is safe." But (a) the const constructors
`from_single_key` and `full_keyspace` skip `new`, even though they
happen to construct valid ranges; (b) `KeyRange`'s fields are private
today but the module is explicitly a "verification model" pending
"promotion to production types," at which point the fields might be
exposed; (c) the file's stated contract is "checked operations" without
qualifier. `wrapping_sub` is not a checked operation. The function also
returns a misleading value for `[5, 5]` (size 0, even though the range
contains one key) — callers who confuse `size` with `count` will
silently get off-by-one results.

## Suggested Fix

Either:
(a) `self.end.checked_sub(self.start).unwrap_or(0)` — though `unwrap_or`
arguably violates the no-unwrap spirit;
(b) `self.end.saturating_sub(self.start)` — semantically a "size floor";
(c) delete `size` and use `count()` everywhere, since `count` is the
correct semantics and already exists.
