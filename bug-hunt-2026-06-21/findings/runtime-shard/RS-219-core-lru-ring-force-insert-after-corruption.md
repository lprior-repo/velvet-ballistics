# RS-219-core-lru-ring-force-insert-after-corruption: `force_insert` continues after a fatal sweep invariant error

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/shard/lru_ring.rs:255`
- **Confidence**: confirmed

## Description
The LRU ring documents `LruRingError` as fatal internal corruption, but `force_insert` logs a sweep error and then continues mutating the ring. That can compound an already-corrupt linked-list or position-map state instead of stopping at the typed error boundary.

## Evidence
```rust
21: //! # Error model
22: //!
23: //! Mutating operations that touch the doubly-linked list
24: //! (`remove`, `sweep_expired`, `unlink`) surface internal invariant
25: //! violations through [`LruRingError`] instead of silently skipping
26: //! the failed pointer fix-up. Production code MUST treat every
27: //! `LruRingError` variant as a fatal corruption indicator; the
28: //! invariants the error type guards (live-slot ↔ position-map
29: //! consistency, doubly-linked-list pointer integrity, free-list
30: //! accounting) cannot be repaired from the call site.
...
255:         if let Err(error) = self.sweep_expired(now) {
256:             tracing::error!(
257:                 target: "vb_runtime::lru_ring",
258:                 error = %error,
259:                 "force_insert encountered lru ring invariant violation during sweep"
260:             );
261:         }
262:         let before = self.position.len();
263:         self.push_tail(item, now);
```

`insert` propagates `sweep_expired` errors through `RuntimeError`, but `force_insert` discards the failure and appends a new node anyway.

## Adversarial Check
This is distinct from normal capacity overflow. The code path only triggers after `sweep_expired` has reported an internal invariant violation that the module documentation says production must treat as fatal. Continuing mutation after that point is the opposite of the documented error model.

## Suggested Fix
Change `force_insert` to return `Result<(), RuntimeError>` and propagate `LruRingError::into_runtime_error`. If the public signature must remain temporarily, poison the ring or expose a fatal error state rather than appending after corruption.
